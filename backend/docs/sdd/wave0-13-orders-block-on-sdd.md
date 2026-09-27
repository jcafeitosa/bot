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

- **Estado:** draft, ciclo 3 do Builder depois do Critic G1 ciclo 2 (APROVADO COM FOLLOW-UP). Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre os seams antes do primeiro teste (seams **a acordar com o owner**).
- **Origem:** observação O3 do Critic; achado **F-ORD-11** e critério **SEC-ORD-17** em [orders-g2-threat-model](../security/orders-g2-threat-model.md). Linha 3 da tabela de prioridades de [master-plan](../planning/master-plan.md) §4.1; executa antes de [W0-12](./wave0-12-order-ambiguous-claim-sdd.md).
- **SDDs relacionados:** [orders-live-execution-gate2-sdd](./orders-live-execution-gate2-sdd.md). Este SDD **define** o port `SpotOrderSubmitPort`/`SpotOrderQueryPort` e o fake de teste; W0-12 (SEC-ORD-08) usa o mesmo port e o mesmo fake.

## Contexto (evidência no código, HEAD `d42b71a5`; análise estática)

Nenhum arquivo citado aqui mudou entre `b8370a75` e `d42b71a5`; as linhas foram conferidas de novo em `d42b71a5`.

- `backend/src/modules/exchanges/adapters/binance_spot_testnet_submit.rs:20-28`: `ccxt_runtime()` cria um runtime Tokio `current_thread` estático.
- Mesmo arquivo, `:158-175` (`submit_testnet_spot_market_order`): `:161-165` fazem, nesta ordem, `request.validate()`, `testnet_credentials()` (lê config/env; sem credencial → `LiveExchangeNotWired`, `:43-50`), `load_registry(Environment::Dev)` (lê arquivos em `src/core/config/exchanges`, `modules/exchanges/bootstrap.rs:11-14`), `dev_spot_account` e `build_dev_spot_binance` (monta o cliente; exige endpoint https exato vindo do arquivo, `binance.rs:30-35`; o host aceito é fixo no código, `https://testnet.binance.vision`, em `exchanges/adapters/account_file.rs:104-124`); só então `:166-172` chamam `ccxt_runtime().block_on(...)`. Dentro do `block_on` vêm as chamadas de rede: `load_markets`, `fetch_ticker` (só venda) e `create_order` (`:76-155`).
- `backend/src/modules/exchanges/adapters/binance_spot_testnet_reconcile.rs:57-64`: `observe_testnet_spot_order_by_client_id` volta `StillPending` sem credencial (`:57-58`); com credencial repete registro/cliente e chama `ccxt_runtime().block_on(...)` (`:64`).
- `backend/src/modules/orders/adapters/live_reconciliation_pg_mirror.rs:10-16, 37`: `MIRROR_RUNTIME.block_on(store.upsert_state(...))`. Único chamador em produção: `monitor/controllers/supervisor.rs:84` (ramo testnet do monitor), inalcançável hoje porque `RunMode::Testnet` é recusado em `core/config/mod.rs:390-393`. **W0-13 não mexe no mirror** (regra única do master-plan): o destino do ramo `supervisor.rs:84` é decidido por [W0-11](./wave0-11-monitor-ramo-testnet-sdd.md). Se W0-11 remover o ramo, o mirror sai junto; se mantiver, tornar o mirror async é escopo obrigatório do próprio W0-11. O poll HTTP grava via `order_reconciliation_pg` async (`state.rs:742-753`) e não passa pelo mirror.
- Caminho do submit, conferido pelo Critic e aqui: handler axum async → `ApiState::submit_order_http` (`state.rs:512`, async) → chamada direta, sem `spawn_blocking`, de `http_bridge::orders::submit_order_http` (sync, `state.rs:560` → `http_bridge/orders.rs:127`) → `controllers::submit_order` (`orders/controllers/submit.rs:6-20`) → `OrderExecutionPort::execute` (sync, `execution_port.rs:3-5`) → `HttpOrderExecutor` (`presentation/http/order_execution.rs:108-119`) → `ExchangeSpotExecutor` (`exchange_spot_executor.rs:10-15`) → `spot_order_submit::submit_spot_order` (`spot_order_submit.rs:71-83`) → `block_on`. O reconcile: `run_order_reconciliation_poll_once`/`reconcile_pending_orders_once` (async, `state.rs:724-775`) → `run_reconciliation_poll_once` (sync, `state.rs:738`) → `spot_order_reconciliation_query.rs:94` → `block_on`.
- Documentação do Tokio (`Runtime::block_on`, seção Panics): "panics … if called within an asynchronous execution context". Logo, com `orders.exchange_submit=testnet` e credenciais, o submit HTTP e o poll de reconciliação entram em pânico **antes de qualquer I/O de rede**, mas **depois** de leitura de env/config e de leitura de arquivo do registro. O backend `recording` não passa por `block_on` e não é afetado.
- Por que os testes não pegam: o teste existente do reconcile (`binance_spot_testnet_reconcile.rs:75` em diante) é `#[test]` síncrono e retorna antes do `block_on` por falta de credencial; nenhum teste exercita o caminho testnet dentro de um runtime.
- Efeito colateral com W0-12: o pânico acontece depois do `try_claim` PG (`state.rs:545`) e antes do `release_claim`; a linha fica em `order_idempotency_keys` e o retry responde `accepted: true` sem ordem (`state.rs:541-543`).
- **Impacto no poll em background:** `server.rs:30-53` roda o poll num `loop` dentro de `tokio::spawn` (`:33-46`). O loop trata `Err` com `warn`, mas um pânico em `run_order_reconciliation_poll_once` derruba a task e a reconciliação para **para sempre**, sem log de erro além do pânico e sem reinício. [W0-12](./wave0-12-order-ambiguous-claim-sdd.md) põe o varredor de claims órfãos no começo de cada tick; com a task morta, o varredor também para em silêncio. Por isso o isolamento por tick entra nesta fatia (A2), não como follow-up.
- **Não verificado dinamicamente** (sem credencial testnet, por regra). R0a abaixo é a verificação.

## Decisão

1. **Async de ponta a ponta (decidido por recomendação; não bloqueia G1).** O `OrderExecutionPort` vira async (o crate `async-trait` já é dependência direta, `Cargo.toml:12`); os chamadores (`http_bridge::orders::submit_order_http`, `controllers::submit_order`, supervisor do monitor `supervisor.rs:221-245`) passam a `await`. `ccxt_runtime()` é removido. O `create_order`/`fetch_order` do ccxt roda no runtime principal.
2. **Port injetável, definido aqui uma vez só** (W0-12 referencia este mesmo port):
   - `SpotOrderSubmitPort` (async, `Send + Sync`): `submit_market(&SubmitOrderRequest) -> Result<SpotOrderSubmitAck, OrdersError>`. Implementações: a struct existente `RecordingSpotOrderSubmitPort` (`spot_order_submit.rs:42`) passa a implementar o trait; `BinanceTestnetSpotOrderSubmit` encapsula o cliente ccxt e faz `load_markets`/`fetch_ticker`/`create_order`.
   - `SpotOrderQueryPort` (async, `Send + Sync`): `find_by_client_order_id(symbol, client_order_id) -> Result<RemoteOrderObservation, OrdersError>`. Implementações: recording e `BinanceTestnetSpotOrderQuery`.
   - **Injeção:** `HttpOrderExecutor` guarda `Arc<dyn SpotOrderSubmitPort>` e `Arc<dyn SpotOrderQueryPort>` no modo `LiveExchange` (deixa de ser `Copy`, continua `Clone`). `from_env()` monta a implementação uma vez no boot: credenciais, registro e cliente ccxt (`:162-165` de hoje) saem do caminho de cada requisição e vão para o boot, sem rede. Falha ao montar → modo `LiveExchangeReserved` com `warn`, como hoje sem credencial.
   - **Fake de teste** (`#[cfg(test)]`, único para W0-13 e W0-12): `FakeSpotOrderPorts` com contador de chamadas, **gate** opcional (`tokio::sync::Notify`: a chamada avisa que entrou e espera o teste liberar; sem atraso por relógio) e roteiro por chamada: `Ack(id)`, `ErrBeforeSend`, `AcceptThenTimeout` (registra a ordem internamente e devolve erro), `Panic` (entra em pânico sem registrar ordem; usado por W0-12 no cenário F-ORD-16 + F-ORD-11) e, para consulta, `Found(state)`/`NotFound`/`Err`.
3. O mirror PG não muda nesta fatia (ver Contexto e A1).
4. **Poll em background supervisionado:** o laço de `server.rs:30-53` é extraído para `spawn_order_reconciliation_poll(state, interval)`, e **cada tick** roda dentro de `catch_unwind` (`futures_util::FutureExt::catch_unwind` com `AssertUnwindSafe`; `futures-util` já é dependência direta, `Cargo.toml:19`). Pânico num tick → log `error` (sem segredo) e o laço segue para o próximo tick. W0-12 coloca o varredor dentro desse mesmo tick.

**Alternativa considerada:** manter os adapters síncronos e envolver a chamada do executor em `tokio::task::spawn_blocking` na fronteira async (`state.rs`, poll e supervisor). Diff menor e sem mudar a assinatura do port, mas mantém dois runtimes, ocupa thread do pool de bloqueio por ordem, deixa o pool PG usado a partir do runtime paralelo, exige lembrar do `spawn_blocking` em cada chamador novo e continua sem seam para teste determinístico. Rejeitada.

## Seams públicos (a acordar com o owner antes do TDD)

| Seam | Proposta |
|---|---|
| `OrderExecutionPort` | `async fn execute(&self, request) -> Result<(), OrdersError>` (via `async_trait`) |
| `SpotOrderSubmitPort` | `async fn submit_market(&self, &SubmitOrderRequest) -> Result<SpotOrderSubmitAck, OrdersError>` |
| `SpotOrderQueryPort` | `async fn find_by_client_order_id(&self, symbol, client_order_id) -> Result<RemoteOrderObservation, OrdersError>` |
| Injeção | `HttpOrderExecutor` com `Arc<dyn …Port>`; montagem no boot em `from_env()`; construtor de teste que recebe os ports |
| Controller | `controllers::submit_order` e `http_bridge::orders::submit_order_http` async |
| Poll em background | laço de `server.rs:30-53` extraído para `spawn_order_reconciliation_poll(state, interval)`, com `catch_unwind` por tick |
| Gate do fake | `tokio::sync::Notify` (entrou / liberar), sem `sleep` |
| Mirror PG | fora desta fatia; decidido por W0-11 |

## Critérios de aceite

- **F-ORD-11** de [orders-g2-threat-model](../security/orders-g2-threat-model.md) (Alto): coberto por R0a (red), R0b (green), SEC-ORD-17 e A1 abaixo. O cenário combinado F-ORD-16 + F-ORD-11 (TM §7.1: pânico depois do claim → retry `accepted: true` sem envio) tem teste próprio em [W0-12](./wave0-12-order-ambiguous-claim-sdd.md), com RED no HEAD pelo mesmo caminho de R0a e GREEN com o fake em `Panic`.
- **R0a — RED de caracterização (HEAD atual, sem rede).** `#[tokio::test(flavor = "multi_thread")]` monta `ApiState` com executor `live_exchange`, `BOT_ORDERS_EXCHANGE_SUBMIT=testnet` e `BINANCE_TESTNET_API_KEY`/`BINANCE_TESTNET_SECRET` **falsos** (env sob o lock de teste do repo) e chama `submit_order_http`. Os passos antes do `block_on` (`binance_spot_testnet_submit.rs:161-165`) não fazem rede: credencial vem do env falso; o registro vem do arquivo versionado `src/core/config/exchanges/binance.toml` (conta spot com `rest_base_url = "https://testnet.binance.vision"`, `:3-8`), que passa no check de endpoint https; `build_dev_spot_binance` só monta o cliente. Esperado hoje: pânico "within an asynchronous execution context" no `:166`. Mesmo formato para o poll (`run_order_reconciliation_poll_once` com uma key `pending` sem binding recording). Comando e saída entram na entrega.
- **Guarda de R0a contra rede real (F-13-2):** um registro fixture não resolve, porque o host do endpoint é fixo no código (`account_file.rs:104-124` só aceita `https://testnet.binance.vision`), e sem pânico o `block_on` faria HTTPS real com chaves falsas. Por isso R0a tem uma guarda que falha **antes** de chamar `submit_order_http`: (1) `tokio::runtime::Handle::try_current()` precisa ser `Ok` (contexto async, condição documentada do pânico do Tokio); (2) o fonte de `binance_spot_testnet_submit.rs`, lido com `include_str!`, ainda contém `ccxt_runtime()` seguido de `.block_on(` em `submit_testnet_spot_market_order`. Se (1) ou (2) falhar, o teste falha na hora, sem chamada e sem rede. R0a roda uma vez, à mão, no HEAD de antes da correção; não entra na CI.
- **R0a não vira teste permanente:** depois da correção, o mesmo cenário faria HTTPS real para a testnet. A verificação com testnet real fica **manual e opcional**.
- **R0b — GREEN determinístico.** Mesmo cenário via `ApiState`, com `FakeSpotOrderPorts` injetado no `HttpOrderExecutor`: o fake de submit é chamado **exatamente 1** vez, sem pânico, e a resposta é `accepted: true`; no poll, o fake de consulta é chamado 1 vez por key pendente, sem pânico. Antes da correção, R0b não compila (o seam não existe); esse é o red dele.
- **Se R0a não reproduzir** (ex.: algum passo antes do `block_on` falha com credencial falsa), muda só a severidade do achado (registrar o motivo na entrega). A fatia **continua antes de W0-12**, porque as duas mexem nos mesmos arquivos.
- **SEC-ORD-17** de [orders-g2-threat-model](../security/orders-g2-threat-model.md), determinístico, sem relógio: `#[tokio::test(flavor = "current_thread")]`, tudo dentro de `tokio::time::timeout`. O fake de submit usa o gate. Passos: (1) `tokio::spawn` do submit via `ApiState`; (2) o teste espera o aviso "entrou" do fake; (3) com o submit parado no gate, `GET /healthz` pelo router responde **200** e o handle do submit ainda não terminou (`!is_finished()`); (4) o teste libera o gate; (5) o submit termina sem pânico. Prova só que a cadeia até o port cede o worker (o que este SDD já admite); não prova nada sobre o ccxt real. Limite: um bloqueio síncrono de verdade travaria o único worker e o `timeout` não dispararia; nesse caso o teste trava em vez de falhar, e quem corta é o limite de tempo do job de CI.
- **A1.** `rg 'block_on' backend/src` sem ocorrência nos caminhos de `modules/orders` e `modules/exchanges`, fora de testes e de `main`, com **uma exceção explícita e única**: `backend/src/modules/orders/adapters/live_reconciliation_pg_mirror.rs:37` (`MIRROR_RUNTIME.block_on`). W0-13 nunca toca esse arquivo. A exceção **cai quando W0-11 for feito**: se W0-11 remover o ramo `supervisor.rs:84`, o mirror sai junto; se mantiver, W0-11 torna o mirror async como escopo obrigatório dele. Em ambos os casos, depois de W0-11 o A1 vale sem exceção.
- **A2.** O poll em background sobrevive a erro **e a pânico** num tick: (a) com o fake de consulta devolvendo `Err` no primeiro tick e `Found` no segundo, `spawn_order_reconciliation_poll` executa os dois ticks e a key sai de `pending`; (b) com o fake de consulta em `Panic` no primeiro tick e `Found` no segundo, a task continua viva, o primeiro tick gera log `error` e a key sai de `pending` no segundo. Os ticks são disparados pelo teste (intervalo de teste + `tokio::time::pause`/`advance`), sem espera real.
- **A3.** Suite atual de orders, monitor e HTTP verde; nenhum contrato HTTP muda.

## Dependências e arquivos compartilhados

- Compartilha arquivos com [W0-12](./wave0-12-order-ambiguous-claim-sdd.md): `binance_spot_testnet_submit.rs`, `execution_port.rs`, `models/error.rs`, `http_bridge/orders.rs`, `presentation/http/state.rs` (`submit_order_http`), `presentation/http/order_execution.rs`. W0-13 primeiro, W0-12 em cima, mesmo Builder ou sequência estrita.
- `state.rs` também é tocado por W0-01 (rotas/`require_http_admin`); coordenar ordem de merge.
- W0-11 decide o ramo `supervisor.rs:84` e, com ele, o mirror PG.
- Não depende de W0-01; G4 depende de W0-02.

## Riscos

- Assinatura async do port mexe em muitos testes (executores fake/recording). Mitigação: mudança mecânica, revisada pelo Critic.
- `HttpOrderExecutor` deixa de ser `Copy`; usos que copiam precisam de `clone()`.
- `LAST_SUBMIT_ACK` global (F-ORD-05) fica mais exposto com concorrência real; não é corrigido aqui (SEC-ORD-09 é outro item). O port devolve o ack direto, o que prepara W2-03.
- Comportamento do ccxt sob o runtime principal (timeouts, cancelamento) não é verificável estaticamente; o fake cobre o contrato, a testnet real fica como verificação manual opcional.
- Pânico genérico no poll deixa de derrubar a task (Decisão 4, A2), mas `catch_unwind` não pega `abort` nem pânico com `panic = "abort"`. Hoje o `Cargo.toml` não tem seção `[profile]` nem `panic = "abort"` (conferido com `rg`), então vale o padrão `unwind`; se alguém mudar o perfil, A2 (b) passa a falhar.

## Validação

- `backend/scripts/verify-backend-gates.sh` (os testes novos rodam no bin `bot`, sem rede e sem PG).
- Evidência de R0a (red) e R0b (red→green) registrada na entrega (AGENTS.md G3).

## Rollout / rollback

- Rollout: próximo build; só muda comportamento com `orders.exchange_submit=testnet`. Nenhum deploy autorizado.
- Rollback: reverter o commit volta ao `block_on` (o caminho testnet volta a entrar em pânico); sem migração nem dado.
