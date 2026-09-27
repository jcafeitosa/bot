---
title: SDD W0-05 — promoções de bots e ledger paper persistidos
description: Fatia Onda 0 que grava promoções (autor declarado, não autenticado) e fills paper no PostgreSQL e os recupera no boot do serve
tags:
  - sdd
  - backend
  - bots
  - portfolio
  - persistence
  - wave0
status: draft
---

# SDD W0-05 — promoções de bots e ledger paper persistidos [SEGURANÇA]

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e de acordo do Julio sobre os seams antes do primeiro teste.
- **Plano:** W0-05 em [master-plan](../planning/master-plan.md) §4.1.
- **SDDs relacionados (não repetidos aqui):** [bots-runtime-live-gate2-sdd](./bots-runtime-live-gate2-sdd.md) (lista "Persistência de promoção (PG vs memória)" como questão aberta), [bots-catalog-persistence-gate1-sdd](./bots-catalog-persistence-gate1-sdd.md), [orders-module-sdd](./orders-module-sdd.md).
- **[SEGURANÇA]:** promoção mexe em autoridade. O autor gravado é **declarado, não autenticado** até P1: `promoted_by` vem do corpo da requisição (F-ORG-03 em [org-module-threat-model](../security/org-module-threat-model.md)). Esta fatia não resolve autenticação; só registra de forma durável e honesta.

## Contexto (evidência no código, HEAD `72eb471d`)

- `backend/src/modules/bots/adapters/runtime_port.rs:35-83`: `InMemoryBotRuntime` guarda a promoção ativa num `Mutex<Option<BotPromotionRecord>>`; `shared_bot_runtime()` (`:94-97`) é singleton do processo. Restart perde a promoção.
- `backend/src/presentation/http/state.rs:358-428` (`promote_bot_http`): valida owner/agência, muda a memória **antes** do PG e depois só enfileira projeção Neo4j (`enqueue_bot_promotion_graph_projection`). Se o PG falhar, a memória já mudou e a resposta é erro.
- `BotPromotionRecord` (`bots/models/runtime.rs:14-19`): `bot_id`, `promoted_by`, `promoted_at_unix_ms`, `state`.
- `backend/src/modules/orders/adapters/paper_ledger_executor.rs:21`: `static LEDGER: Mutex<Vec<PaperFill>>`; `PaperFill` tem `symbol`, `side`, `quote_amount: f64`, `fill_unit_price: Option<f64>`, sem `client_order_id` nem horário. `GET /portfolio/paper-snapshot` (`http_bridge/portfolio.rs:31-67`) lê esse vetor.
- O monitor em modo `paper` escreve no mesmo `LEDGER` do seu processo (`supervisor.rs:231-236`); o monitor TUI e o `serve` são processos diferentes e não veem os fills um do outro.
- `0011_monitor_supervisor_snapshot.sql`: `promoted_bot_id` é só metadado advisory, não SoT.
- Boot do `serve` (`state.rs:229-300`) hidrata agentes e reconciliação e chama `persist_bot_catalog`, que faz `DELETE FROM bot_catalog_entries` e reinsere (`bots/adapters/pg_catalog.rs:70-74`).

## Decisão

1. **Promoções:** tabela nova, append-only, de eventos de promoção (`promoted`/`demoted`), com `bot_id`, `declared_by` (texto do corpo), `author_authenticated BOOLEAN NOT NULL DEFAULT false`, `agency_id` opcional, `state`, `at_ms`. A promoção ativa é o último evento. Com PG ligado, a ordem é: TX PG (evento + outbox) → só então memória. Falha PG → **503**, memória sem mudança.
2. **Ledger paper:** tabela nova de fills paper (`fill_id`, `client_order_id` opcional e único quando presente, `symbol`, `side`, `quote_amount NUMERIC`, `fill_unit_price NUMERIC NULL`, `at_ms`). Grava na mesma operação do submit paper; falha PG → **503** e nada entra na memória.
3. **Boot do `serve`:** hidrata a promoção ativa e os fills do PG antes de abrir o socket, no mesmo ponto em que hoje hidrata agentes e reconciliação.
4. Migração na **próxima sequência livre no momento da implementação** (não fixar número). `0011` continua advisory.
5. Sem FK de promoções para `bot_catalog_entries`, porque o boot apaga e reinsere o catálogo; a checagem de existência continua em `assert_bot_promotion_allowed`.
6. Escopo: processo `serve` (inclui `--with-monitor`). O monitor TUI em processo separado não hidrata nem grava; fica para W2-03/W2-05.

**Alternativa considerada:** uma linha "estado atual" (UPSERT) em vez de eventos append-only. Mais simples de ler, mas perde o histórico de quem promoveu/despromoveu, que é justamente o que falta para auditoria. Rejeitada para promoções; aceitável para fills (já são um log).

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| `BotPromotionStore` | `append(event) -> Result<(), StoreError>`, `load_active() -> Result<Option<BotPromotionRecord>, StoreError>` |
| `PaperLedgerStore` | `append_fill(fill) -> Result<(), StoreError>`, `load_all() -> Result<Vec<PaperFill>, StoreError>` |
| `BotPromotionRecord` | ganha `author_authenticated: bool` (sempre `false` até P1); HTTP expõe o campo em `GET /bots/runtime/status` |
| `PaperFill` | ganha `client_order_id: Option<String>` e `at_ms`; valores em `Decimal` no store e `NUMERIC` no PG. A requisição de ordem continua `f64` e é convertida na borda (`Decimal` desde a requisição é F-ORD-15, fora daqui) |
| DTO de posições | sem tabela de posições: posições continuam derivadas dos fills; o DTO de `GET /portfolio/paper-snapshot` não muda |
| Política sem PG | memória explícita (comportamento de hoje) e `GET /meta` → `promotion_persistence` / `paper_ledger_persistence` = `memory` \| `postgres` — **precisa de decisão** (alternativa: recusar promover sem PG) |

## Critérios de aceite

- A1. Com PG: promover, reiniciar o `ApiState` (novo boot sobre o mesmo PG), `GET /bots/runtime/status` devolve a mesma promoção com `author_authenticated=false`.
- A2. Falha injetada no PG durante promote → 503 e `status` inalterado (memória não muda). Idem para demote.
- A3. Com PG: submit paper, reboot, `GET /portfolio/paper-snapshot` igual ao de antes do reboot.
- A4. Fill com `client_order_id` repetido não duplica linha (idempotente).
- A5. Tabela de eventos de promoção sem `UPDATE`/`DELETE` pelo código da aplicação (teste de que o store só faz `INSERT`).
- A6. Testes PG entram no manifesto de `run-pg-integration-tests.sh`.

## Dependências

- W0-02 para G4 (teste PG precisa falhar sem PG).
- W0-13: com o port de execução async, a gravação do fill cabe no executor paper sem runtime paralelo. Se W0-13 atrasar, gravar o fill em `state.rs` depois do `execute` (async), não dentro do executor síncrono.
- W0-12: o `client_order_id` do fill segue o mesmo claim; não duplicar lógica de idempotência.
- Habilita W0-11 (reintrodução futura do ramo testnet exige promoção persistida) e o fechamento de bots G2 (P1).

## Riscos

- Duas fontes (memória + PG) podem divergir se algum caminho mutar só a memória; mitigação: a memória só muda depois do commit.
- `static LEDGER` e `shared_bot_runtime` globais continuam (W2-03); testes seguem com `--test-threads=1`.
- `declared_by` pode ser confundido com autor verificado; o campo `author_authenticated=false` e o nome da coluna existem para evitar isso.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável.

## Rollout / rollback

- Rollout: próximo build; sem PG, comportamento igual ao atual (com o flag em `/meta`). Nenhum deploy autorizado.
- Rollback: reverter o código volta à memória; as tabelas novas ficam órfãs e inofensivas; remover exige outra migração na próxima sequência livre.
