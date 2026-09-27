---
title: SDD W0-09 — V18: nota de verificação e delta restante
description: Verifica no código o que o commit fcaf14f8 entregou para V18 e lista o que falta do critério W0-09 (rollback com outbox, limpeza, evidência)
tags:
  - sdd
  - backend
  - persistence
  - postgres
  - wave0
status: draft
---

# SDD W0-09 — V18: nota de verificação e delta restante

- **Estado:** draft. Nenhum gate aprovado. **Fluxo normal:** G1 deste SDD → TDD → G4 (ver "Processo" abaixo). Não há mais G1/G4 retroativo.
- **Plano:** W0-09 em [master-plan](../planning/master-plan.md) §4.1. Critério: erro injetado no meio de TX de domínio (identidade+outbox, idempotência+outbox) → nada persiste, outbox vazia; limpeza verificada (DB dropado ou schema vazio); evidência com host/versão.
- **SDD existente:** [monitor-persistence-v18-sdd](./monitor-persistence-v18-sdd.md). Este arquivo não o substitui; registra a verificação e o que falta.

## Processo: implementação fora do gate (revertida)

- Os commits `0b3ea04b` (16:30 COT, manifesto conflitante em `persist_dataset`) e `b8370a75` (16:34 COT, rollback agents + graph outbox) implementaram parte do W0-09 direto na `main`, fora do gate, sem G1 nem G2.
- Por decisão do owner, os dois foram **revertidos** em `origin/main`: `2089a496` (16:39 COT, reverte `b8370a75`) e `afe1f411` (16:39 COT, reverte `0b3ea04b`), já pushados.
- Com isso o W0-09 segue o **fluxo normal**: G1 deste SDD → TDD (red → green) → G4. O delta volta a incluir o que os commits revertidos faziam (fatia agents + outbox e manifesto conflitante); ver "Delta restante".
- **Desvio de processo registrado: `fcaf14f8`** (16:25 COT, "V18 PG rollback e idempotência de persist_dataset"). Entregou escopo do W0-09 fora do gate, sem G1 nem G2:
  - teste `core/persistence/v18_pg_tests.rs` e registro em `core/persistence/mod.rs`;
  - script `scripts/pg-v18-monitor-persistence-audit.sh`;
  - linha no manifesto `scripts/run-pg-integration-tests.sh`;
  - docs: criou [monitor-persistence-v18-sdd](./monitor-persistence-v18-sdd.md) e mexeu em `monitor-persistence-policy-sdd.md`, README e docs de planning, dizendo que a fatia estava fechada.
  - Não mudou comportamento de produção (só teste, script e docs).
  - **Não foi revertido** e continua em `origin/main`. O que ele entregou entra na verificação abaixo e no G4 desta fatia; não conta como aprovado.
- Estado local conferido às ~16:45 COT: `main` local = `origin/main` em `d42b71a5` (que também reverteu os auto-saves `026153ea` e `f2b71da2`), já com os dois reverts; o working tree não contém mais `DatasetManifestConflict` nem `persist_identity_and_enqueue_graph_projection_tx`. A verificação abaixo foi feita em `afe1f411` e reconferida no working tree.
- Este SDD não aprova nada do que já foi entregue.

## Verificação (`origin/main` em `afe1f411`, 2026-09-27 ~16:42 COT; refs de linha reconferidas em `d42b71a5` às ~16:55 COT)

O commit `fcaf14f8` ("V18 PG rollback e idempotência de persist_dataset") diz fechar W0-09/T-15. **Está parcial.**

| Item do critério | Estado | Evidência |
|---|---|---|
| Rollback do caminho de candles | feito, com ressalva | `core/persistence/v18_pg_tests.rs:101` (`pg_persist_dataset_transaction_rollback_and_idempotent_replay`). O helper `insert_dataset_in_tx_then_rollback` (:63-99) **copia o SQL** de `persist_dataset` numa TX própria e faz `ROLLBACK` manual: prova o Postgres, não o código de `persist_dataset`. A parte de replay idempotente usa o código real. |
| Manifesto conflitante | **falta** (commit `0b3ea04b` revertido por `afe1f411`) | Em `afe1f411`, o manifesto PG ainda lista `persist_dataset_rejects_conflicting_manifest_for_same_id` (`run-pg-integration-tests.sh:22`), mas nenhum teste com esse nome existe em `backend/src` → o filtro executa 0 casos. `persist_dataset` usa `INSERT … ON CONFLICT (dataset_id) DO NOTHING` (`core/persistence/mod.rs:59`) e aceita em silêncio um manifesto diferente para o mesmo `dataset_id`; não existe `DatasetManifestConflict`. |
| Identidade + outbox | **falta** (commit `b8370a75` revertido por `2089a496`) | Em `afe1f411` não existem `persist_identity_and_enqueue_graph_projection_tx` nem o teste `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`. Só há `pg_agent_identity_and_graph_projection_same_transaction` (`run-pg-integration-tests.sh:25`), que prova o caminho feliz na mesma TX, não o rollback com erro injetado. |
| Idempotência + outbox (orders) | **em andamento, código não commitado, por outra sessão** | em `afe1f411`, `orders/adapters/pg_idempotency.rs:108` (`persist_idempotency_and_enqueue_graph_projection`) abre e fecha a própria TX. A mudança local (código e manifesto ainda `M`) cria `persist_idempotency_and_enqueue_graph_projection_tx` (`pub(crate)`) e o teste `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure`, com `rollback()` explícito (é o plano B abaixo) |
| Limpeza verificada | **falta** | os testes apagam só as linhas do fixture (`delete_v18_fixture`); nada prova DB dropado ou schema vazio |
| Evidência host/versão | **falta** | `scripts/pg-v18-monitor-persistence-audit.sh` só recusa host não local (:17-24); não registra `server_version` nem host |
| Não passar em skip | **falta** (W0-02) | todos usam `database_for_integration_test()`, que devolve `None` e o teste retorna verde sem PG (`core/persistence/pg_integration.rs:6-7`) |

## Divergência doc × código

- [monitor-persistence-v18-sdd](./monitor-persistence-v18-sdd.md), seção "Estado", no HEAD `d42b71a5` (L70): "W0-09 fatia agents+outbox rollback **fechada**". No working tree local há uma correção não commitada de outra sessão (marca as fatias 2 e 3 como "não entregue"); até ela ser commitada, o HEAD continua dizendo "fechada".
- O código não sustenta isso:
  - a parte de agents foi revertida (`2089a496`) e o manifesto conflitante também (`afe1f411`);
  - a parte de orders não está commitada (`pg_idempotency.rs` e `run-pg-integration-tests.sh` ainda modificados no índice local, por outra sessão);
  - nenhum dos itens tem G1/G2;
  - faltam limpeza verificada, evidência host/versão e o fim do skip silencioso.
- Origem do texto no HEAD (conferido com `git log` e `git diff b8370a75 d42b71a5 -- backend/docs/sdd/monitor-persistence-v18-sdd.md`, sem diferença): a L70 atual é **idêntica à do `b8370a75`**. O auto-save `026153ea` tinha trocado o texto; o `d42b71a5` (revert dos auto-saves) restaurou a versão do `b8370a75`; e o `2089a496`, que reverteu o código do `b8370a75`, não tocou nesse doc. Por isso o doc diz "fechada" para um código que não está mais em `main`.
- Este arquivo não edita o SDD de V18; a limpeza dos outros docs fica com o Guardião e o Critic decide a correção.

## Delta restante (decisão)

1. **Idempotência + outbox:** teste PG que chama `persist_idempotency_and_enqueue_graph_projection` com dois messages, o segundo inválido para o PG (por exemplo, `idempotency_key` com byte `\0`, que o PG recusa em `TEXT`). Espera: `Err`, e zero linhas em `order_idempotency_keys` e em `graph_projection_outbox` para o fixture. Isso prova o caminho de erro do código real, sem seam só de teste.
2. **Candles:** trocar a cópia de SQL por injeção equivalente no `persist_dataset` real (candle inválido no fim do lote) ou registrar no SDD base que o teste prova só o Postgres.
3. **Limpeza e evidência:** fica com o job de W0-02 (DB descartável por execução). O job registra `SELECT version()` e o host no log e termina com o DB dropado; o script V18 passa a imprimir `server_version`.
4. **Identidade + outbox (volta ao escopo; o commit que fazia isso foi revertido):** teste PG de rollback com erro injetado no meio da TX de `agent_identities` + `graph_projection_outbox`, chamando o código real de persistência de identidade; espera zero linhas nas duas tabelas para o fixture. Seam: nenhum público no caminho recomendado; se o Critic aceitar o plano B, uma variante `_tx` `pub(crate)` (como o commit revertido fazia).
5. **Manifesto conflitante (volta ao escopo; o commit que fazia isso foi revertido).** Recomendação:
   - manifesto **idêntico** para um `dataset_id` existente → `Ok`, sem gravar nada (nem manifesto nem candle);
   - manifesto **diferente** → erro tipado (ex.: `DatasetManifestConflict`) na mesma TX, sem gravar nenhum candle;
   - o teste `persist_dataset_rejects_conflicting_manifest_for_same_id` passa a existir e rodar em PG, com os dois casos.
   - **Pré-condição do RED:** a decisão do owner sobre esse comportamento tem de estar registrada (no master plan, §6, com ID `D-*`) antes do primeiro teste. Não achei ID existente com `rg` em `backend/docs`; se a decisão já existir com outro nome, o master plan aponta qual.
   - Alternativas registradas: sobrescrever o manifesto (perde a garantia de imutabilidade do dataset) ou aceitar qualquer manifesto em silêncio (comportamento atual, `ON CONFLICT (dataset_id) DO NOTHING` em `core/persistence/mod.rs:59`).
   - Enquanto isso não entra, o nome no manifesto executa 0 casos, o que viola o critério "cada teste do manifesto roda exatamente uma vez" de W0-02.
6. Idempotência + outbox (em andamento por outra sessão) e identidade + outbox: o Critic confere se `rollback()` explícito depois do enqueue basta para o critério "erro injetado no meio da TX". Se não bastar, somar o teste do item 1 (erro real do código), sem remover os existentes.

**Alternativa considerada:** criar variante `_tx(&mut tx, …)` de `persist_idempotency_and_enqueue_graph_projection`, como já existe em agents, e fazer `rollback()` no teste. Mais uniforme, mas prova só que o PG desfaz uma TX aberta pelo teste, não que o código de produção desfaz em erro. É o caminho que a outra sessão está seguindo; aceitável se o Critic concordar (item 4).

## Seams públicos

Nenhum no caminho recomendado. No plano B: `PostgresOrderIdempotencyStore::persist_idempotency_and_enqueue_graph_projection_tx` e uma variante `_tx` equivalente em agents (visibilidade `pub(crate)`). Item 5: erro tipado de manifesto conflitante no resultado de `persist_dataset` (variante de `PersistenceError`/`DatabaseError` e mapeamento em readiness); o comportamento (idêntico → `Ok` sem gravar; diferente → erro, sem candle) depende da decisão registrada do owner antes do RED.

## Dependências

- W0-02: limpeza verificada, evidência de host/versão e falha sem PG.
- Habilita W0-10 (G4 de T-15).

## Riscos

- A injeção por `\0` depende do comportamento do driver/PG; se o sqlx recusar antes de enviar, o erro ainda acontece no meio da TX (depois do `INSERT` da chave), o que basta.
- Contagem do manifesto muda de novo; usar a linha `OK:` e o manifesto, não número copiado.

## Validação

- `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável; `backend/scripts/verify-backend-gates.sh`.

## Rollout / rollback

- Só testes e scripts; sem efeito em runtime. Rollback: reverter o commit.
