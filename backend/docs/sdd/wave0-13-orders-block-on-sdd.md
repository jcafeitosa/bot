---
title: SDD W0-13 — orders sem block_on dentro do runtime Tokio
description: Fatia Onda 0 que remove Runtime::block_on chamado a partir de contexto async no caminho testnet de submit/reconcile (F-ORD-11 / SEC-ORD-17)
tags:
  - sdd
  - backend
  - orders
  - exchanges
  - wave0
status: draft
---

# SDD W0-13 — orders sem `block_on` dentro do runtime Tokio

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre os seams antes do primeiro teste.
- **Origem:** observação O3 do Critic; achado **F-ORD-11** e critério **SEC-ORD-17** em [orders-g2-threat-model](../security/orders-g2-threat-model.md). ID novo, fora de [master-plan](../planning/master-plan.md) §4.1; proposto no lote 1 junto de [W0-12](./wave0-12-order-ambiguous-claim-sdd.md).
- **SDDs relacionados:** [orders-live-execution-gate2-sdd](./orders-live-execution-gate2-sdd.md).

## Contexto (evidência no código, HEAD `b2001a7c`; análise estática)

- `backend/src/modules/exchanges/adapters/binance_spot_testnet_submit.rs:20-28`: `ccxt_runtime()` cria um runtime Tokio `current_thread` estático.
- Mesmo arquivo, `:166-172`: `submit_testnet_spot_market_order` chama `ccxt_runtime().block_on(...)`.
- `backend/src/modules/exchanges/adapters/binance_spot_testnet_reconcile.rs:64`: `observe_testnet_spot_order_by_client_id` chama `ccxt_runtime().block_on(...)`.
- `backend/src/modules/orders/adapters/live_reconciliation_pg_mirror.rs:10-16, 37`: `MIRROR_RUNTIME.block_on(store.upsert_state(...))`. Único chamador em produção: `supervisor.rs:84` (monitor). **Não** está no caminho de reconciliação disparado por `state.rs` (o poll HTTP grava via `order_reconciliation_pg` async). Hoje latente: o espelho só é registrado no boot HTTP e o ramo testnet do monitor é inalcançável (W0-11). Pela regra do Critic, entra como escopo **recomendado**, não obrigatório; se ficar fora, vira pré-condição de W0-11.
- Caminho de chamada do submit: handler axum async → `ApiState::submit_order_http` (`state.rs:512`, async) → chamada direta, sem `spawn_blocking`, de `http_bridge::orders::submit_order_http` (sync, `state.rs:560`) → `OrderExecutionPort::execute` (sync, `execution_port.rs:3-5`) → `spot_order_submit::submit_spot_order` (`spot_order_submit.rs:70-82`) → `block_on`. O reconcile vem de `run_order_reconciliation_poll_once` (async) → `spot_order_reconciliation_query.rs:94` → `block_on`.
- Documentação do Tokio (`Runtime::block_on`, seção Panics): "panics … if called within an asynchronous execution context". Logo, com `orders.exchange_submit=testnet` e credenciais, o submit HTTP e o poll de reconciliação entram em pânico antes de enviar qualquer coisa. O backend `recording` não passa por `block_on` e não é afetado.
- Por que os testes não pegam: o teste existente do reconcile (`binance_spot_testnet_reconcile.rs:118`) é `#[test]` síncrono e retorna antes do `block_on` por falta de credencial; nenhum teste exercita o caminho testnet dentro de um runtime.
- Efeito colateral com W0-12: o pânico acontece depois do `try_claim` PG e antes do `release_claim`; a key fica presa em estado "claimed" e o retry responde `accepted: true` sem ordem.
- **Não verificado dinamicamente** (sem credencial testnet, por regra). O teste RED abaixo é a verificação.

## Decisão

1. **Async de ponta a ponta:** o port de execução e os adapters testnet passam a ser `async`; o `create_order`/`fetch_order` do ccxt é aguardado no runtime principal. `ccxt_runtime()` é removido; `MIRROR_RUNTIME` também, se o escopo recomendado (espelho PG) for aceito. O `OrderExecutionPort` vira async (o crate `async-trait` já é dependência direta), e os chamadores (`http_bridge::orders::submit_order_http`, `controllers::submit_order`, supervisor do monitor) passam a `await`.
2. Com o espelho no escopo, some também o uso do pool `sqlx` (criado no runtime principal) a partir do runtime paralelo.

**Alternativa considerada:** manter os adapters síncronos e envolver a chamada do executor em `tokio::task::spawn_blocking` na fronteira async (`state.rs`, poll e supervisor). Diff menor e sem mudar a assinatura do port, mas mantém dois runtimes, ocupa thread do pool de bloqueio por ordem, deixa o pool PG usado a partir do runtime paralelo e exige lembrar do `spawn_blocking` em cada chamador novo. Fica como plano B se o owner quiser a menor mudança agora.

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| `OrderExecutionPort` | `async fn execute(&self, request) -> Result<(), OrdersError>` (via `async_trait`) |
| Adapters testnet | `async fn submit_testnet_spot_market_order`, `async fn observe_testnet_spot_order_by_client_id` |
| Controller | `submit_order` async; assinatura de `http_bridge::orders::submit_order_http` async |
| Espelho PG | `try_mirror_reconciliation_upsert` async ou removido em favor da escrita já feita em `state.rs` (decisão do Builder com o Critic) |

## Critérios de aceite

- R0 (RED primeiro, via `ApiState`, G3). Teste `#[tokio::test(flavor = "multi_thread")]` que monta `ApiState` com executor `live_exchange` + backend `testnet` e credenciais testnet **falsas** (env sob o lock de teste do repo) e chama `submit_order_http`. O pânico do `block_on` acontece na entrada, antes de qualquer I/O, então o teste não precisa de rede. Na HEAD atual deve falhar com o pânico "within an asynchronous execution context"; comando e saída do red entram na entrega. Mesmo formato para o poll (`run_order_reconciliation_poll_once` com uma key `pending`).
- **Se o RED não reproduzir** (ex.: algum passo antes do `block_on` falha com credencial falsa), a fatia cai de posição no ranking e o achado vira "não confirmado"; registrar o motivo e devolver ao Orquestrador.
- **SEC-ORD-17** de [orders-g2-threat-model](../security/orders-g2-threat-model.md): em runtime multi-thread, o caminho testnet com transporte falso não entra em pânico e não bloqueia outra requisição concorrente além do limite definido pelo Critic.
- A1. `rg 'block_on' backend/src` sem ocorrência nos caminhos de orders/exchanges (fora de testes e de `main`).
- A2. Suite atual de orders, monitor e HTTP verde; nenhum contrato HTTP muda.

## Dependências e arquivos compartilhados

- Compartilha arquivos com [W0-12](./wave0-12-order-ambiguous-claim-sdd.md): `binance_spot_testnet_submit.rs` (W0-12 classifica erro antes/depois do `create_order`, que W0-13 reescreve), `execution_port.rs`, `models/error.rs`, `http_bridge/orders.rs`, `presentation/http/state.rs` (`submit_order_http`). Recomendação: W0-13 primeiro, W0-12 em cima, mesmo Builder ou sequência estrita, para evitar conflito.
- `state.rs` também é tocado por W0-01 (rotas/`require_http_admin`); coordenar ordem de merge.
- Não depende de W0-01; G4 depende de W0-02.

## Riscos

- Assinatura async do port mexe em muitos testes (executores fake/recording). Mitigação: mudança mecânica, revisada pelo Critic.
- `LAST_SUBMIT_ACK` global (F-ORD-05) fica mais exposto com concorrência real; não é corrigido aqui (SEC-ORD-09 é outro item).
- Comportamento do ccxt sob o runtime principal (timeouts, cancelamento) não é verificável estaticamente; o transporte falso cobre o contrato, a testnet real fica como verificação manual opcional.

## Validação

- `backend/scripts/verify-backend-gates.sh` (os testes novos rodam no bin `bot`, sem rede e sem PG).
- Evidência do red→green registrada na entrega (AGENTS.md G3).

## Rollout / rollback

- Rollout: próximo build; só muda comportamento com `orders.exchange_submit=testnet`. Nenhum deploy autorizado.
- Rollback: reverter o commit volta ao `block_on` (o caminho testnet volta a entrar em pânico); sem migração nem dado.
