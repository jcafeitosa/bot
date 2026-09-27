---
title: SDD W0-05 — promoções de bots e ledger paper persistidos
description: Fatia Onda 0 que grava promoções (autor declarado, não autenticado) e fills paper do caminho HTTP no PostgreSQL, com checagens antes da TX, aplicação infalível depois do commit e recuperação no boot do serve
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

- **Estado:** draft, ciclo 2 do G1 (ciclo 1: REPROVADO). Nenhum gate aprovado. Precisa de Critic independente (G1) e de acordo do Julio sobre os seams antes do primeiro teste.
- **Plano:** W0-05 em [master-plan](../planning/master-plan.md) §4.1.
- **SDDs relacionados (não repetidos aqui):** [bots-runtime-live-gate2-sdd](./bots-runtime-live-gate2-sdd.md) (lista "Persistência de promoção (PG vs memória)" como questão aberta), [bots-catalog-persistence-gate1-sdd](./bots-catalog-persistence-gate1-sdd.md), [orders-module-sdd](./orders-module-sdd.md), [wave0-12-order-ambiguous-claim-sdd](./wave0-12-order-ambiguous-claim-sdd.md) (fases P1–P5 do submit).
- **[SEGURANÇA]:** promoção mexe em autoridade. O autor gravado é **declarado, não autenticado** até P1: `promoted_by` vem do corpo da requisição (F-ORG-03 em [org-module-threat-model](../security/org-module-threat-model.md)). Esta fatia não resolve autenticação; só registra de forma durável e honesta.

## Contexto (evidência no código, HEAD `d42b71a5`)

- `modules/bots/adapters/runtime_port.rs`:
  - trait `BotRuntimePort` com `status`, `promote`, `demote` (`:8-12`);
  - `FailClosedBotRuntime` rejeita `promote` e `demote` com `RuntimeDisabled` (`:25-31`);
  - `InMemoryBotRuntime` guarda um único slot `Mutex<Option<BotPromotionRecord>>` (`:35-37`);
  - `promote` valida o request e **cria o registro e o timestamp internamente** (`now_unix_ms()`, `:63-73`);
  - `demote` sem promoção ativa devolve `RuntimeNotPromoted` (`:75-82`);
  - `shared_bot_runtime()` é singleton do processo (`:94-97`) e escolhe a implementação por `BOT_RUNTIME_ENABLED` (`bot_runtime_from_env`, `:85-91`).
- `presentation/http/state.rs:358-428` (`promote_bot_http`): valida owner e agência, checa o catálogo (`assert_bot_promotion_allowed`, `:397-403`), muda a memória (`bots_runtime::promote_bot`, `:405-406`) e **só depois** enfileira a projeção no PG (`enqueue_bot_promotion_graph_projection`, `:407-418`). Se o PG falhar, a memória já mudou.
- `state.rs:430-450` (`demote_bot_http`): mesma ordem, memória primeiro (`:432`), outbox depois (`:434-441`).
- `BotPromotionRecord` (`bots/models/runtime.rs:14-19`): `bot_id`, `promoted_by`, `promoted_at_unix_ms`, `state`.
- `modules/orders/adapters/paper_ledger_executor.rs:21`: `static LEDGER: Mutex<Vec<PaperFill>>`. `OrderExecutionPort::execute` é síncrono e devolve `()` (`execution_port.rs:3-5`); o executor paper grava no vetor dentro do `execute`. `PaperFill` tem `symbol`, `side`, `quote_amount: f64`, `fill_unit_price: Option<f64>`, sem `client_order_id` nem horário. `GET /portfolio/paper-snapshot` (`http_bridge/portfolio.rs:31-67`) lê esse vetor.
- O monitor em modo `paper` usa o mesmo `PaperLedgerExecutor` (`supervisor.rs:231-236`); em `serve --with-monitor` os fills do monitor e os do HTTP vão para o mesmo vetor.
- `0011_monitor_supervisor_snapshot.sql`: `promoted_bot_id` é só metadado advisory, não SoT.
- Boot do `serve` (`state.rs:229-330`): hidrata reconciliação (`:292`) e depois chama `persist_bot_catalog` (`:299`), que faz `DELETE FROM bot_catalog_entries` e reinsere (`bots/adapters/pg_catalog.rs:70-74`).

## Decisão

### D1 — Promoções: checar, gravar, aplicar

Com PG, `promote_bot_http` e `demote_bot_http` seguem esta ordem, sob um lock (D2):

1. **Checagens, todas antes da TX:** validação do request; owner e agência (código atual); catálogo (`assert_bot_promotion_allowed`); runtime habilitado (`status().runtime_enabled`); no demote, existe promoção ativa (`status().active.is_some()`). Qualquer falha devolve o erro atual e não toca PG nem memória.
2. **Registro montado fora do runtime:** `state.rs` monta o `BotPromotionRecord` com o timestamp do relógio do `ApiState`. O mesmo valor vai para o PG e para a memória.
3. **TX PG:** `INSERT` do evento (`promoted` ou `demoted`) + enqueue do outbox de grafo, na mesma TX. Falha → **503** `bot_promotion_store_unavailable`, memória intacta.
4. **Aplicação em memória depois do commit, infalível:** `apply_promotion(record)` ou `apply_demotion()`. Não valida de novo; as checagens do passo 1 já rodaram sob o mesmo lock, então o estado não mudou no meio.

**Alternativa rejeitada:** aplicar em memória primeiro e, se o PG falhar, gravar um evento compensatório (ou desfazer a memória). Deixa uma janela em que o estado exposto não existe no PG, e o compensatório pode falhar também. Rejeitada.

### D2 — Concorrência

- Um lock **global** (`tokio::sync::Mutex<()>` no `ApiState`) cobre checagens, TX PG e aplicação. Global e não por bot, porque o runtime tem um único slot ativo: `promote(A)` e `promote(B)` disputam o mesmo slot.
- O volume é baixo (ação administrativa manual), então um lock global não gera contenção.
- Premissa: um único processo `serve` por banco. Com dois processos no mesmo PG, o lock em memória não basta.

**Alternativa registrada:** lock de linha no PG (`SELECT … FOR UPDATE` numa linha de controle) + coluna de versão (otimista). Cobre vários processos, mas custa mais do que a premissa atual pede. Entra se houver mais de um `serve` por banco.

### D3 — Hydrate no boot

Depois de `persist_bot_catalog` (`state.rs:299`), o boot lê o último evento de promoção e decide:

| Situação | Ação |
|---|---|
| Sem evento, ou último evento `demoted` | nada |
| Runtime desabilitado (`BOT_RUNTIME_ENABLED` falso) | **não ativa**; `warn` no log; `GET /meta` → `bot_promotion_hydration = skipped_runtime_disabled` |
| Bot fora do catálogo ou reprovado por `assert_bot_promotion_allowed` (ex.: timeframe mudou) | **não ativa**; `warn`; evento de auditoria `hydration_skipped` (com motivo) na mesma tabela append-only; a linha original fica; `/meta` → `skipped_not_allowed` |
| Caso normal | `apply_promotion(record)`; `/meta` → `restored` |

Evento `hydration_skipped` não conta como estado: a promoção ativa é o último `promoted`/`demoted`. Nenhum caso ativa algo em silêncio.

### D4 — Ledger paper e as fases de W0-12

Fases de W0-12: P1 claim → P2 pré-executor → P3 `pending` no ledger de reconciliação (só `live_exchange`) → P4 executor → P5 pós-executor (`mark_completed` no PG, depois memória).

- **Paper não tem P3:** não há reconciliação de ordem simulada. W0-05 não muda P1–P4.
- **P4 em paper com PG:** o executor calcula o fill e **não** grava em memória. Variante `PaperFillExecutor` com o mesmo cálculo do `PaperLedgerExecutor`. O fill é função pura do request (símbolo, lado, valor, preço), então `state.rs` o reconstrói sem mudar a assinatura de `execute`.
- **P5 em paper com PG — é aqui que W0-05 muda W0-12:**
  - uma TX faz `mark_completed(key)` e o `INSERT` do fill (chave `client_order_id`);
  - depois do commit, a memória recebe `record_completed` e o fill;
  - se a TX falhar, nada foi gravado nem aplicado e não houve efeito externo, então o claim é liberado e a resposta é **503**.
  - Isso é uma exceção ao "P5 nunca libera" de W0-12, que existe por causa de efeito externo em `live_exchange`. É coerente com a regra de W0-12 para modos não-live (erro no executor libera). O SDD de W0-12 precisa registrar essa exceção.
- **Sem PG:** comportamento de hoje (`PaperLedgerExecutor` grava em memória no `execute`); `GET /meta` → `paper_ledger_persistence = memory`.
- **Monitor** (`supervisor.rs:231-236`): continua com `PaperLedgerExecutor` em memória. **Fora de escopo**, com follow-up nomeado **FU-W0-05-monitor-fills**: persistir os fills paper do monitor, a planejar junto de W2-03 (ledger injetado).

### D5 — Demais

- Migração na **próxima sequência livre depois da `0012` (reservada para W0-12)**, conferida no momento da implementação. `0011` continua advisory.
- Sem FK de promoções para `bot_catalog_entries`, porque o boot apaga e reinsere o catálogo (D3 trata o bot ausente).
- Sem PG: promote e demote → **503** `bot_promotion_persistence_unavailable`, memória intacta (mesmo raciocínio de W0-12: `live_exchange` sem PG recusa o boot). **Alternativa registrada, reversível pelo owner:** promoção em memória explícita, com `/meta` → `promotion_persistence = memory`.
- Escopo de processo: `serve` (inclui `--with-monitor`). O monitor TUI em processo separado não hidrata nem grava promoção; fica para W2-03/W2-05.

**Alternativa considerada (modelo de dados):** uma linha "estado atual" (UPSERT) em vez de eventos append-only. Perde o histórico de quem promoveu e despromoveu. Rejeitada para promoções; para fills não se aplica (já são um log).

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| `BotRuntimePort::apply_promotion(&self, record: BotPromotionRecord)` | novo; sem retorno de erro; não valida; registro e timestamp vêm de fora e são os gravados no PG. Em `FailClosedBotRuntime` nunca é chamado (a checagem de runtime habilitado barra antes); implementação vazia com `debug_assert!` |
| `BotRuntimePort::apply_demotion(&self)` | novo; mesmas regras |
| `promote`/`demote` do port | continuam para o caminho de teste do bridge (`http_bridge/bots_runtime.rs:9-16`); o `ApiState` passa a usar só `status` + `apply_*` |
| `BotPromotionStore` | `append_tx(tx, event)`, `load_last_state_event() -> Result<Option<PromotionEvent>, StoreError>`, `append_hydration_skipped(bot_id, reason)` |
| Evento de promoção | `bot_id`, `kind` (`promoted` \| `demoted` \| `hydration_skipped`), `declared_by`, `author_authenticated BOOLEAN NOT NULL DEFAULT false`, `agency_id` opcional, `reason` opcional, `at_ms` |
| `BotPromotionRecord` | ganha `author_authenticated: bool` (sempre `false` até P1); `GET /bots/runtime/status` expõe o campo |
| `PaperLedgerStore` | `append_fill_tx(tx, fill)`, `load_all() -> Result<Vec<PaperFill>, StoreError>` |
| `PaperFill` | ganha `client_order_id: Option<String>` e `at_ms`; `Decimal` no store e `NUMERIC` no PG; o request de ordem continua `f64`, convertido na borda (F-ORD-15 fora daqui) |
| DTO de posições | sem tabela de posições; `GET /portfolio/paper-snapshot` não muda |
| Erros HTTP | 503 `bot_promotion_store_unavailable` (falha PG), 503 `bot_promotion_persistence_unavailable` (sem PG) |
| `GET /meta` | `bot_promotion_hydration` (`restored` \| `none` \| `skipped_runtime_disabled` \| `skipped_not_allowed`), `paper_ledger_persistence` (`postgres` \| `memory`) |

## Critérios de aceite

- A1. Com PG: promover, reconstruir o `ApiState` sobre o mesmo PG, `GET /bots/runtime/status` devolve o mesmo registro (mesmo `promoted_at_unix_ms`) com `author_authenticated=false`.
- A2. Falha injetada na TX de promote → 503 `bot_promotion_store_unavailable`, `status` inalterado, zero linhas novas de evento e de outbox. Idem para demote.
- A3. **Caminho HTTP:** com PG, `POST /orders/submit` em paper, reconstruir o `ApiState`, `GET /portfolio/paper-snapshot` igual ao de antes. Os fills paper do monitor em `serve --with-monitor` ficam fora deste critério (FU-W0-05-monitor-fills).
- A4. Fill com `client_order_id` repetido não duplica linha.
- A5. Falha injetada na TX de P5 paper → 503, claim liberado, nenhum fill no PG nem em memória; retry com a mesma key executa uma vez.
- A6. **Concorrência:** N promote e demote simultâneos (bots diferentes, `tokio::spawn`) terminam com o último evento do PG igual ao estado em memória.
- A7. `demote` sem promoção ativa → erro atual (`RuntimeNotPromoted`), nenhuma linha no PG. Runtime desabilitado → erro atual (`RuntimeDisabled`), nenhuma linha no PG.
- A8. Hydrate: runtime desabilitado → nada ativo e `/meta` `skipped_runtime_disabled`; bot removido do catálogo → nada ativo, evento `hydration_skipped` gravado, linha original intacta, `/meta` `skipped_not_allowed`.
- A9. Sem PG: promote e demote → 503 `bot_promotion_persistence_unavailable`, `status` inalterado; submit paper aceito e `/meta` mostra `paper_ledger_persistence = memory`.
- A10. O store de promoção só faz `INSERT` (teste de que não há `UPDATE`/`DELETE`).
- A11. Testes PG novos entram no manifesto de `run-pg-integration-tests.sh` (contagem conferida pelo próprio manifesto).

## Testes existentes afetados

Levantados com `rg -n "fn [a-z0-9_]*(promot|demot)[a-z0-9_]*\(" src` no HEAD `d42b71a5`. O Builder confere quais rodam sem PG e esperam sucesso: esses passam a esperar 503 (A9) ou a rodar com PG.

- `presentation/http/http_integration_tests.rs`: `bots_runtime_promote_requires_admin_bearer_when_enabled`, `bots_runtime_demote_requires_admin_bearer_when_enabled`, `bots_runtime_promote_and_demote_succeed_with_admin_bearer`, `bots_runtime_promote_denied_when_bound_agency_without_capable_agent`, `bots_runtime_promote_allowed_when_bound_agency_and_capable_agent`, `bots_runtime_promote_rejects_promoted_by_mismatch_when_product_owner_verified`.
- `presentation/http/state.rs`: `persist_catalog_then_promote_monitor_registry_v2_bot`, `persist_catalog_then_promote_bot_http_with_in_memory_runtime`, `promote_bot_http_enforces_agent_capability_when_agency_bound`, `promote_bot_http_rejects_owner_mismatch_when_product_owner_verified`, `promote_bot_http_mutates_same_runtime_arc_used_by_strategy_binding`, `promote_bot_http_rejects_when_postgres_without_owner_bootstrap`, `demote_bot_http_clears_in_memory_runtime_after_promote`.
- Port e bridge (sem mudança de comportamento esperada, mas tocam a trait): `modules/bots/tests.rs` (`fail_closed_runtime_rejects_promotion`, `in_memory_runtime_promote_and_demote`), `modules/http_bridge/mod.rs` (`bots_runtime_bridge_promote_and_demote_in_memory`).

## Dependências

- W0-02 para G4 (teste PG precisa falhar sem PG).
- W0-12: D4 depende das fases P1–P5 e acrescenta a exceção de P5 em paper; W0-05 entra depois de W0-12.
- W0-13: não é pré-requisito. O fill é gravado em `state.rs` (async), fora do executor síncrono.
- Habilita W0-11 (reintrodução futura do ramo testnet exige promoção persistida) e o fechamento de bots G2 (P1).

## Riscos

- O lock global serializa promote e demote; aceitável pelo volume. Com mais de um `serve` por banco, vale a alternativa de D2.
- `apply_*` "infalível" ainda pode entrar em pânico se o `Mutex` interno estiver envenenado (`expect("bot runtime lock")`); é o mesmo risco de hoje.
- `static LEDGER` e `shared_bot_runtime` globais continuam (W2-03); testes seguem com `--test-threads=1`.
- `declared_by` pode ser confundido com autor verificado; o campo `author_authenticated=false` e o nome da coluna existem para evitar isso.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável.

## Rollout / rollback

- Rollout: próximo build. Sem PG, promote e demote passam a responder 503 (hoje aceitam em memória); o ledger paper segue em memória com o flag em `/meta`. Nenhum deploy autorizado.
- Rollback: reverter o código volta à memória; as tabelas novas ficam órfãs e inofensivas; remover exige outra migração na próxima sequência livre.
