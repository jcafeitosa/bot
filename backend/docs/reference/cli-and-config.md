---
title: Referência de CLI e configuração
description: Comandos, opções, variáveis de ambiente e contratos de configuração do backend
tags:
  - reference
  - backend
  - cli
  - configuration
---
# Referência de CLI e configuração

## Comandos

O binário tem dois caminhos principais:

| Caminho | Uso |
|---|---|
| Monitor | Executado sem subcomando; inicia a TUI e o fluxo de mercado configurado. |
| Backtest | `cargo run -- backtest --config <arquivo>`; gera candles sintéticos e imprime um resumo JSON. |
| HTTP API | `cargo run -- --config src/core/config/bot.toml serve --bind 127.0.0.1:8080`; OpenAPI em `/openapi.json`, UI Scalar em `/docs`. |
| Graph query (supervision chain) | `cargo run -- graph query supervision-chain --agency-id <id> --agent-id <id>` — read-only; mesmo pré-requisito Neo4j que agents list; [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md). |
| Graph query (read-only) | `cargo run -- graph query agents --limit 32`; `graph query supervision-chain` / `bots-for-agent` — requer `BOT_AGENTS_ENABLED` + Neo4j; ver [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md). |
| Graph projection drain | `cargo run -- graph-projection drain --limit 32` — requer `DATABASE_URL` + `BOT_AGENTS_ENABLED` + Neo4j; ver [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md) F2.1.3. |

Exemplo do monitor:

```sh
cargo run -- --config src/core/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative
```

Sem `--config`, o processo procura `src/core/config/bot.toml` relativo ao diretório de execução (execute a partir de `backend/` ou passe caminho absoluto).

## Configuração em camadas

| Artefato | Função |
|---|---|
| `backend/.env` (gitignored) | Secrets e overrides; **vence** TOML quando definido. Modelo: `backend/.env.example`. |
| `src/core/config/system.toml` | Defaults não sensíveis (orders, bots, monitor, providers endpoints, neo4j estrutura). Carregado via `SystemConfig` no boot. |
| `src/core/config/bot.toml` | Preset monitor (market, strategy, risk, `run_mode`). |
| `src/core/config/exchanges/*.toml` | Contas/endpoints exchange (`binance.toml`); `load_registry` ignora `credentials.toml` (só documenta env `BINANCE_TESTNET_*`). |
| PostgreSQL `provider_credentials` | API keys LLM (primary); env `TYPESAFE_*` / `OPENAI_*` = bootstrap deprecated. |

Ordem de bootstrap: `ensure_dotenv_loaded()` → `system.toml` → `bot.toml`. Detalhes: [SDD configuração centralizada](../sdd/centralized-config-sdd.md), [provider credentials](../sdd/provider-credentials-db-sdd.md).

## Subcomando `serve` (HTTP)

Flags globais (`--config`, `--environment`, …) vêm **antes** de `serve`. Também é possível repetir o TOML no subcomando:

```sh
cargo run -- --config src/core/config/bot.toml serve --bind 127.0.0.1:8080
cargo run -- serve --config src/core/config/bot.toml --bind 127.0.0.1:8080
```

| Opção | Descrição |
|---|---|
| `--bind` | Endereço de escuta (padrão `127.0.0.1:8080`). |
| `--config` | Override opcional do caminho `bot.toml` só para o processo da API. |
| `--with-monitor` | Sobe o monitor headless no mesmo processo; `/api/v1/monitor/*` deixa de retornar 503. |
| `GET /api/v1/meta` | Metadados + `http_seams` read-only; `order_execution_mode`/`live_exchange_wired` alinham com `GET /orders/execution-status`; `bot_runtime_enabled` alinha com `GET /bots/runtime/status` (`runtime_enabled`). |

Sem `--with-monitor`, rotas `/api/v1/monitor/*` respondem **503**. Com `--with-monitor`, o loop de mercado roda headless (sem Ratatui) e snapshot/comandos HTTP funcionam. Registro de agents, risco, estratégia, backtest e config snapshot funcionam sem monitor anexo. Advisory Jev exige `jev.enabled` e credenciais no ambiente.

Rotas principais dos módulos alvo do goal (prefixo `/api/v1`):

| Grupo | Rotas | Notas |  |
|---|---|---| - |
| `agents` | `GET/POST /agents`, lifecycle, `POST …/advisory` | Sem autenticação do owner (Gate 1 pendente). |  |
| `bots` | `GET /bots/catalog`, persist/snapshot, ranking; `GET /bots/runtime/status`, `POST /bots/runtime/promote|demote` (mutações exigem admin quando token ativo) | Runtime default fail-closed; `BOT_RUNTIME_ENABLED=true` + `shared_bot_runtime`. Promote: `assert_bot_promotion_allowed` (catálogo + mercado do config). Com `BOT_HTTP_AGENCY_ID`, agente com `promote_runtime_bot`. Supervisor: `MonitorStrategyRegistry` + `strategy_evaluation_binding` + `BotSignal.bot_id`. Catálogo HTTP inclui `monitor_fast_period` / `monitor_slow_period` / `monitor_evaluator` (`sma_cross` ou `ema_cross` via `[[strategy.monitor_registry]]`). |
| `orders` | `GET /orders/execution-status` (somente leitura), `POST /orders/submit` | Status: `mode` + `live_exchange_wired` (`recording` ou `testnet`+credenciais). Submit: fail-closed **503**; `paper`/`dev_accept` **200** após risco; `client_order_id` dedupe; corpo opcional `paper_fill_unit_price` (modo paper → portfolio `positions`). |  |
| `portfolio` | `GET /portfolio/paper-snapshot?quote=…` | Saldo paper + `positions[]` quando fills têm preço (`paper_fill_unit_price` no submit ou `BOT_PAPER_FILL_UNIT_PRICE`); baseline 1000 na quote. |  |

Detalhes: [auditoria de completude](../planning/modules-completeness-audit.md).


## Opções do monitor

| Opção | Valores |
|---|---|
| `--environment` | `dev`, `prod` |
| `--operation` | `hft`, `scalper`, `day-trader`, `swing-trader` |
| `--risk-profile` | `conservative`, `moderate`, `aggressive`, `auto` |
| `--mode` | `observe`, `paper`, `testnet` |
| `--config` | caminho para TOML, padrão `src/core/config/bot.toml` |

O modo `hft` é explicitamente rejeitado pelo backend atual. `testnet` não habilita ordens na versão atual.

## Timeframes por operação

| Operação | Timeframes aceitos |
|---|---|
| `scalper` | `1m`, `3m`, `5m` |
| `day-trader` | `5m`, `15m`, `30m` |
| `swing-trader` | `1h`, `4h` |
| `hft` | `1m`, mas a operação é rejeitada |

A validação de período SMA e timeframe é feita junto com a configuração. Combinações inválidas impedem a inicialização.

### Registry de estratégias do monitor (opcional)

Além de `sma-cross@1` derivado de `[strategy].sma_fast` / `sma_slow`, o TOML pode declarar entradas extras para catálogo HTTP e `MonitorStrategyRegistry`:

```toml
[[strategy.monitor_registry]]
id = "sma-cross"
version = 2
name = "SMA crossover alt"
fast_period = 3
slow_period = 15
```

Cada linha deve ter `0 < fast_period < slow_period` e `version > 0`. O supervisor ainda só avalia estratégias registradas com evaluator SMA; ids desconhecidos na promoção caem em fallback com log.

`GET /api/v1/config/active` e `GET /api/v1/config/snapshot` incluem o array `monitor_registry` (somente entradas extras do TOML; cada item pode definir `evaluator` = `sma_cross` ou `ema_cross`; `sma-cross@1` continua refletido em `sma_fast` / `sma_slow`).

## Variáveis de ambiente

| Variável | Finalidade |
|---|---|
| `BINANCE_TESTNET_API_KEY` / `BINANCE_TESTNET_SECRET` | Credenciais opcionais da conta Spot de teste; devem ser fornecidas em conjunto. |
| `DATABASE_URL` | Conexão + migração PostgreSQL; no `serve`, hidrata registry de agents (se vazio) e faz write-through best-effort do catálogo de bots; idempotência em `order_idempotency_keys`; reconciliação pós-submit live em `order_reconciliation` (write-through após `POST /orders/submit` com `client_order_id`). |
| `PERSIST_MARKET_DATA=1` | Ativa a persistência opcional do monitor. |
| `TYPESAFE_API_KEY` | Credencial para avaliações consultivas do Jev/TypeSafe quando habilitadas. |
| `TYPESAFE_ENDPOINT` | Endpoint compatível alternativo (URL completa do advisory); HTTP só é aceito para localhost. |
| `OPENAI_API_KEY` | Bearer alternativo quando `TYPESAFE_API_KEY` não está definida (proxies OpenAI-compatible). |
| `OPENAI_BASE_URL` | Raiz OpenAI-compatible para `core::providers::openai_compatible` (ex.: proxy 9router). |
| `NINE_ROUTER_BASE_URL` | Alias documentado para a mesma raiz; precede `OPENAI_BASE_URL`. |
| `NVIDIA_API_KEY` / `NGC_API_KEY` | Bearer para NVIDIA NIM (`core::providers::nvidia_nim`); `NGC_API_KEY` é fallback. |
| `BOT_AGENCY` | Quando definida, `serve --with-monitor` usa `RegistryMonitorAgentHook` para a agência (registry compartilhado com HTTP agents). |
| `BOT_HTTP_OWNER_ID` | Com `BOT_HTTP_ADMIN_TOKEN`, restringe `owner_id` no registro de agentes ao valor configurado. |
| `BOT_PRODUCT_OWNER_BOOTSTRAP_ID` | Com `DATABASE_URL` e `BOT_PRODUCT_OWNER_BOOTSTRAP_ACK`, grava owner singleton em PG (`0010_product_owner_bootstrap`); boot HTTP carrega `VerifiedProductOwner` e `GET /meta` → `product_owner_bootstrap_active`. |
| `BOT_PRODUCT_OWNER_BOOTSTRAP_ACK` | Deve ser `1`/`true`/`yes` junto com `BOT_PRODUCT_OWNER_BOOTSTRAP_ID` para mutar PG (fail-closed sem ACK). |
| `BOT_HTTP_AGENCY_ID` | Restringe rotas `/api/v1/agents*` ao `agency` configurado (query ou body); falha **403** `http_agency_mismatch`. `GET /meta` → `http_agency_binding_active` (booleano, sem expor o ID). |
| `BOT_HTTP_ADMIN_TOKEN` | Quando não vazio, rotas HTTP mutantes exigem `Authorization: Bearer <token>` (fail-closed; não substitui auth do owner). |
| `BOT_RUNTIME_ENABLED` | `true` ativa `InMemoryBotRuntime` (promoção/demote em processo); default/false fail-closed (**503** em promote). |
| `BOT_ORDERS_EXCHANGE_SUBMIT` | Com `BOT_ORDERS_EXECUTION=live_exchange`, `recording` liga executor sem rede; `testnet` + `BINANCE_TESTNET_*` liga `ExchangeSpotExecutor` + submit ccxt (market **buy** por `quote_amount`; rede real). Sem credenciais → `live_exchange_reserved`. |
| `BOT_ORDERS_EXECUTION` | vazio/`disabled` (fail-closed); `dev_accept` (double local); `paper` (`PaperLedgerExecutor`, ledger in-process); `live_exchange` + `BOT_ORDERS_EXCHANGE_SUBMIT=recording` → **200** após risco; `live_exchange` sem submit backend — **503** `live_exchange_not_wired`). Outros valores → `disabled`. |
| `DATABASE_URL` + migração `0006` | Com HTTP `serve`, `ApiState` registra `register_live_reconciliation_pg_mirror`; submits testnet do monitor espelham `order_reconciliation` via `try_mirror_reconciliation_upsert`. |
| `BOT_ORDERS_RECONCILIATION_POLL_SECS` | Opcional com `live_exchange` wired: intervalo em segundos para `ApiState::run_order_reconciliation_poll_once` em background no `serve` (LiveExchange query; default desligado se vazio ou `0`). |
| `BOT_PAPER_FILL_UNIT_PRICE` | Opcional com `paper`: preço quote/base usado no ledger para calcular `positions` no snapshot HTTP (ex.: `50000` para BTC/USDT). |
| `client_order_id` (body HTTP) | Campo opcional em `POST /api/v1/orders/submit`; replays retornam `accepted: true` sem reexecutar (memória; PG quando `DATABASE_URL` + migração `0004`). Com `live_exchange_wired`, reconciliação `pending`→`reconciled` (memória + PG `0006`); consulta `GET /api/v1/orders/reconciliation/{client_order_id}`; testnet usa `newClientOrderId` no submit; poller testnet pode `fetch_order` sem binding local. |
| `paper_fill_unit_price` (body HTTP) | Opcional em modo `paper`: preço quote/base por ordem para `positions` no snapshot (alternativa a `BOT_PAPER_FILL_UNIT_PRICE`). |
| `BOT_AGENTS_ENABLED` / `BOT_NEO4J_*` | Grafo Neo4j opcional para agentes; projeção write-only F1–F3.1 + outbox F2.1; ver `docs/operations/postgres-and-graph-dev.md`. |
| `neo4j.graph_projection_outbox_drain_secs` (`system.toml`) | Intervalo do worker de drain do outbox PG→Neo4j quando PG+Neo4j wired no `serve`/monitor; default **30**; **0** desliga o ticker. |
| `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS` | Override env do intervalo (mesma semântica que `system.toml`; vence TOML quando definido). |
| `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_BATCH` | Tamanho do lote por tick de drain (default **32**, clamp 1–500). |
| `GET /healthz` | Campo opcional `graph_projection_outbox` (pending/retry/idade) e `status: degraded` com backlog; ver [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md). |
| `NVIDIA_NIM_BASE_URL` | Raiz da integrate API (default `https://integrate.api.nvidia.com`); opcional em TOML como `providers.nim_base_url`. |

Variáveis comentadas e exemplos mínimos: `backend/.env.example` (inclui seams `BOT_ORDERS_*`, `BOT_RUNTIME_ENABLED`, `BOT_HTTP_*`).

Nunca comite `.env` ou credenciais.

## Persistência

- O backtest usa `--persist` para gravar o dataset sintético.
- O monitor só persiste com `PERSIST_MARKET_DATA=1`, `DATABASE_URL` e timeframe `1m`.
- As migrações ativas ficam em `src/core/database/migrations/` (via `PostgresDatabase::migrate` / `Database::migrate`).
- Falha de conexão não deve ser tratada como prova de histórico completo.

A semântica de estados, gaps e recuperação está no [SDD T-15](../sdd/monitor-persistence-policy-sdd.md). O fluxo de candles e avaliação está no [SDD T-10](../sdd/monitor-pause-resume-sdd.md).

### PG orders retention (Gate 2)

Política operacional sugerida (não há purge automático no binário `bot`):

| Tabela | Retenção sugerida | Ação se violada |
|--------|-------------------|-----------------|
| `order_idempotency_keys` | **90 dias** após `completed_at` | Job SQL/manual de delete; dedupe em memória reinicia com o processo |
| `order_reconciliation` (terminal `reconciled` / `divergent`) | **180 dias** | Arquivar ou apagar linhas antigas após backup |
| `order_reconciliation` (`pending`) | alerta se **> 7 dias** | `POST /api/v1/orders/reconciliation/poll` + investigar exchange; manter `BOT_ORDERS_RECONCILIATION_POLL_SECS` ≥ **60** com submit live wired |

Detalhes e threat model: [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md#threat-model-rascunho).

## Verificação local (gates)

```sh
cd backend
./scripts/verify-backend-gates.sh          # → 464 passed, 0 ignored (bin bot); assert-completeness-evidence.sh
./scripts/verify-backend-full.sh         # gates + PG 21/21 quando DATABASE_URL → trading_bot
bot graph query agents --limit 32   # read-only Neo4j (fail-closed sem stack)
bot graph query supervision-chain --agency-id agency-a --agent-id worker-1
bot graph query bots-for-agent --agency-id agency-a --agent-id agent-promoter --limit 32
```

Matriz e manifesto PG: [test-matrix](../reference/test-matrix.md). Auditoria do goal: [modules-completeness-audit](../planning/modules-completeness-audit.md).

## Contratos de segurança

O backend trabalha com dados públicos e não envia ordens no fluxo atual. Redirects entre origens são tratados pelo [SDD T-05](../sdd/rest-redirect-sdd.md). Alterações de configuração, mercado e validação devem seguir o [SDD T-03](../sdd/backend-corrections-sdd.md).

