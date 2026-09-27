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

Exemplo do monitor:

```sh
cargo run -- --config src/core/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative
```

Sem `--config`, o processo procura `src/core/config/bot.toml` relativo ao diretório de execução (execute a partir de `backend/` ou passe caminho absoluto).

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

| Grupo | Rotas | Notas |
|---|---|---|
| `agents` | `GET/POST /agents`, lifecycle, `POST …/advisory` | Sem autenticação do owner (Gate 1 pendente). |
| `bots` | `GET /bots/catalog`, persist/snapshot, ranking; `GET /bots/runtime/status`, `POST /bots/runtime/promote|demote` (mutações exigem admin quando token ativo) | Runtime default fail-closed; `BOT_RUNTIME_ENABLED=true` + `shared_bot_runtime`. Promote: `assert_bot_promotion_allowed` (catálogo + mercado do config). Com `BOT_HTTP_AGENCY_ID`, agente com `promote_runtime_bot`. Supervisor: `MonitorStrategyRegistry` + `strategy_evaluation_binding` + `BotSignal.bot_id`. Catálogo HTTP inclui `monitor_fast_period` / `monitor_slow_period` / `monitor_evaluator` (`sma_cross` ou `ema_cross` via `[[strategy.monitor_registry]]`). |
| `orders` | `GET /orders/execution-status` (somente leitura), `POST /orders/submit` | Status expõe `mode` (`disabled` / `dev_accept` / `paper` / `live_exchange` / `live_exchange_reserved`) e `live_exchange_wired` (sempre `false` até adapter). Submit: fail-closed **503**; `live_exchange_not_wired` só em `live_exchange`; `paper` e `dev_accept` **200** após risco; **422** se risco rejeita; `client_order_id` opcional com dedupe memória/PG. |

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
| `DATABASE_URL` | Conexão + migração PostgreSQL; no `serve`, hidrata registry de agents (se vazio) e faz write-through best-effort do catálogo de bots; idempotência de orders em `order_idempotency_keys`. |
| `PERSIST_MARKET_DATA=1` | Ativa a persistência opcional do monitor. |
| `TYPESAFE_API_KEY` | Credencial para avaliações consultivas do Jev/TypeSafe quando habilitadas. |
| `TYPESAFE_ENDPOINT` | Endpoint compatível alternativo (URL completa do advisory); HTTP só é aceito para localhost. |
| `OPENAI_API_KEY` | Bearer alternativo quando `TYPESAFE_API_KEY` não está definida (proxies OpenAI-compatible). |
| `OPENAI_BASE_URL` | Raiz OpenAI-compatible para `core::providers::openai_compatible` (ex.: proxy 9router). |
| `NINE_ROUTER_BASE_URL` | Alias documentado para a mesma raiz; precede `OPENAI_BASE_URL`. |
| `NVIDIA_API_KEY` / `NGC_API_KEY` | Bearer para NVIDIA NIM (`core::providers::nvidia_nim`); `NGC_API_KEY` é fallback. |
| `BOT_AGENCY` | Quando definida, `serve --with-monitor` usa `RegistryMonitorAgentHook` para a agência (registry compartilhado com HTTP agents). |
| `BOT_HTTP_OWNER_ID` | Com `BOT_HTTP_ADMIN_TOKEN`, restringe `owner_id` no registro de agentes ao valor configurado. |
| `BOT_HTTP_AGENCY_ID` | Restringe rotas `/api/v1/agents*` ao `agency` configurado (query ou body); falha **403** `http_agency_mismatch`. `GET /meta` → `http_agency_binding_active` (booleano, sem expor o ID). |
| `BOT_HTTP_ADMIN_TOKEN` | Quando não vazio, rotas HTTP mutantes exigem `Authorization: Bearer <token>` (fail-closed; não substitui auth do owner). |
| `BOT_RUNTIME_ENABLED` | `true` ativa `InMemoryBotRuntime` (promoção/demote em processo); default/false fail-closed (**503** em promote). |
| `BOT_ORDERS_EXCHANGE_SUBMIT` | Com `BOT_ORDERS_EXECUTION=live_exchange`, `recording` liga `ExchangeSpotExecutor` (sem rede; dev/test). Futuro: `testnet` para REST Spot. |
| `BOT_ORDERS_EXECUTION` | vazio/`disabled` (fail-closed); `dev_accept` (double local); `paper` (`PaperLedgerExecutor`, ledger in-process); `live_exchange` + `BOT_ORDERS_EXCHANGE_SUBMIT=recording` → **200** após risco; `live_exchange` sem submit backend — **503** `live_exchange_not_wired`). Outros valores → `disabled`. |
| `client_order_id` (body HTTP) | Campo opcional em `POST /api/v1/orders/submit`; replays retornam `accepted: true` sem reexecutar (memória; PG quando `DATABASE_URL` + migração `0004`). |
| `BOT_AGENTS_ENABLED` / `BOT_NEO4J_*` | Grafo Neo4j opcional para agentes; ver `docs/operations/postgres-and-graph-dev.md`. |
| `NVIDIA_NIM_BASE_URL` | Raiz da integrate API (default `https://integrate.api.nvidia.com`); opcional em TOML como `providers.nim_base_url`. |

Nunca comite `.env` ou credenciais.

## Persistência

- O backtest usa `--persist` para gravar o dataset sintético.
- O monitor só persiste com `PERSIST_MARKET_DATA=1`, `DATABASE_URL` e timeframe `1m`.
- As migrações ativas ficam em `src/core/database/migrations/` (via `PostgresDatabase::migrate` / `Database::migrate`).
- Falha de conexão não deve ser tratada como prova de histórico completo.

A semântica de estados, gaps e recuperação está no [SDD T-15](../sdd/monitor-persistence-policy-sdd.md). O fluxo de candles e avaliação está no [SDD T-10](../sdd/monitor-pause-resume-sdd.md).

## Contratos de segurança

O backend trabalha com dados públicos e não envia ordens no fluxo atual. Redirects entre origens são tratados pelo [SDD T-05](../sdd/rest-redirect-sdd.md). Alterações de configuração, mercado e validação devem seguir o [SDD T-03](../sdd/backend-corrections-sdd.md).

