---
title: SDD W0-12 — resultado ambíguo de ordem não libera o claim
description: Fatia Onda 0 que impede ordem duplicada ou "aceita sem envio" quando a exchange pode ter aceitado a ordem mas a resposta falhou, ou quando o claim está ocupado (F-ORD-03, F-ORD-16 / SEC-ORD-06, 08, 20)
tags:
  - sdd
  - backend
  - orders
  - security
  - wave0
status: draft
---

# SDD W0-12 — resultado ambíguo de ordem não libera o claim [SEGURANÇA]

- **Estado:** draft, ciclo 3 do Builder depois do Critic G1 ciclo 2 (APROVADO COM FOLLOW-UP). Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre os seams antes do primeiro teste (seams **a acordar com o owner**).
- **Origem:** achados **F-ORD-03** (Alto) e **F-ORD-16** (Alto), abuso **AB-O6** e a combinação F-ORD-16 + F-ORD-11 (§7.1) em [orders-g2-threat-model](../security/orders-g2-threat-model.md); critérios **SEC-ORD-08**, **SEC-ORD-20** e a parte "PG obrigatório em `live_exchange`" de **SEC-ORD-06**. Linha 4 da tabela de prioridades de [master-plan](../planning/master-plan.md) §4.1, logo depois de [W0-13](./wave0-13-orders-block-on-sdd.md).
- **SDDs relacionados:** [orders-live-execution-gate2-sdd](./orders-live-execution-gate2-sdd.md), [orders-module-sdd](./orders-module-sdd.md). Este SDD é só o delta do claim. O port de exchange e o fake de teste são os definidos em [W0-13](./wave0-13-orders-block-on-sdd.md) (`SpotOrderSubmitPort`, `SpotOrderQueryPort`, `FakeSpotOrderPorts`).

## Contexto (evidência no código, HEAD `d42b71a5`)

Nenhum arquivo citado aqui mudou entre `b8370a75` e `d42b71a5`; as linhas foram conferidas de novo em `d42b71a5`. A cópia de trabalho tem uma mudança em stage (outra sessão, não commitada) em `modules/orders/adapters/pg_idempotency.rs`; as linhas desse arquivo abaixo são as do HEAD (`git show HEAD:…`).

- `backend/src/presentation/http/state.rs:512-630` (`submit_order_http`): com PG, faz `try_claim(key)` (`:545`) e, em **qualquer** `Err` devolvido por `http_bridge::orders::submit_order_http` (`:560-573`), chama `pg.release_claim(key)` (`:568-570`).
- `http_bridge/orders.rs:127-174`: chama o executor via `controllers::submit_order` (`:166`; `orders/controllers/submit.rs:6-20`, que valida, aplica risco e só então chama `executor.execute`) e, **depois** do sucesso, grava `record_completed` na memória (`:168`) e `mark_pending` no ledger (`:170`). O `mark_pending` devolve `InvalidRequest("client_order_id already tracked for reconciliation")` quando a key já está no ledger (`orders/adapters/in_memory_reconciliation.rs:96-99`); esse `Err` vem depois do `Ok` do executor e cai no `release_claim` de `state.rs:567-571`. Ou seja, hoje um erro posterior a uma ordem enviada libera o claim.
- A variante do erro não diz a posição: no adapter testnet, `map_bot_error` converte qualquer erro do ccxt em `OrdersError::InvalidRequest` (`binance_spot_testnet_submit.rs:35-37`), e `map_config_error` faz o mesmo para config (`:39-41`). Validação, rede antes do envio, timeout depois do envio e "already tracked" chegam todos como `InvalidRequest`.
- `modules/orders/adapters/pg_idempotency.rs:60-67`: `release_claim` apaga a linha de `order_idempotency_keys`. O retry com a mesma key executa de novo.
- `pg_idempotency.rs:23-32`: `is_completed` = "linha existe"; a linha nasce no claim (`try_claim`, `:46-57`), então uma key em voo parece concluída (`state.rs:541-543`) — parte de F-ORD-16.
- `state.rs:545-547`: quando `try_claim` perde a corrida, o código chama `record_completed` na memória e responde `accepted: true` sem ordem garantida; daí em diante esta instância responde "aceito" pela memória (`state.rs:537-538`) — F-ORD-16. Não depende de pânico: vale também em `paper`/`dev_accept` com PG.
- Combinação F-ORD-16 + F-ORD-11 (TM §7.1): no caminho testnet, o `block_on` entra em pânico depois do claim (W0-13); a linha fica, e todo retry recebe `accepted: true` sem ordem enviada. A reconciliação só é registrada depois de submit bem-sucedido (`state.rs:575-596`), então nada indica que a ordem não existe.
- Sem PG não há claim durável (`state.rs:536-551`; `http_bridge/orders.rs:144-148`): erro não grava nada e o retry reexecuta (AB-O4 / F-ORD-04).
- Migração `0004_order_idempotency_keys.sql`: só `client_order_id` e `recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`; sem estado. O `try_claim` (`pg_idempotency.rs:46-57`) grava a linha, então `recorded_at` já é a hora do claim. Hoje a última migração é `0011_monitor_supervisor_snapshot.sql`.
- **Só `LiveExchange` despacha para exchange:** `HttpOrderExecutor::execute` (`presentation/http/order_execution.rs:108-119`) só chama `ExchangeSpotExecutor` no modo `LiveExchange` (`:114`); `Disabled`, `DevAccept`, `Paper` e `LiveExchangeReserved` usam executores sem efeito externo. `HttpOrderExecutor` é um enum de modo `Copy` (`:8-25`), sem executor injetável no HEAD.
- **"Sem PG" neste SDD** = `order_idempotency_pg` é `None`, o que acontece com `DATABASE_URL` ausente **ou** com `DATABASE_URL` presente e conexão/migração falhando: `bootstrap_http_api` (`core/database/bundle.rs:51-71`) só registra `warn` e segue sem PG. Os dois casos são tratados igual.
- Teste existente `pg_submit_order_idempotency_releases_claim_when_submit_fails` (`state.rs:1885`) cobre rejeição por risco (antes do executor). Esse comportamento continua correto e deve continuar passando.
- Poll de reconciliação em background: só sobe com `live_exchange` e intervalo > 0 (`server.rs:30-31`); `reconciliation_poll_secs = 0` é o padrão (`core/config/system.toml:33`) e `order_reconciliation_poll_interval_secs` devolve `None` para 0 (`core/config/orders/file.rs:70-77`). Um pânico dentro do laço derruba a task para sempre (`server.rs:33-46`; ver W0-13).
- Não há timeout de requisição HTTP no router (`server.rs:81-94` só tem `TraceLayer`); o único limite de tempo no caminho testnet é o `timeout_secs(15)` do cliente ccxt (`exchanges/adapters/binance.rs:44`), por chamada (`load_markets`, `fetch_ticker` na venda, `create_order`).
- Impacto hoje: `live_exchange` testnet/recording, opt-in (padrão `orders.execution=""` → 503). Bloqueia G2 e qualquer mainnet.

## Contradição doc × código

- O comentário da tabela em `0004` diz "Completed client_order_id values", mas a linha é gravada no claim, antes da execução (`pg_idempotency.rs:46-57`). A migração `0012` corrige o comentário.

## Decisão

### D1 — regra de liberação pela posição no fluxo, nunca pela variante do erro

O fluxo de `ApiState::submit_order_http` passa a ter fases explícitas:

| Fase | O que acontece | Erro nesta fase |
|---|---|---|
| P1 claim | `try_claim` grava a linha com estado `in_flight` e `execution_mode`; `recorded_at` (já existente, `DEFAULT NOW()`) é a hora do claim | nada a liberar |
| P2 pré-executor | `request.validate()`, risco (`controllers/submit.rs:11-18`) e gate de política (`gate_order_submit`, hoje em `exchange_spot_executor.rs:12`, passa para antes do executor) | **libera** o claim (executor não foi chamado) |
| P3 intenção (só `live_exchange`) | ledger de reconciliação recebe `pending` para a key, primeiro no PG (`order_reconciliation`), depois na memória, **antes** de chamar o executor | se a key já estava no ledger: estado `unknown`, **409** `order_outcome_unknown` (uma tentativa anterior pode ter enviado); outro erro: remove o `pending` parcial, libera o claim e devolve o erro |
| P4 executor | `OrderExecutionPort::execute` (→ `SpotOrderSubmitPort` de W0-13); o future é envolvido em `catch_unwind` (`futures-util` já é dependência direta) | em `live_exchange`: `Err` **ou pânico** → estado `unknown`, **nunca** libera; `pending` fica no ledger |
| P5 pós-executor | primeira escrita: PG `mark_completed(key)`; depois memória `record_completed`; depois `confirm_exchange_order`, `upsert_state`, outbox | **nunca** libera em `live_exchange` (ver D4); exceção única: TX de P5 em `paper` com PG (D4) |

- O controller devolve a fase em que parou (`PreExecutor` ou `Executor`), marcada pelo ponto do código onde o erro surgiu, e não inferida do tipo do erro. `state.rs` decide só por essa marca e por "o executor já devolveu `Ok`?".
- **Justificativa:** a variante não carrega a posição (`map_bot_error` colapsa tudo em `InvalidRequest`; o "already tracked" do `mark_pending` também é `InvalidRequest` e vem depois do envio). Errar para o lado de travar a key é seguro (exige reconciliação ou ação manual); errar para o lado de liberar duplica ordem. Por isso, uma vez chamado o executor em `live_exchange`, não se libera mais.
- **Custo aceito:** falhas de rede antes do `create_order` dentro do port (ex.: `load_markets`) também viram `unknown`. Credenciais, registro e cliente ccxt já saem do caminho da requisição e vão para o boot (W0-13), o que reduz esse caso. Se a taxa de `unknown` incomodar, a alternativa é dividir o port em `prepare`/`submit`; fica registrado, não entra agora.
- **Modos não-live** (`paper`, `dev_accept`, `disabled`, `live_exchange_reserved`): não há efeito externo nem reconciliação que resolva `unknown`. Com PG, os estados `in_flight`/`completed` e os 409 valem igual (F-ORD-16 também afeta esses modos), mas `Err` ou pânico no executor **libera** o claim, como hoje. **Por que liberar é seguro:** só `LiveExchange` despacha para `ExchangeSpotExecutor` (`order_execution.rs:108-119`, `:114`); nos outros modos nada sai do processo. Para o bot Segurança: o texto de SEC-ORD-20 (b) no TM é escrito sem restringir modo e precisa ser ajustado para "em `live_exchange`"; este SDD aplica o 409 `order_outcome_unknown` só ao `live_exchange`.

### D2 — estados da key e resposta a nova requisição com a mesma key

| Estado | Significado | Resposta | Executor chamado? |
|---|---|---|---|
| `in_flight` | claim ocupado (o Critic chama de `claimed`; o TM, de `in_flight`) | **409** `idempotency_in_flight` | não |
| `unknown` | executor chamado, resultado não confirmado | **409** `order_outcome_unknown` | não |
| `completed` | executor devolveu `Ok` (ou reconciliação achou a ordem) | replay `accepted: true` | não |
| (sem linha) | nunca usada ou liberada antes do executor | executa | sim |

- `try_claim` perdido → lê o estado e responde pela tabela; **nunca** grava em `InMemoryOrderIdempotencyStore` (remove o `record_completed` de `state.rs:546`) — SEC-ORD-20 (d).
- `record_completed` na memória só depois de `completed` no PG. **Onde a ordem muda:** o `record_completed` sai de `http_bridge/orders.rs:168` (hoje antes de qualquer escrita PG) e vai para `state.rs`, logo depois do `mark_completed` PG de P5; o `mark_pending` sai de `http_bridge/orders.rs:169-171` (depois do executor) e vai para P3 (antes). Sem PG e fora de `live_exchange`, `http_bridge` continua gravando a memória depois do sucesso, como hoje.
- A requisição original que termina em `unknown` recebe **502** `order_outcome_unknown`; os retries recebem **409** `order_outcome_unknown`. Corpo sem texto da exchange (SEC-ORD-15).

### D3 — claim órfão (pânico fora do executor, crash, processo morto)

- A hora do claim é o `recorded_at` que o `try_claim` já grava (`0004`, `DEFAULT NOW()`); não há coluna nova para isso. No boot do `serve` (depois da hidratação PG em `build_api_state_for_http_serve`, `state.rs:230-304`) e no início de cada poll de reconciliação (`reconcile_pending_orders_once`, `state.rs:724`, **antes** do retorno antecipado de `:734-736`), um varredor procura linhas `in_flight` com `recorded_at` mais velho que **T**:
  - `execution_mode = live_exchange` → estado `unknown` e `pending` no ledger (PG e memória), se ainda não houver;
  - outros modos → libera (apaga a linha): não há efeito externo e o estado em memória morreu com o processo.
- **T:** configurável em `core/config`, padrão 300 s, validado no boot como maior que 3 × o timeout do cliente ccxt (15 s, três chamadas em série no pior caso). Como não há timeout HTTP, uma requisição pode passar de T; por isso `mark_completed` é condicional (`UPDATE … WHERE state IN ('in_flight','unknown')`) e uma conclusão tardia continua válida. O varredor nunca libera linha `live_exchange`, então não há risco de duplicar.
- **Conclusão tardia não-live com a linha já apagada** pelo varredor (o `UPDATE` condicional afeta 0 linhas): o `state.rs` grava `completed` com `INSERT … ON CONFLICT DO NOTHING`. Se houver conflito (um retry já fez claim novo da key), a resposta original sai normalmente e fica um `warn` `idempotency_late_completion_conflict`; o retry segue pelo próprio claim. Pode haver execução paper/dev duplicada em memória, sem efeito externo (só `LiveExchange` despacha). Com [W0-05](./wave0-05-promocoes-portfolio-persistidos-sdd.md), o fill paper persistido tem chave `client_order_id`, então o segundo fill falha na TX de P5 e cai na exceção de D4 (503, claim liberado).
- **Quando o varredor roda:** o poll em background só sobe com `live_exchange` e intervalo > 0 (`server.rs:30-31`). Com o padrão `reconciliation_poll_secs = 0` (`system.toml:33`), ou fora de `live_exchange`, o varredor roda **só no boot e no `POST /orders/reconciliation/poll` manual**. Nesse caso uma linha `in_flight` órfã fica `in_flight` (retry → 409 `idempotency_in_flight`) até o próximo boot ou poll manual: trava a key, mas não duplica ordem. O runbook registra isso.
- **Supervisão da task do poll (dependência explícita de W0-13):** como o varredor roda no começo de cada tick, um pânico que derrube a task pararia o varredor em silêncio. A função `spawn_order_reconciliation_poll` extraída por W0-13 envolve **cada tick** (varredor + poll) em `catch_unwind` (`futures_util::FutureExt::catch_unwind` com `AssertUnwindSafe`), registra `error` e segue para o próximo tick. Critério em W0-13 A2.

### D4 — falha depois do `Ok` do executor

- Quem grava: `state.rs` grava `completed` em P5 (primeira escrita depois do `Ok`) e `unknown` em P3/P4; o varredor (D3) grava `unknown` para órfãos; a reconciliação (D5) grava `completed`.
- Falha em `confirm_exchange_order`, `upsert_state` ou outbox depois de `completed`: a requisição original devolve o erro que já devolve hoje (ex.: **503** `order_store_unavailable`); o retry recebe replay `accepted: true` sem chamar o executor, porque a ordem existe.
- Falha no próprio `mark_completed`: a linha fica `in_flight`; retry → **409** `idempotency_in_flight`; depois de T, o varredor a move para `unknown` (**409** `order_outcome_unknown`); a reconciliação encontra a ordem e grava `completed`.
- **Exceção de P5 em `paper` com PG (origem: [W0-05](./wave0-05-promocoes-portfolio-persistidos-sdd.md) D4, que usa as fases P1–P5 deste SDD):** em `paper` com PG, o `mark_completed(key)` e o `INSERT` do fill paper vão na mesma TX. Se essa TX falhar, o claim é **liberado** e a resposta é **503**. É seguro porque nada foi despachado para exchange: só `LiveExchange` despacha (`order_execution.rs:114`). É a única exceção ao "P5 nunca libera", que existe por causa de efeito externo em `live_exchange`. Este SDD não edita o W0-05.

### D5 — saída de `unknown`

- Só por reconciliação contra a exchange (`SpotOrderQueryPort` de W0-13; hoje `observe_testnet_spot_order_by_client_id`): ordem encontrada → `completed` + `reconciled`. Ordem encontrada **cancelada** (`Divergent`, hoje `binance_spot_testnet_reconcile.rs:36-38` → `reconciliation_poll.rs:23-25`) → idempotência `completed` + ledger `divergent`: a key foi consumida por uma ordem real, então uma nova tentativa precisa de key nova; o replay responde `accepted: true` (o mesmo significado de hoje, "aceita pelo gate de risco", `routes/orders.rs:87`) e o estado final é lido em `GET /orders/reconciliation/{client_order_id}`. Não encontrada → continua `unknown` nesta fatia (sem liberação automática). Liberação manual fica como decisão do owner com o bot Segurança (pergunta abaixo); não bloqueia G1.

### D6 — política sem PG

- `live_exchange` sem PG **recusa o boot** do `serve` (SEC-ORD-06, parte "PG obrigatório em `live_exchange`"), nos dois sentidos de "sem PG" do Contexto (URL ausente ou PG inacessível/migração falhando). A checagem fica em `server::run` logo depois de `bootstrap_runtime` (`server.rs:21`) e antes do `bind` (`:62`), porque depende do resultado da conexão. Decidido; sem pergunta pendente. O claim atômico em memória do SEC-ORD-06 fica fora desta fatia.

### D7 — schema

- Migração **`0012`, reservada para W0-12** (já registrada assim em [master-plan](../planning/master-plan.md) §4.1 e respeitada por [W0-05](./wave0-05-promocoes-portfolio-persistidos-sdd.md), que usa a seguinte). A confirmar no master-plan no momento da implementação; este SDD não edita o plano.
- `order_idempotency_keys` ganha `state` (`CHECK` em `in_flight | completed | unknown`, `NOT NULL`) e `execution_mode TEXT` (nulo nas linhas legadas). Não ganha `claimed_at`: o `recorded_at` existente já é a hora do claim (D3). O comentário da tabela passa a descrever claim + estado.
- **Linhas existentes (legado):** linha cuja key tem `state = 'pending'` em `order_reconciliation` migra como **`unknown`** (a ordem pode existir e ainda não foi confirmada); as demais migram como `completed`. Assim uma key legada com envio não confirmado não responde `accepted: true` para sempre.
- **Runbook antes de migrar:** rodar `SELECT k.client_order_id, k.recorded_at, r.state FROM order_idempotency_keys k LEFT JOIN order_reconciliation r USING (client_order_id) WHERE r.client_order_id IS NULL OR r.state = 'pending' ORDER BY k.recorded_at;`. As linhas com `pending` viram `unknown` pela migração. As linhas **sem** reconciliação (ex.: claims órfãos do pânico testnet F-ORD-16 + F-ORD-11, que nunca chegaram a gravar reconciliação) viram `completed`; o operador as revisa antes e decide à mão (apagar para permitir retry, ou manter). Só existem se alguém rodou `live_exchange` testnet com PG.

**Alternativa descartada:** sem migração, derivar "unknown" de `order_reconciliation` e só remover o `release_claim` pós-envio. Mantém `accepted: true` no replay e falha SEC-ORD-08 e SEC-ORD-20. Descartada; não há plano B.

## Seams públicos (a acordar com o owner antes do TDD)

| Seam | Proposta |
|---|---|
| Port de exchange e fake | `SpotOrderSubmitPort`/`SpotOrderQueryPort` e `FakeSpotOrderPorts` de [W0-13](./wave0-13-orders-block-on-sdd.md), com os roteiros `Ack`, `ErrBeforeSend`, `AcceptThenTimeout`, `Panic`, `Found`, `NotFound` |
| Fase do erro | resultado do controller marcado `PreExecutor` \| `Executor` pela posição no código |
| HTTP | **409** `idempotency_in_flight`; **409** `order_outcome_unknown` (retry); **502** `order_outcome_unknown` (requisição original); corpo sem detalhe da exchange (SEC-ORD-15) |
| Store | `PgOrderIdempotencyStore`: `try_claim(key, mode)`, `state(key)`, `mark_completed(key)` (condicional), `mark_unknown(key)`, `sweep_stale_in_flight(T)` (por `recorded_at`); `release_claim` só em P2/P3, fora de `live_exchange`, ou na TX de P5 paper que falhou (D4) |
| Config | T (nome proposto `BOT_ORDERS_CLAIM_STALE_SECS`, lido em `core/config`), padrão 300 s |
| Schema | migração `0012` (D7) |
| Boot | `live_exchange` sem PG (URL ausente ou PG inacessível) → erro de boot em `server::run`, antes do `bind` |
| Supervisão do poll | `catch_unwind` por tick em `spawn_order_reconciliation_poll` (W0-13 A2) |

## Critérios de aceite

- **SEC-ORD-08:** com `FakeSpotOrderPorts` em `AcceptThenTimeout`, a primeira requisição recebe 502 `order_outcome_unknown`; N retries com a mesma key recebem **409** `order_outcome_unknown`; a fake registra **exatamente 1** ordem. Depois, poll com `Found` → key `completed`; novo retry → replay `accepted: true`, ainda 1 ordem.
- **SEC-ORD-20 / F-ORD-16 (RED primeiro).**
  - **RED no HEAD para (a) e (d), sem executor injetável** (no HEAD `HttpOrderExecutor` é um enum `Copy`, `order_execution.rs:8-25`): teste PG que pré-grava o claim com `pg.try_claim(key)` e depois chama `submit_order_http` com a mesma key. O HEAD responde `accepted: true` de forma determinística (`state.rs:545-547`) e grava a key em `InMemoryOrderIdempotencyStore` (`:546`); o teste espera 409 `idempotency_in_flight` e memória vazia, então falha.
  - (a) GREEN: com um submit em voo, o segundo submit com a mesma key recebe **409** `idempotency_in_flight`, sem `accepted`, executor chamado 1 vez. O "em voo" é um **gate** no fake (`tokio::sync::Notify` ou `Barrier`): o fake para no gate, o teste faz o segundo submit e só depois libera o gate. Sem atraso por relógio.
  - (b) Em `live_exchange`, `Panic` no fake depois do claim → retry → **409** `order_outcome_unknown`, nunca `accepted: true`.
  - (c) N retries concorrentes → a fake registra exatamente 1 ordem e `accepted: true` só aparece depois que ela existe.
  - (d) Nenhum caminho grava a key em `InMemoryOrderIdempotencyStore` quando o PG respondeu claim ocupado.
- **Combinação F-ORD-16 + F-ORD-11 (TM §7.1), teste PG com RED no HEAD.** `live_exchange` + testnet com credenciais falsas, como no R0a de W0-13 (o pânico vem antes de qualquer rede): primeira requisição entra em pânico depois do claim; o retry com a mesma key recebe hoje `accepted: true` sem ordem → o teste falha. Depois de W0-13 + W0-12, o mesmo cenário roda com o fake em `Panic`: o retry recebe **409** `order_outcome_unknown`, a key está `unknown` com `pending` no ledger e a fake registra 0 ordens.
- **A0.** Rejeição por risco com a key em claim libera a linha; o retry com limites válidos executa **exatamente 1** vez.
- **A1.** Rejeição por risco ou modo desligado continua liberando o claim (`pg_submit_order_idempotency_releases_claim_when_submit_fails` segue verde).
- **A2.** Falha injetada em `upsert_state` depois do `Ok` do executor: a requisição original recebe o erro atual (503), a key está `completed`, o retry recebe replay `accepted: true` e a fake continua com 1 ordem. Falha injetada em `mark_completed`: retry → 409 `idempotency_in_flight`; varredor com T de teste → 409 `order_outcome_unknown`; poll `Found` → `completed`.
- **A3.** "Already tracked" em P3 → `unknown` + 409 `order_outcome_unknown`, executor não chamado.
- **A4.** Varredor (idade por `recorded_at`): linha `in_flight` antiga com `execution_mode = live_exchange` → `unknown` + `pending`; linha antiga `paper` → apagada; linha recente → intacta. Roda no boot e no `POST /orders/reconciliation/poll`, inclusive fora de `live_exchange` e com `reconciliation_poll_secs = 0`.
- **A4b.** Conclusão tardia não-live com a linha apagada: `INSERT … ON CONFLICT DO NOTHING` grava `completed`; com conflito, resposta normal e `warn` `idempotency_late_completion_conflict`.
- **A4c.** Reconciliação `Divergent` (fake `Found` cancelado) → key `completed`, ledger `divergent`; retry → replay `accepted: true`, fake com 1 ordem.
- **A4d.** Paper com PG: falha injetada na TX de P5 (`mark_completed` + fill) → **503**, claim liberado; retry com a mesma key executa uma vez (mesmo cenário do A5 de W0-05).
- **A5.** Reconciliação que encontra a ordem move a key para `completed` e o ledger para `reconciled`; teste PG isolado.
- **A6.** Migração `0012`: reaplicação idempotente; linha antiga com `pending` em `order_reconciliation` vira `unknown`; as demais viram `completed`; `recorded_at` preservado; teste de migração no manifesto PG.
- **A7.** Boot com `live_exchange` falha com mensagem estável, antes do `bind`, nos dois casos: `DATABASE_URL` ausente e `DATABASE_URL` apontando para porta sem PG.
- **A8.** Log do caso ambíguo sem segredo nem texto bruto da exchange.

## Dependências

- W0-01 recomendado antes (submit sem auth amplia o abuso), mas não bloqueia o desenho.
- [W0-05](./wave0-05-promocoes-portfolio-persistidos-sdd.md) usa as fases P1–P5 e a exceção de P5 paper (D4); entra depois desta.
- [W0-13](./wave0-13-orders-block-on-sdd.md) antes, em sequência estrita: define o port/fake, remove o `block_on` onde esta fatia marca as fases e extrai `spawn_order_reconciliation_poll` com `catch_unwind` por tick (W0-13 A2), de que o varredor depende. Arquivos compartilhados: `binance_spot_testnet_submit.rs`, `execution_port.rs`, `models/error.rs`, `http_bridge/orders.rs`, `state.rs`, `order_execution.rs`.
- W0-02: os testes PG novos só valem como evidência quando falham sem PG.
- Relaciona-se com SEC-ORD-06 (claim atômico em memória), 07 e 09 (F-ORD-04/05); ficam fora desta fatia, salvo decisão do owner.

## Riscos

- Keys `unknown` sem ordem na exchange (falha antes do `create_order` dentro do port) ficam travadas até ação manual; precisa de runbook.
- `fetch_order` por `client_order_id` no ccxt não é verificável estaticamente (o adapter Binance vem de `ccxt-exchanges`, que não é vendorizado; só `ccxt-core` está em `backend/vendor/ccxt-core-0.1.5`, `Cargo.toml:50`); se não consultar por `origClientOrderId`, a key fica `unknown` para sempre. Teste com o fake e verificação manual em testnet.
- O ack global `LAST_SUBMIT_ACK` (F-ORD-05) continua; W0-13 prepara a devolução do ack pelo port.
- Monitor (`supervisor.rs:221-245`) chama `submit_order` direto, sem claim PG; o ramo testnet do monitor é inalcançável hoje (W0-11).

## Validação

- `backend/scripts/verify-backend-gates.sh`; os testes PG novos entram em `run-pg-integration-tests.sh` e no manifesto verificado por `assert-pg-integration-manifest.sh` (W0-02).
- Testes unitários com o fake de W0-13 (sem rede) para os 409, o pânico e "exatamente 1 chamada".

## Rollout / rollback

- Rollout: próximo build; muda comportamento com PG em todos os modos (409 em vez de `accepted` para claim ocupado) e, em `live_exchange`, o tratamento de erro/pânico. Nenhum deploy autorizado.
- Rollback: reverter o código volta a liberar o claim (reabre F-ORD-03/16). As colunas novas podem ficar (código antigo as ignora; `state` tem default). Remover exige outra migração. Keys em `unknown` devem ser revisadas antes de qualquer rollback: o código antigo trata qualquer linha como concluída e responderia `accepted: true` para elas.
