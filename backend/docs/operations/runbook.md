---
title: Runbook operacional do backend
description: Procedimentos de inicialização, operação, diagnóstico e recuperação do backend
tags:
  - operations
  - backend
  - runbook
---
# Runbook operacional do backend

## Escopo

O backend é um processo de terminal para leitura de dados públicos da Binance Spot Test Network, avaliação de estratégia e modo paper/observe. A criação de sinal não envia ordem. O fluxo de pausa e retomada está especificado em [SDD T-10](../sdd/monitor-pause-resume-sdd.md), e a persistência opcional em [SDD T-15](../sdd/monitor-persistence-policy-sdd.md).

## Pré-voo

1. Execute a partir de `backend/`.
2. Confirme que o arquivo de configuração está legível.
3. Use apenas credenciais por variáveis de ambiente; nunca registre ou comite segredos.
4. Para persistência, confirme `DATABASE_URL` apontando para o banco `trading_bot` e verifique as migrações ativas antes de iniciar.
5. Comece em `dev` com `observe` ou `paper`. O modo HFT é rejeitado pelo backend atual.

## Inicialização

Monitor:

```sh
cargo run -- --config src/core/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative
```

Backtest:

```sh
cargo run -- backtest --config src/core/config/bot.toml
```

Persistência é opt-in. Para o backtest, use `--persist`. Para o monitor, defina `PERSIST_MARKET_DATA=1` e forneça `DATABASE_URL`; mantenha o timeframe em `1m`.

## Operação durante a execução

- Pressione Espaço para pausar ou retomar a avaliação.
- Pressione `q` ou Esc para encerrar.
- Observe a TUI, stderr e os logs JSON rotacionados.
- Em `1m`, o monitor combina WS e REST; perda do WS deve manter o caminho REST ativo.
- Uma pausa confirmada impede novas avaliações; a retomada exige backfill REST válido antes de avaliar.

As regras de redirects e validação da janela REST estão detalhadas no [SDD de redirects REST](../sdd/rest-redirect-sdd.md) e no [SDD de correções do backend](../sdd/backend-corrections-sdd.md).

## Diagnóstico rápido

| Sintoma | Ação |
|---|---|
| Configuração não encontrada ou inválida | Confirme o caminho de `--config`, TOML e combinações de modo/timeframe na [referência de CLI e configuração](../reference/cli-and-config.md). |
| HFT rejeitado | Use um modo suportado; o backend atual é baseado em polling REST e não oferece infraestrutura HFT. |
| WS indisponível | Verifique logs e conectividade; mantenha a execução se REST estiver ativo. |
| Persistência indisponível | Trate o aviso como degradação, valide `DATABASE_URL` e não interprete o arquivo como histórico completo. |
| Janela REST rejeitada | Corrija a origem dos dados ou aguarde nova janela válida; não persista a janela rejeitada. |
| Redirect externo rejeitado | Preserve o erro e investigue a origem configurada; não relaxe a política sem revisar o [SDD T-05](../sdd/rest-redirect-sdd.md). |
| `GET /healthz` com `status: degraded` e `graph_projection_outbox` | Backlog na tabela `graph_projection_outbox` (pending/retry ou idade); confirme Neo4j Bolt, `DATABASE_URL` e worker F2.1.2; drain manual F2.1.3 abaixo. |
| `graph query` ou admin graph HTTP **503** `graph_query_unavailable` | Stack de grafo desligada ou Neo4j indisponível; não é falha de orders/paper. Ver [postgres-and-graph-dev](./postgres-and-graph-dev.md). |

## Grafo de produto — outbox Neo4j (F2.1.2 / F2.1.3)

Projeção **write-only** (agents/bots/orders) com fila PG `graph_projection_outbox`. Detalhes: [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md).

**Pré-requisitos:** `DATABASE_URL` → `trading_bot`; `BOT_GRAPH_ENABLED=true` (ou legado `BOT_AGENTS_ENABLED`) + `BOT_NEO4J_*`. Sem isso, o processo segue sem worker/outbox no health.

**F2.1.2 — worker em background:** no `serve` (e monitor com PG+Neo4j), `spawn_graph_projection_outbox_worker` drena a outbox periodicamente. Intervalo: `neo4j.graph_projection_outbox_drain_secs` em `system.toml` (default 30) ou `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS` (`0` desliga). Lote: `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_BATCH` (default 32).

**Sinal operacional:** `GET /healthz` expõe `graph_projection_outbox` (`pending`, `retry`, `oldest_pending_age_secs`, `degraded`) e `status: degraded` quando há backlog relevante. `/readyz` continua fail-closed só se PG ou Neo4j estiverem down (não duplica a política da outbox).

**F2.1.3 — drain manual (ops):**

```sh
cd backend
export DATABASE_URL=postgresql://USER:PASSWORD@127.0.0.1:5432/trading_bot
export BOT_GRAPH_ENABLED=true
export BOT_NEO4J_URI=bolt://127.0.0.1:7688
export BOT_NEO4J_USER=neo4j
export BOT_NEO4J_PASSWORD=<local-only>
cargo run --locked -- graph-projection drain --limit 32
```

Saída JSON: `{ "processed", "succeeded", "failed" }`. Fail-closed com mensagem clara se PG ou Neo4j ausentes.

**F3 — leitura advisory (sem mutação):** CLI `cargo run --locked -- graph query agents --limit 32` (também `supervision-chain`, `bots-for-agent`, `code-impact --module-path …`; `code-impact` depende do push externo do grafo de código via `scripts/sync-code-graph-neo4j.sh` + CLI `graphify` — sem ele retorna lista vazia). HTTP admin read-only com bearer: ver [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md). Não habilite `live_exchange` nem prod REST como recuperação de grafo.


## Orders PG — retenção Gate 2 (purge)

Pré-requisito: `DATABASE_URL` → `trading_bot`. Não habilita trading live.

```sh
cd backend
export DATABASE_URL=postgresql://USER:PASSWORD@127.0.0.1:5432/trading_bot
cargo run --locked -- orders retention-purge
cargo run --locked -- orders retention-purge --apply   # somente após revisar JSON e backup
```

Saída JSON: `dry_run`, `idempotency_rows_deleted`, `reconciliation_terminal_rows_deleted`, `pending_stale`. Política: [cli-and-config § PG orders retention](../reference/cli-and-config.md#pg-orders-retention-gate-2).

## Encerramento e recuperação

1. Encerre com `q` ou Esc.
2. Preserve stderr e os logs JSON da execução.
3. Se houver falha de persistência, mantenha o monitor em observe/paper e desative o opt-in até a causa ser corrigida.
4. Não habilite produção ou ordens como tentativa de recuperação.
5. Registre a ocorrência no plano de execução quando ela alterar um gate ou bloqueio.

## Verificação antes de aceitar uma alteração

Gate canônico (mesmo comando do job `rust` da CI; o job ainda não passou no CI — ver [test-matrix](../reference/test-matrix.md)):

```sh
./scripts/verify-backend-gates.sh
```

Inclui `fmt`, `clippy --bin bot`, import-direction, testes do bin `bot` com `--test-threads=1`, cinco suítes em `tests/` e `scripts/assert-completeness-evidence.sh` (valida [modules-completeness-evidence.json](../planning/modules-completeness-evidence.json) contra a contagem do gate e o manifesto PG). Com PostgreSQL descartável (`DATABASE_URL` → `trading_bot`):

```sh
./scripts/verify-backend-full.sh
```

HTTP mutante/bearer (paridade local):

```sh
cargo test --locked --bin bot http_integration -- --test-threads=1
```

Baseline esperada (2026-09-27): linha `OK:` do gate → **518** passed, **0** ignored no bin `bot`; `http_integration` → **62** passed; com PG (18+) → `run-pg-integration-tests.sh` executa o manifesto de **28** testes (nenhuma execução registrada em evidência até 27/09). Baseline e detalhes: [auditoria de completude](../planning/modules-completeness-audit.md#verificação-local). O [plano de execução](../planning/backend-work-plan.md) registra gates T-03…T-15 e a trilha paralela de completude de módulos.
