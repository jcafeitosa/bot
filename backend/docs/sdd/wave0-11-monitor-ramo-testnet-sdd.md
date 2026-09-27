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

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1).
- **Plano:** W0-11 em [master-plan](../planning/master-plan.md) §4.1 (recomendado: remover; reintroduzir só na Onda 2/3 com promoção persistida e ator autenticado).

## Contexto (evidência no código, HEAD `72eb471d`)

- `core/config/mod.rs:390-393`: `Config::validate` recusa `RunMode::Testnet` ("testnet order mode is intentionally blocked in v1").
- `modules/monitor/controllers/supervisor.rs:196-247`: ramo `spot_seam = run_mode == Testnet && live_exchange_submit_backend_enabled()`, que submeteria ordem spot testnet a partir do monitor. Inalcançável em produção, porque o `validate` roda antes.
- Helpers só desse ramo: `monitor_submit_execution_mode` (`:62-68`) e `record_monitor_spot_submit_reconciliation` (`:70-90`), que chama `try_mirror_reconciliation_upsert` (`:84`).
- `orders/adapters/live_reconciliation_pg_mirror.rs:37`: o espelho usa `MIRROR_RUNTIME.block_on`. O chamador externo é só o monitor (`supervisor.rs:84`); o registro acontece no boot HTTP (`state.rs:324`). O caminho de reconciliação do `serve` usa `pg.upsert_state(...).await` direto.
- Teste em `supervisor.rs:~2554` monta `Config` com `run_mode: RunMode::Testnet` sem passar por `validate`: prova um caminho que o binário não executa.
- `take_last_spot_submit_ack` também é usado pelo `serve` (`state.rs:577`); fica.

## Decisão

1. Remover o ramo `spot_seam` do supervisor e os helpers `monitor_submit_execution_mode` e `record_monitor_spot_submit_reconciliation`. O monitor submete só em `paper` (e não submete em `observe`).
2. Remover o teste que monta `Testnet` sem `validate`; adicionar teste de que `validate` com `Testnet` falha (se ainda não houver).
3. Com o ramo fora, remover o espelho `live_reconciliation_pg_mirror` (`register_…`, `clear_…`, `try_mirror_…`, `MIRROR_RUNTIME`) e o registro/limpeza em `state.rs:324-326`. Isso tira um `block_on` do escopo de W0-13.
4. Manter o enum `RunMode::Testnet` e a mensagem do `validate`, para config antiga falhar com texto claro.
5. Ordens testnet continuam só pelo `serve`, com os gates atuais.

**Alternativa considerada:** manter o ramo atrás de flag explícita, com testes que passam por `validate`. Mantém código de ordem real no monitor sem promoção persistida (W0-05) nem ator autenticado (P1); rejeitada para a Onda 0.

## Seams públicos

- Nenhum HTTP. A superfície `pub use` de `modules/orders` perde `register_live_reconciliation_pg_mirror`, `clear_live_reconciliation_pg_mirror` e `try_mirror_reconciliation_upsert` (interno ao crate).

## Critérios de aceite

- A1. `rg -n "spot_seam|record_monitor_spot_submit_reconciliation|MIRROR_RUNTIME" backend/src` sem resultado.
- A2. Monitor em `paper` e `observe` com o mesmo comportamento (testes existentes passam).
- A3. `Config` com `run_mode = testnet` falha no `validate` com a mensagem atual.
- A4. Reconciliação do `serve` com PG continua gravando (testes PG existentes passam).

## Dependências

- W0-05 só para a reintrodução futura, não para a remoção.
- Coordenar com W0-13: se W0-11 entrar primeiro, W0-13 não precisa tratar o espelho; se W0-13 entrar primeiro, o espelho entra no escopo dele ([wave0-13-orders-block-on-sdd](./wave0-13-orders-block-on-sdd.md)).

## Riscos

- Remover o espelho pode esconder um uso que o `rg` não pega (macro, `cfg`); o build e os testes PG confirmam.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh`.

## Rollout / rollback

- Rollout: próximo build, sem efeito para config válida. Rollback: reverter o commit.
