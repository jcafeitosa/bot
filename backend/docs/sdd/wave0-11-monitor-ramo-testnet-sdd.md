---
title: SDD W0-11 — remoção do ramo testnet inalcançável do monitor
description: Fatia Onda 0 que remove o ramo de ordens testnet do supervisor do monitor, bloqueado por Config::validate, e o espelho PG que só ele usa
tags:
  - sdd
  - backend
  - monitor
  - orders
  - wave0
status: draft
---

# SDD W0-11 — ramo testnet do monitor

- **Estado:** draft. G1 ciclo 1: APROVADO COM FOLLOW-UP; follow-up aplicado (lista de código morto completa e rename do teste PG). Nenhum gate marcado.
- **Plano:** W0-11 em [master-plan](../planning/master-plan.md) §4.1 (recomendado: remover; reintroduzir só na Onda 2/3 com promoção persistida e ator autenticado).

## Contexto (evidência no código, reconferida no HEAD `d42b71a5`)

- `core/config/mod.rs:390-393`: `Config::validate` recusa `RunMode::Testnet` ("testnet order mode is intentionally blocked in v1").
- `modules/monitor/controllers/supervisor.rs:196-247`: ramo `spot_seam = run_mode == Testnet && live_exchange_submit_backend_enabled()`, que submeteria ordem spot testnet a partir do monitor. Inalcançável em produção, porque o `validate` roda antes.
- Helpers só desse ramo: `monitor_submit_execution_mode` (`:62-68`) e `record_monitor_spot_submit_reconciliation` (`:70-90`), que chama `try_mirror_reconciliation_upsert` (`:84`).
- `orders/adapters/live_reconciliation_pg_mirror.rs:37`: o espelho usa `MIRROR_RUNTIME.block_on`. O chamador externo é só o monitor (`supervisor.rs:84`); o registro acontece no boot HTTP (`state.rs:324`). O caminho de reconciliação do `serve` usa `pg.upsert_state(...).await` direto.
- Teste `testnet_run_mode_submits_via_spot_executor_when_recording_seam` (`supervisor.rs:2538`) monta `Config` com `run_mode: RunMode::Testnet` sem passar por `validate`: prova um caminho que o binário não executa.
- `take_last_spot_submit_ack`, `recording_bind_client_exchange`, `confirm_exchange_order` e `shared_live_order_reconciliation_ledger` também são usados pelo `serve` (`state.rs:341`, `:577-584`); ficam no módulo `orders`, só saem os usos do monitor.

## Decisão

1. Remover o ramo `spot_seam` do supervisor e os helpers `monitor_submit_execution_mode` e `record_monitor_spot_submit_reconciliation`. O monitor submete só em `paper` (e não submete em `observe`).
2. Remover o teste que monta `Testnet` sem `validate`; adicionar teste de que `validate` com `Testnet` falha (se ainda não houver).
3. Com o ramo fora, remover o espelho `live_reconciliation_pg_mirror` (`register_…`, `clear_…`, `try_mirror_…`, `MIRROR_RUNTIME`) e o registro/limpeza em `state.rs:324-326`, conforme a regra do mirror abaixo.
4. Manter o enum `RunMode::Testnet` e a mensagem do `validate`, para config antiga falhar com texto claro.
5. Ordens testnet continuam só pelo `serve`, com os gates atuais.

### Código morto que sai junto (conferido no HEAD `d42b71a5`)

O gate roda `cargo clippy --locked --bin bot -- -D warnings` (`verify-backend-gates.sh:5`). O crate é só binário (sem `lib.rs`) e o clippy roda sem `--all-targets`, então item `pub` sem chamador fora de `#[cfg(test)]` vira `dead_code` e quebra o gate. Tirar o ramo obriga a remover também:

| Item | Onde | Por quê |
|------|------|---------|
| imports `live_exchange_submit_backend_enabled` e `ExchangeSpotExecutor` | `supervisor.rs:43` | só usados no ramo (`:197`, `:230`) |
| `monitor_spot_client_order_id` + teste | `supervisor.rs:53-60`; teste em `:2522` | só chamado em `:215`, dentro de `spot_seam.then(...)` |
| `monitor_submit_execution_mode` | `supervisor.rs:62-68` | só no ramo (`:246-247`) |
| `record_monitor_spot_submit_reconciliation` | `supervisor.rs:70-90` | usos do monitor de ledger/bind/ack: `shared_live_order_reconciliation_ledger` (`:75`), `mark_pending` (`:76`), `take_last_spot_submit_ack` (`:79`), `recording_bind_client_exchange` (`:80`), `confirm_exchange_order` (`:81`), `try_mirror_reconciliation_upsert` (`:84`); imports em `:72-73` |
| `spot_seam` e `monitor_client_order_id` | `supervisor.rs:196-197`, `:214-215` | o `client_order_id` só existe com `spot_seam`; no paper fica `None` |
| `SPOT_EXECUTOR` e o `else if spot_seam` | `supervisor.rs:230`, `:232-234` | o executor vira `Paper` → `PaperLedgerExecutor`, senão `FailClosedExecutor` |
| bloco `if let Some(ref client_order_id)` pós-submit | `supervisor.rs:241-261` | chama `record_…` e `project_order_intent_after_submit` (`:248`), o único chamador em produção |
| `project_order_intent_after_submit` + reexport + teste | `orders/adapters/graph_projection.rs:90`; `adapters/mod.rs:28`; teste em `graph_projection.rs:290` | sem o `:248` só sobra o teste; o HTTP usa `order_graph_projection_outbox_messages` na TX do claim. Remover (não manter só com teste) |
| `PgOrderIdempotencyStore::enqueue_graph_projection_outbox_messages` | `orders/adapters/pg_idempotency.rs:84` (doc-comment em `:83` diz "monitor supervisor") | único chamador em produção é `project_order_intent_after_submit` (`graph_projection.rs:105`). Remover, ou manter com doc-comment corrigido se o Builder achar outro chamador real |
| teste `testnet_run_mode_submits_via_spot_executor_when_recording_seam` | `supervisor.rs:2538` | prova o caminho removido (item 2) |
| mirror inteiro | `live_reconciliation_pg_mirror.rs`; `state.rs:324-326`; `pub use` em `orders/mod.rs:15-18` e `adapters/mod.rs:41` | regra do mirror abaixo |

Candidatos que o clippy vai confirmar ou descartar: o import de `GraphProjectionSync` (`supervisor.rs:18`) e o campo `neo4j` do contexto do tick (`:145`, `:164`, `:356`), hoje lidos só em `:249-251`. O Builder remove o que o clippy apontar e anota no PR; não inventa uso novo para calar o aviso.

**Rename do teste PG.** `pg_monitor_supervisor_graph_projection_outbox_same_transaction` (HEAD: `pg_idempotency.rs:255`; working tree, com mudança staged de outra sessão: `:328`; manifesto: `run-pg-integration-tests.sh:33`) fica com nome enganoso, porque o monitor deixa de projetar. Se `enqueue_graph_projection_outbox_messages` sair, o teste sai junto e a linha do manifesto também. Se ficar, renomear para `pg_graph_projection_outbox_enqueue_same_transaction` e atualizar o manifesto no mesmo commit.

**Regra do mirror (única, N3 do master plan):**

- W0-11 decide o destino do ramo que chama o mirror (`supervisor.rs:84`).
- Se W0-11 **remove** o ramo, o mirror (`live_reconciliation_pg_mirror.rs`, com `MIRROR_RUNTIME.block_on`) sai junto, nesta fatia.
- Se W0-11 **mantém** o ramo, tornar o mirror async (sem `block_on`) passa a ser escopo obrigatório **desta** fatia.
- W0-13 não toca o mirror em nenhum caso.

A decisão acima (remover) aplica o primeiro caso.

**Alternativa considerada:** manter o ramo atrás de flag explícita, com testes que passam por `validate`. Mantém código de ordem real no monitor sem promoção persistida (W0-05) nem ator autenticado (P1), e pela regra acima obrigaria esta fatia a tornar o mirror async. Rejeitada para a Onda 0; o owner pode reverter essa escolha, e nesse caso o mirror async entra aqui.

## Seams públicos

- Nenhum HTTP. A superfície `pub use` de `modules/orders` perde `register_live_reconciliation_pg_mirror`, `clear_live_reconciliation_pg_mirror`, `try_mirror_reconciliation_upsert` e `project_order_intent_after_submit` (interno ao crate). `PgOrderIdempotencyStore` pode perder `enqueue_graph_projection_outbox_messages` (ver tabela).

## Critérios de aceite

- A1. `rg -n "spot_seam|record_monitor_spot_submit_reconciliation|MIRROR_RUNTIME|monitor_spot_client_order_id|monitor_submit_execution_mode|SPOT_EXECUTOR|project_order_intent_after_submit|pg_monitor_supervisor_graph_projection" backend/src backend/scripts` sem resultado.
- A1b. `cargo clippy --locked --bin bot -- -D warnings` limpo, sem `#[allow(dead_code)]` novo (`rg -n "allow\(dead_code\)"` no diff sem linha nova).
- A2. Monitor em `paper` e `observe` com o mesmo comportamento (testes existentes passam).
- A3. `Config` com `run_mode = testnet` falha no `validate` com a mensagem atual.
- A4. Reconciliação do `serve` com PG continua gravando (testes PG existentes passam).

## Dependências

- W0-05 só para a reintrodução futura, não para a remoção.
- W0-13 ([wave0-13-orders-block-on-sdd](./wave0-13-orders-block-on-sdd.md)) não toca o mirror; o critério A1 dele tem exceção explícita para `live_reconciliation_pg_mirror.rs`. Essa exceção cai quando W0-11 for feito.

## Riscos

- Remover o espelho pode esconder um uso que o `rg` não pega (macro, `cfg`); o build e os testes PG confirmam.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh`.

## Rollout / rollback

- Rollout: próximo build, sem efeito para config válida. Rollback: reverter o commit.
