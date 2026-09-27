---
title: Integrações do backend
description: Binance, ccxt, PostgreSQL, Jev, terminal e controles de integração
tags:
  - architecture
  - backend
  - integrations
  - security
---
# Integrações do backend

## Mapa de integrações

| Integração | Adapter/seam | Uso atual | Estado |
|---|---|---|---|
| Binance Spot Test Network REST | `modules::exchanges::binance` + `MarketDataSource` | Backfill público de candles OHLCV. | Ativo em `dev`; origem restrita. |
| Binance Spot Test Network WebSocket | `modules::exchanges::live` + `tokio-tungstenite` | Klines fechados `1m` no monitor. | Ativo apenas em `1m`; REST permanece fallback. |
| `ccxt-core` / `ccxt-exchanges` | Patch local em `vendor/ccxt-core-0.1.5` | Cliente HTTP e adapter da exchange. | Vendorizado; política de redirect sob gate de segurança. |
| PostgreSQL | `core::database` + `core::persistence::Database` | Migrações (PG 18+, Timescale/pgvector), datasets/candles, espelho agents/bots Gate 1 (`0005` `promote_runtime_bot`), idempotência/reconciliação orders (`0004`/`0006`); boot HTTP hidrata agents + reconciliação orders em memória. | Opt-in; `DATABASE_URL` → `trading_bot`; PG integração **14/14** (`run-pg-integration-tests.sh`). |
| TypeSafe/Jev | `JevAdvisor` | Avaliação consultiva de regime, qualidade do sinal e anomalia. | Opcional; não autoriza ordens. |
| Orders ↔ exchanges (Gate 2) | `submit_spot_order` + `observe_testnet_spot_order_by_client_id`; `shared_live_order_reconciliation_ledger` (HTTP + monitor); PG `0004`/`0006`; `run_reconciliation_poll_once`; `map_bot_error` redige `BINANCE_TESTNET_*` | `recording`/`testnet`; GET/POST reconciliation; poll job `BOT_ORDERS_RECONCILIATION_POLL_SECS`. | Prod REST bloqueado; threat model ops doc + redação erros; LGTM **Critic** pendente. |
| Orders ↔ portfolio (paper) | HTTP `POST /orders/submit` ou monitor `RunMode::Paper` → `PaperLedgerExecutor` (preço = `close` do candle ou body/env) → `http_bridge::portfolio` | `GET /portfolio/paper-snapshot`: saldo quote + `positions[]`. | Ativo; baseline 1000 quote. |
| HTTP API (Axum) | `modules::http_bridge` + `presentation::http` | OpenAPI/Scalar; `GET /api/v1/meta` expõe `http_seams` read-only (modo de orders, admin bearer, bindings owner/agency **sem IDs**, runtime bots, `live_exchange_wired`); agents/bots PG opcional; bearer admin e binds owner/agency; orders/monitor commands mutantes protegidos; orders via `HttpOrderExecutor` (`GET /orders/execution-status`, `BOT_ORDERS_EXECUTION`, `ReservedLiveExchangeExecutor`) + dedupe `client_order_id` (memória + PG `order_idempotency_keys`); bots runtime promote/demote (`BOT_RUNTIME_ENABLED`, `shared_bot_runtime()` no `serve`); catálogo/promoção multi-versão via `[[strategy.monitor_registry]]` + `build_catalog_from_monitor_registry` (`GET /config/active` inclui `monitor_registry`; testes HTTP `sma-cross@2`); supervisor usa `strategy_evaluation_binding` + `evaluate_for_kind` (`sma_cross`/`ema_cross`) + `BotSignal.bot_id`; backtest `run_sma_crossover` usa o mesmo `evaluator`; promote HTTP usa `assert_bot_promotion_allowed`. Ver [SDD HTTP admin](../sdd/http-admin-auth-seam-sdd.md); matriz de testes 401/2xx em [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token). `./scripts/verify-backend-gates.sh` (**395** testes bin `bot`, `--test-threads=1`); PG **14/14** via `verify-backend-full.sh`. Mapa de facades: [module-catalog §3d](./module-catalog.md#3d-facade-http_bridge-srcmoduleshttp_bridge). | Ativo em `serve`. |
| Terminal | Ratatui + Crossterm | Dashboard, comandos de pausa/retomada/saída e logs. | Caminho operacional principal. |

## Binance REST

O adapter recebe `symbol`, `timeframe` e `limit` por `MarketDataSource::candles`. Antes da chamada:

1. `authorize_rest_use` permite somente backfill histórico público da conta Spot em `dev`.
2. A conta valida a origem REST exata da configuração.
3. A janela retornada é filtrada para candles fechados.
4. Cada candle é validado quanto a timestamp, finitude, preços, volume, limites high/low e duplicidade.
5. Uma janela inválida inteira é rejeitada antes de chegar ao feed ou à persistência.

O [SDD T-03](../sdd/backend-corrections-sdd.md) registra as correções de mercado e validação. O [SDD T-05](../sdd/rest-redirect-sdd.md) registra a política para redirects.

## Binance WebSocket

O WS é criado apenas para timeframe `1m`. O parser aceita somente eventos de kline fechados, do símbolo e intervalo esperados, com timestamp alinhado e OHLCV válido. Eventos inválidos são ignorados e registrados sem payload sensível.

O WS alimenta o mesmo `HybridCandleFeed` usado pelo REST. Se a sessão falhar ou fechar, o monitor continua com REST e UI. A pausa invalida trabalhos em voo por geração monotônica; a retomada exige backfill válido antes da avaliação. O contrato completo está no [SDD T-10](../sdd/monitor-pause-resume-sdd.md).

## ccxt vendorizado

O projeto fixa `ccxt-core = 0.1.5` por `[patch.crates-io]` e mantém a cópia em `vendor/ccxt-core-0.1.5`. A alteração de segurança fica no cliente HTTP, antes de a requisição seguir um redirect.

A política pretendida:

- compara a origem inicial e o destino por scheme, host e porta efetiva;
- rejeita mudança de origem, downgrade, userinfo, cadeia vazia e mais de dez redirects;
- permite redirect dentro da mesma origem;
- transforma rejeição em erro observável no poll.

O código não deve usar a validação posterior de `response.url()` como substituto da política no cliente. A prova de transporte local foi executada fora do sandbox; os testes bloquearam redirect cross-origin antes do contato e aceitaram redirect na mesma origem.

## PostgreSQL

A persistência:

- exige `DATABASE_URL`;
- rejeita qualquer banco diferente de `trading_bot`;
- abre pool com limite de oito conexões e timeout de aquisição;
- aplica `src/core/database/migrations/` (market data, scaffold agents/bots, `order_idempotency_keys` para dedupe HTTP de orders);
- grava o manifesto em `market_datasets` e candles em `candles_1m`;
- usa `ON CONFLICT DO NOTHING` para reexecução idempotente;
- executa a gravação em transação.

O teste de round-trip é ignorado por padrão e requer um banco descartável ou dedicado de integração. Persistência indisponível não deve ser interpretada como histórico completo.

## TypeSafe/Jev

A integração é ativada por `jev.enabled = true` e `TYPESAFE_API_KEY`. O endpoint padrão é HTTPS; HTTP é permitido apenas para localhost. O timeout é limitado entre um e trinta segundos.

O payload contém um snapshot reduzido de mercado e indicadores. Não inclui credenciais, saldos, identificadores de conta ou dados privados de ordem. A resposta é convertida em observações textuais e volta para a camada do monitor. Jev é advisory-only: não escolhe conta, não chama exchange e não autoriza execução.

## Terminal e arquivos locais

A CLI usa Clap. A TUI usa Ratatui/Crossterm e o runtime usa Tokio. Configuração é lida de TOML; segredos vêm do ambiente. Logging usa `tracing`, stderr e arquivos JSON rotacionados.

Os contratos operacionais estão na [referência de CLI e configuração](../reference/cli-and-config.md) e no [runbook operacional](../operations/runbook.md).

## Matriz de risco de integração

| Risco | Controle atual | Pendência |
|---|---|---|
| Origem REST incorreta | Conta valida origem e `authorize_rest_use` restringe o uso. | Cobertura contínua ao atualizar adapters. |
| Redirect para host externo | Política no cliente HTTP vendorizado. | Prova HTTP observável passou fora do sandbox; revalidar quando o vendor mudar. |
| Mensagem WS inválida | Parser e validação antes do feed. | Monitorar métricas/rejeições estruturadas. |
| Banco errado | Nome `trading_bot` obrigatório. | Provisionamento operacional documentado fora do código. |
| Vazamento de segredo | Variáveis de ambiente e payload Jev reduzido. | Rotação e runbook de incidente de credencial. |
| API mutante sem auth | `BOT_HTTP_ADMIN_TOKEN` opcional (bearer); `BOT_HTTP_OWNER_ID` / `BOT_HTTP_AGENCY_ID` restringem registro e rotas de agentes. | Auth verificável do owner e bootstrap (Gate 1). |
| Perda de WS | REST mantém fallback. | Alertas operacionais para degradação prolongada. |
| Jev indisponível | Erro consultivo não vira ordem. | Definir política de observabilidade e retry por ambiente. |
