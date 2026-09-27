---
title: Threat model — orders Gate 2 (execução exchange testnet/paper)
description: Modelo de ameaças proporcional de orders G2 com achados, mitigações e critérios de aceite testáveis SEC-ORD
tags:
  - security
  - threat-model
  - backend
  - orders
  - exchanges
status: draft
---

# Threat model — orders Gate 2

- **Estado:** proposto — aguardando revisão do Critic independente (ciclo 2 de ENTREGA SEC-TM; follow-ups O1-O3 aplicados, ver §11). Não fecha o item "Threat model + revisão Critic" do checklist G2 e não autoriza prod REST nem trading live.
- **Tipo:** documentação apenas. Complementa (não substitui nem edita) a seção "Threat model" de [`sdd/orders-live-execution-gate2-sdd.md`](../sdd/orders-live-execution-gate2-sdd.md).
- **Autor (Builder):** agente executor de segurança/Blue Team. **Critic:** pendente de atribuição.
- **Data da revisão:** 2026-09-27. **Método:** leitura estática local; nenhuma chamada a exchange, testnet ou sistema vivo.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01, causa raiz de AB-O2), [provider-credentials-plaintext](./provider-credentials-plaintext.md), [org-module-threat-model](./org-module-threat-model.md), [owner-auth-idp-threat-model](./owner-auth-idp-threat-model.md) (P1).

## 0. Entradas revisadas

| Entrada | Estado |
|---|---|
| [`sdd/orders-live-execution-gate2-sdd.md`](../sdd/orders-live-execution-gate2-sdd.md) | `status: draft`, estado "parcial"; igual a HEAD `2d1e3863` no momento da revisão |
| [`sdd/orders-module-sdd.md`](../sdd/orders-module-sdd.md) | `status: draft`; com alterações não commitadas no working tree (outra sessão) |
| [`sdd/http-admin-auth-seam-sdd.md`](../sdd/http-admin-auth-seam-sdd.md) | `status: partial` |
| Código (HEAD `2d1e3863`; arquivos citados inalterados desde `0880cdd5`; ciclo 2: `state.rs`, `pg_idempotency.rs` e os adapters Binance reconferidos por sha256 em HEAD `bb0d320f`, idênticos) | `presentation/http/{routes/orders.rs,state.rs (submit_order_http 512-630),order_execution.rs,error.rs,admin_auth.rs}`; `modules/http_bridge/orders.rs`; `modules/orders/{controllers/submit.rs,models/request.rs,adapters/{idempotency.rs,pg_idempotency.rs,spot_order_submit.rs,exchange_spot_executor.rs,exchange_order_gate.rs,spot_order_reconciliation_query.rs},threat_model_invariants.rs}`; `modules/exchanges/{rest.rs,adapters/{binance.rs,binance_spot_testnet_submit.rs,binance_spot_testnet_reconcile.rs,account_file.rs}}`; `core/config/{exchanges/credentials.rs,orders/file.rs,mod.rs}`; migrations `0004`, `0006` |

## 1. Contexto resumido (estado real do código)

- `POST /api/v1/orders/submit` → `require_http_admin` (bearer **opcional**) → `ApiState::submit_order_http` → dedupe (memória e, se houver PG, `try_claim`) → `risk::validate_intent` → `HttpOrderExecutor` (`disabled` padrão; `dev_accept`; `paper`; `live_exchange` = `recording` in-process ou `testnet` Binance Spot via ccxt com `BINANCE_TESTNET_*`).
- Prod REST bloqueado por `authorize_rest_use` (só `Environment::Dev` + Binance Spot) e pelo builder `build_dev_spot_binance` (`sandbox(true)`, endpoint exato `testnet.binance.vision`, `account_file.rs` com allowlist rígida) — controles efetivos e bem testados.
- Reconciliação: ledger memória + PG `order_reconciliation`; `POST /orders/reconciliation/poll` (bearer opcional) e `GET /orders/reconciliation/{client_order_id}` (**sem** bearer).

## 2. Ativos

| ID | Ativo |
|---|---|
| O1 | Fundos/saldos na exchange (hoje testnet; futuro mainnet) e posições. |
| O2 | Chaves de exchange (`BINANCE_TESTNET_*`; loader `BINANCE_PROD_*` existe em `credentials_for_environment`). |
| O3 | Limites de risco (`max_order_quote`, `max_daily_loss_quote`, `max_open_positions`) e exposição corrente. |
| O4 | Integridade de idempotência (`order_idempotency_keys`) e reconciliação (`order_reconciliation`). |
| O5 | Ledger paper/portfolio. |
| O6 | Trilhas de auditoria e projeção Neo4j de `OrderIntent`. |
| O7 | Disponibilidade do submit/poll (workers Tokio). |

## 3. Atores

Operador com token admin (seam); cliente HTTP sem credencial (quando token vazio ou em rotas sem bearer); futuro principal/agente autorizado via `org` (pós-P1); exchange/testnet (terceiro, confiável só via TLS + allowlist de host); atacante de rede entre bot e exchange; operador de infra (env, PG).

## 4. Fronteiras de confiança

```text
[Cliente/agente] --(TB1 HTTP: bearer opcional hoje)--> [routes/orders.rs → ApiState]
   --(TB2)--> [risk::validate_intent (limites vindos do corpo!)] --(TB3)--> [OrderExecutionPort]
   --(TB4 TLS, host allowlist testnet)--> [Binance Spot testnet via ccxt]
[ApiState] --(TB5 sqlx)--> [PG: order_idempotency_keys, order_reconciliation, outbox] --(TB6)--> [Neo4j]
[Operador] --(TB7 env)--> [BOT_ORDERS_EXECUTION, BOT_ORDERS_EXCHANGE_SUBMIT, BINANCE_*]
```

## 5. Casos de abuso

| ID | Caso | Evidência |
|---|---|---|
| AB-O1 | **Adulteração de limites de risco:** cliente envia `limits.max_order_quote` enorme, `estimated_daily_loss=0`, `open_positions=0`; o gate de risco aprova qualquer valor. | `http_bridge/orders.rs:18-32` (DTO com `limits`, `estimated_daily_loss`, `open_positions`), `:149` (`limits_body.into()`); `controllers/submit.rs` usa esses valores. O SDD G2 (tabela de riscos, linha "Bypass de risco") registra gap "—". |
| AB-O2 | **Ordem não autorizada:** com `BOT_HTTP_ADMIN_TOKEN` vazio (padrão de `system.toml` `[http.admin]`), qualquer cliente que alcance a porta envia ordens (paper/testnet/recording); com token, qualquer portador — sem principal, org, conta ou bot. | `admin_auth.rs:91-94`; `routes/orders.rs:94-105`; ver [F-ADM-01](./admin-http-auth-fail-open.md). |
| AB-O3 | **Leitura cross-tenant/anônima** do estado de reconciliação (`exchange_order_id`, `divergent_reason`) por `client_order_id` adivinhável. | `routes/orders.rs:39-58` (sem `require_http_admin`); PK global `client_order_id` em `0006`. |
| AB-O4 | **Replay/duplicidade por corrida** sem PG: dois submits simultâneos com a mesma key passam por `is_completed` antes de qualquer `record_completed`. | `state.rs:537`; `http_bridge/orders.rs:144-148, 166-171`; `idempotency.rs` (check e insert separados). |
| AB-O5 | **Reuso de key com payload diferente** retorna `accepted: true` sem executar — cliente acredita que a nova ordem foi aceita. | `state.rs:536-548`; tabela `0004` não guarda hash do payload. |
| AB-O6 | **Erro depois do envio → ordem dupla (gasto duplo):** a exchange aceita a ordem, mas `create_order` devolve erro (timeout, conexão caída, resposta ilegível), ou um passo local depois do envio falha; o erro sobe e `release_claim` apaga o claim; o retry do cliente, outra instância ou um restart envia a ordem de novo. Ordem a mercado já executada não é deduplicada pela exchange. | `binance_spot_testnet_submit.rs:91-100, 144-153, 167-173`; `http_bridge/orders.rs:166-170`; `state.rs:560-573`; `pg_idempotency.rs:60-61`. |
| AB-O7 | **Troca de acks entre requisições concorrentes:** ack guardado em `static LAST_SUBMIT_ACK` e lido depois por `take_last_spot_submit_ack()` → `client_order_id` A ligado ao `exchange_order_id` de B. | `spot_order_submit.rs:9, 33-39, 81`; `state.rs:575-587`. |
| AB-O8 | **Garantia ilusória de ambiente:** o executor real sempre passa uma conta Dev fixa ao gate; os testes "prod negado" exercitam uma conta Prod construída no teste, não o caminho real. A proteção efetiva é o builder testnet. | `exchange_spot_executor.rs:10-14`; `exchange_order_gate.rs:18-26`; `threat_model_invariants.rs:31-55`; `binance.rs:21-45`. |
| AB-O9 | **Dimensionamento incorreto:** compra a mercado envia `amount = 1` (base) + param `cost`; se o adapter ccxt ignorar `cost`, compra 1 unidade base (ex.: 1 BTC). | `binance_spot_testnet_submit.rs:76-101` (crate `ccxt_exchanges` não está vendorizado — não verificável estaticamente). |
| AB-O10 | **Reconciliação confia no status da exchange** sem comparar símbolo/lado/quantidade/custo; erro de fetch vira "pending" silencioso. | `binance_spot_testnet_reconcile.rs:32-43, 46-67`. |
| AB-O11 | **Sem kill switch/rate limit:** parar envio exige mudar env e reiniciar; sem limite de taxa/notional por janela. | `order_execution.rs` (modo fixado no boot); nenhuma checagem por request. |
| AB-O12 | **DoS/pânico:** `Runtime::block_on` de runtime current-thread chamado dentro de handler async. | `binance_spot_testnet_submit.rs:20-28, 166-167`; `binance_spot_testnet_reconcile.rs:64`. |
| AB-O17 | **Claim órfão sem recuperação:** o processo morre (crash, OOM, deploy, `kill -9`) ou o handler entra em pânico entre o claim e o fim do fluxo; a linha fica em `order_idempotency_keys` para sempre, sem estado, prazo nem varredura; todo retry recebe `accepted: true` e nenhuma ordem é enviada, ou, se a ordem chegou a sair, ninguém reconcilia. | `pg_idempotency.rs:46-57`; `0004:2-5`; `state.rs:541-543, 566-571`. |
| AB-O18 | **Reconciliação morta em silêncio:** a task de poll em background entra em pânico (hoje, pelo `block_on` de F-ORD-11, no primeiro tick com credenciais testnet) e termina; ninguém observa o `JoinHandle`, não há reinício, métrica nem log de saída; ordens `pending`/`unknown` nunca são resolvidas e divergências com a exchange não aparecem. | `server.rs:30-45`; `state.rs:724-771`; `binance_spot_testnet_reconcile.rs:58-64`. |
| AB-O16 | **Falso "aceito" com claim ocupado:** a tabela de idempotência não distingue "em execução" de "concluída". Uma requisição concorrente, ou um retry depois de falha que não liberou o claim, recebe `accepted: true` sem que ordem alguma tenha sido enviada; a instância ainda grava a key como concluída em memória. | `state.rs:540-547`; `pg_idempotency.rs:23-31, 46-57`; `0004:2-5` (sem coluna de estado). |
| AB-O13 | **Vazamento em erros:** texto bruto de exchange/driver volta ao cliente; redação só por substituição exata dos valores testnet (não cobre prod, assinatura HMAC, valores parciais/codificados). | `binance_spot_testnet_submit.rs:35-41`; `core/config/exchanges/credentials.rs:27-37`; `error.rs:195-214`. |
| AB-O14 | **Adulteração de P&L paper** via `paper_fill_unit_price` do corpo. | `http_bridge/orders.rs:29-31`. |
| AB-O15 | **Spoofing de resposta da exchange** (MITM/DNS). | Mitigado: HTTPS + host exato (`account_file.rs:111-120` e testes `dev_spot_rejects_urls_outside_exact_testnet_allowlist`); manter e não permitir override. |

## 6. Mitigações e critérios de aceite testáveis (SEC-ORD)

**Coluna "Dep." (O2):** **agora** = implementável no código atual; **pós-P1** = precisa de owner/principal autenticado; **pós-org** = precisa de capability/epoch do `org`. Critérios com dependência trazem a **variante pré-P1** que vale até lá.

| ID | Dep. | Mitigação | Critério de aceite testável |
|---|---|---|---|
| **SEC-ORD-01** | agora | Limites de risco somente server-side (config/policy por conta/bot/org); DTO sem `limits` (`deny_unknown_fields`). | Body com `limits` → **400** `unknown_field`; com limite server-side 10 e `quote_amount` 50 → **422** `risk_rejected` e `RecordingExecutor::call_count() == 0`, independentemente do que o corpo declarar. |
| **SEC-ORD-02** | agora | Exposição (`estimated_daily_loss`, `open_positions`) calculada do ledger/portfolio/reconciliação, não do cliente. | Ledger com `open_positions == max` e corpo declarando 0 → **422**; campo no corpo → **400**. |
| **SEC-ORD-03** | agora (pós-org: capability) | Autenticação obrigatória para qualquer modo que execute: startup falha se `BOT_ORDERS_EXECUTION ∈ {dev_accept, paper, live_exchange}` sem `BOT_HTTP_ADMIN_TOKEN` (ou bind não-loopback sem auth). Pós-P1/`org`: capability `orders.submit` do principal para a conta/org, com epoch verificado (SEC-ORG-11). | Teste de boot com execução `paper` e token vazio → erro de configuração; submit sem bearer → **401**; pós-org: principal sem capability → **403** e audit deny. |
| **SEC-ORD-04** | agora (pós-org: escopo) | `GET /orders/reconciliation/{id}` autenticado e escopado por tenant. | Sem bearer → **401**; pedido de ordem de outra org → **404** idêntico a inexistente. |
| **SEC-ORD-05** | pós-org; pré-P1: `org_id`/`account_id` fixos de config | Escopo por tenant/conta nas tabelas: `PRIMARY KEY (org_id, account_id, client_order_id)` em idempotência e reconciliação. | Mesma `client_order_id` em duas orgs → duas ordens independentes; replay de A nunca responde por B. |
| **SEC-ORD-06** | agora | Claim atômico também em memória (insert-if-absent antes de executar; estado `in_flight`); PG obrigatório em `live_exchange`. | 50 submits concorrentes com a mesma key (executor fake com latência) → `call_count() == 1`; demais recebem replay ou **409** `idempotency_in_flight`; boot `live_exchange` sem PG → erro. |
| **SEC-ORD-07** | agora | Key amarrada ao hash canônico do payload. | Mesma key com `quote_amount` diferente → **409** `idempotency_key_reuse`, executor não chamado. |
| **SEC-ORD-08** | agora | Depois de **qualquer tentativa de envio** à exchange, o claim **nunca** é liberado: erro ambíguo pós-envio (timeout, conexão caída, resposta ilegível) ou erro local depois do envio leva a key para `unknown`/`reconciling`, e só a reconciliação decide entre `completed` e `failed`. `release_claim` só é permitido em falha **pré-dispatch** comprovada (validação, risco, credencial ausente, montagem do cliente), marcada por tipo de erro distinto. A memória nunca grava "concluído" antes do fim do fluxo. | (a) Fake exchange que aceita e depois devolve timeout: após N retries do cliente com a mesma key, em outra instância e depois de restart, a fake registra **exatamente 1** ordem; retry durante `unknown` → **409** `order_outcome_unknown`. (b) Erro local injetado depois do envio (ex.: `mark_pending` falhando) → key em `unknown`, claim presente no PG, retry → 409, fake com exatamente 1 ordem. (c) Falha pré-dispatch (credencial ausente) → claim liberado e retry permitido. |
| **SEC-ORD-09** | agora | Ack retornado pelo port (`execute` → `OrderAck`), sem estado global. | 100 submits concorrentes (backend recording) → mapeamento `client_order_id ↔ exchange_order_id` é bijetivo e cada ack corresponde à própria requisição. |
| **SEC-ORD-10** | agora | Gate de ambiente usa a conta efetivamente resolvida da config/registro (não conta Dev fixa); teste no caminho real do executor. | Registro com conta Spot `Prod` + seam `recording` → submit via `ExchangeSpotExecutor` retorna **503** `live_exchange_not_wired`, sem ack; teste de rede fake prova que nenhum host mainnet é contatado. |
| **SEC-ORD-11** | agora | Dimensionamento explícito (`quoteOrderQty` para compra por quote; quantidade base calculada com `Decimal` e precisão de mercado) + teto de notional server-side por ordem e por janela. | Fake HTTP captura a requisição de compra: contém `quoteOrderQty == quote_amount` e **não** contém `quantity=1`; ordem acima do teto → **422** antes do adapter. |
| **SEC-ORD-12** | pós-P1 (toggle); pré-P1: flag PG alternada só por CLI local de operador, auditada | Kill switch persistente (flag PG verificada a cada submit, fail-closed se ilegível); alternar exige owner autenticado (P1) e gera audit. | Com flag ativa → **503** `orders_halted` já no próximo request; PG indisponível → **503**; toggle sem principal owner → **403**; evento de audit com actor e correlation. Pré-P1: não há rota HTTP de toggle; o comando de CLI grava o evento com o usuário do SO e o host. |
| **SEC-ORD-13** | agora (por token/IP); pós-P1: por principal | Rate limit por principal/conta em submit e poll. | Mais de N submits/min (N configurado) → **429** e executor não chamado. Pré-P1 a chave é o IP do peer (ver SEC-ADM-11). |
| **SEC-ORD-14** | agora | Reconciliação valida símbolo, lado, `client_order_id`, quantidade/custo executados (tolerância definida); erros de fetch contam tentativas e alertam. | Fake retorna `Closed` com lado/quantidade divergentes → estado `divergent` com razão categorizada; 3 erros consecutivos → métrica/alerta, sem marcar `reconciled`. |
| **SEC-ORD-15** | agora | Erros mapeados para códigos genéricos; detalhe apenas em log redigido (chaves testnet e prod, `signature=`, `X-MBX-APIKEY`). | Fake error contendo chave, secret e `signature=…` → corpo HTTP sem nenhum deles; log capturado com `<redacted>`. |
| **SEC-ORD-16** | agora (actor = `admin_token`); pós-P1: principal | Auditoria append-only de submit (allow/deny): actor, org, conta, `client_order_id`, lado, quote, modo, resultado, correlation. | Cada submit (aceito, rejeitado por risco, 401/403, 503) gera 1 evento; role da app sem UPDATE/DELETE. Pré-P1 o actor é `admin_token` (sem identidade humana), e isso fica explícito no evento. |
| **SEC-ORD-17** | agora | Execução assíncrona (sem `block_on` em contexto Tokio) ou `spawn_blocking`. | **Teste vermelho primeiro (O3):** antes da correção, um `#[tokio::test(flavor = "multi_thread")]` que chama `ApiState::submit_order_http` com `live_exchange`/`testnet` deve **falhar**. O `Runtime::block_on` entra em pânico ao ser chamado, antes de fazer poll do future, mas o caminho de submit só chega lá depois de `testnet_credentials()?`, `load_registry` e `build_dev_spot_binance` (`binance_spot_testnet_submit.rs:162-167`); por isso o teste precisa de credenciais `BINANCE_TESTNET_*` **falsas** no ambiente do teste, e não de um transporte ccxt fake (nenhuma requisição sai antes do pânico; não verificado dinamicamente). O teste deve **falhar** (pânico "Cannot start a runtime from within a runtime" ou equivalente), registrando o comportamento atual; depois da correção o mesmo teste passa, sem pânico, e um segundo request concorrente não fica bloqueado além de X ms. O mesmo par vermelho/verde vale para `POST /orders/reconciliation/poll`. Pânico no caminho de submit nunca deixa claim órfão (ver SEC-ORD-20). |
| **SEC-ORD-18** | agora | `paper_fill_unit_price` do corpo só em `paper` + ambiente dev; senão fonte server-side. | Em modo diferente de `paper` → **400**. |
| **SEC-ORD-19** | agora | Allowlist de símbolos por conta/mercado. | Símbolo fora da lista → **400** antes do risco/adapter. |
| **SEC-ORD-21** | agora | Recuperação de claim órfão: a linha de claim ganha `state` (`in_flight`/`completed`/`unknown`/`failed`), `claimed_at` e `lease_until` (lease renovado enquanto o fluxo roda); no boot e periodicamente, uma varredura move `in_flight` com lease vencido para `unknown`; nenhum reenvio acontece para uma key `unknown` sem reconciliação prévia com a exchange (`fetch_order` por `client_order_id`). | (a) Pânico injetado depois do claim → após o lease, a key está `unknown`, retry → **409** `order_outcome_unknown`, nunca `accepted: true`. (b) **Crash real do processo** (teste que mata o processo filho com SIGKILL entre claim e envio, e outro entre envio e resposta) → no boot seguinte a varredura marca `unknown` e a reconciliação resolve: sem ordem na fake → `failed` e retry permitido; com ordem → `completed` e retry → `accepted: true` com o mesmo `exchange_order_id`. (c) Nenhuma key fica `in_flight` além do lease. |
| **SEC-ORD-22** | agora | Supervisão da reconciliação em background: o `JoinHandle` da task de poll é observado; saída por pânico ou retorno gera log `error` com o motivo, métrica (`order_reconciliation_poll_alive` = 0, contador de reinícios) e reinício com backoff limitado; `/readyz` fica **não pronto** quando a task está morta, ou quando o último tick bem-sucedido é mais antigo que K intervalos, enquanto `live_exchange` estiver ligado; alerta operacional quando reinícios passam do limite. | (a) Teste que injeta pânico no tick → log `error` capturado, métrica muda, task reiniciada; (b) teste que aborta a task e não a deixa voltar → `/readyz` responde não pronto em até K intervalos; (c) com erro recorrente (`Err` em todo tick), o contador de falhas sobe e `/readyz` degrada, em vez de só `warn`. |
| **SEC-ORD-20** | agora | Claim com estado explícito (`in_flight` → `completed`/`unknown`/liberado); `in_flight` e `unknown` nunca respondem `accepted`; memória só grava concluído depois de conclusão real; claim `in_flight` órfão (pânico, crash) vira `unknown` e exige reconciliação, não aceite. | (a) Com um submit em voo (executor fake bloqueado), o segundo submit com a mesma key → **409** `idempotency_in_flight`, `accepted` ausente, executor chamado 1 vez. (b) **Só no caminho `LiveExchange`**, o único modo que despacha para `ExchangeSpotExecutor` (`presentation/http/order_execution.rs:108-119`, braço `LiveExchange` em `:114`; `Disabled` → `FailClosedExecutor`, `DevAccept` → `AcceptingExecutor`, `Paper` → `PaperLedgerExecutor`, `LiveExchangeReserved` → `ReservedLiveExchangeExecutor`): pânico injetado em `ExchangeSpotExecutor` depois do claim → retry com a mesma key → **409** `order_outcome_unknown`, nunca `accepted: true`. (b′) Para cada modo diferente de `LiveExchange`, um submit com executor de exchange espião nunca chama `ExchangeSpotExecutor` (contagem zero), e o teste de dispatch cobre os cinco braços do `match`. (c) **Exatamente uma vez:** N retries concorrentes contra fake exchange → a fake registra exatamente 1 ordem e o cliente recebe `accepted: true` só quando essa ordem existe. (d) Nenhum caminho grava a key em `InMemoryOrderIdempotencyStore` quando o PG respondeu "claim ocupado". |

## 7. Achados

| ID | Sev. | Achado | Local | Correção | Critérios |
|---|---|---|---|---|---|
| **F-ORD-01** | **Alto** | Limites de risco e exposição vêm do corpo do cliente; o gate de risco é contornável por quem pode chamar o submit. O threat model do SDD G2 marca "Bypass de risco" sem gap. | `http_bridge/orders.rs:18-32, 149`; `controllers/submit.rs`; SDD G2 tabela de riscos ("Bypass de risco") | Limites e exposição server-side; remover campos do DTO. | SEC-ORD-01, 02 |
| **F-ORD-02** | **Alto** | Sem principal/tenant em orders: submit protegido só por bearer compartilhado e **opcional** (fail-open com env vazio); reconciliação GET sem auth; keys e ledger globais. | `admin_auth.rs:91-94`; `routes/orders.rs:39-58, 94-105`; migrations `0004:2-5`, `0006:2-10` | Corrigir F-ADM-01 (SEC-ADM-01…03); auth obrigatória em modos que executam; escopo org/conta; futuramente capability via `org`. | SEC-ORD-03, 04, 05; SEC-ADM-01, 03, 05 |
| **F-ORD-03** | **Alto** (**Crítico** com fundos reais) | Erro depois do envio libera o claim → a próxima tentativa envia **ordem duplicada** (gasto duplo com dinheiro real). Qualquer `Err` que saia de `submit_order_http` cai em `release_claim` (`state.rs:566-571`), que apaga a linha (`pg_idempotency.rs:60-61`), sem distinguir erro antes do envio de erro depois dele. Erros possíveis depois de a ordem sair: `create_order` com timeout/conexão caída/resposta ilegível (`binance_spot_testnet_submit.rs:91-100` compra, `:144-153` venda, propagado em `:167-173`); e, no bridge, a ordem sai em `http_bridge/orders.rs:166`, a key é gravada como concluída **em memória** em `:168` e só depois `mark_pending` roda em `:170`. `mark_pending` pode devolver `InvalidRequest` (`in_memory_reconciliation.rs:90-94` tamanho, `:96-99` "already tracked"); o erro de tamanho é inalcançável hoje porque `validate_client_order_id` (`http_bridge/orders.rs:117`) valida antes, e "already tracked" exige ledger e store divergentes, mas o desenho libera o claim em qualquer erro pós-envio. A memória gravada em `:168` esconde a duplicação na mesma instância (retry lá recebe `accepted: true` por `state.rs:537`), enquanto outra instância ou um restart reenviam. | `state.rs:560-573`; `pg_idempotency.rs:60-61`; `binance_spot_testnet_submit.rs:91-100, 144-153, 167-173`; `http_bridge/orders.rs:166-170`; `in_memory_reconciliation.rs:90-99` | Nunca liberar o claim depois de uma tentativa de envio; erro ambíguo pós-envio → `unknown`/`reconciling`; memória só grava concluído no fim do fluxo; reconciliar antes de reenviar. | SEC-ORD-08, 20, 21 |
| F-ORD-04 | Médio | Dedupe em memória não atômico (TOCTOU) e key sem vínculo ao payload. | `state.rs:537`; `http_bridge/orders.rs:144-148, 166-171`; `idempotency.rs` | Claim atômico + hash do payload; PG obrigatório em live. | SEC-ORD-06, 07 |
| F-ORD-05 | Médio | Ack global `LAST_SUBMIT_ACK` → correlação errada sob concorrência (integridade da reconciliação). | `spot_order_submit.rs:9, 33-39, 81`; `state.rs:575-587` | Ack no retorno do port. | SEC-ORD-09 |
| F-ORD-06 | Médio | Reconciliação aceita `Closed` sem validar lado/quantidade; erros viram `pending` silencioso. | `binance_spot_testnet_reconcile.rs:32-43, 63-66` | Validação de campos + contagem de falhas/alerta. | SEC-ORD-14 |
| F-ORD-07 | Médio | Conta Dev fixa no executor; invariantes "prod negado" não cobrem o caminho real (falsa garantia no checklist G2). | `exchange_spot_executor.rs:12`; `exchange_order_gate.rs:18-26`; `threat_model_invariants.rs:31-55` | Passar conta resolvida; teste do caminho real. | SEC-ORD-10 |
| F-ORD-08 | Médio | Compra a mercado depende de o ccxt honrar `cost` com `amount=1`; não verificável sem o crate `ccxt_exchanges`. | `binance_spot_testnet_submit.rs:76-101` | `quoteOrderQty` explícito + teste de payload + teto de notional. | SEC-ORD-11 |
| F-ORD-09 | Médio | Sem kill switch em runtime e sem rate limit. | `order_execution.rs` (modo no boot); rotas sem limitador | Flag PG + limitador. | SEC-ORD-12, 13 |
| F-ORD-10 | Médio | Sem trilha de auditoria de submit com actor; idempotência não guarda payload; projeção Neo4j é best-effort. | `state.rs:598-627`; `0004` | Tabela de auditoria append-only. | SEC-ORD-16 |
| **F-ORD-11** | **Alto** (era Médio) | `Runtime::block_on` de um runtime current-thread próprio chamado de dentro de handler async, sem `spawn_blocking` nem `block_in_place`: `ApiState::submit_order_http` (async) chama `http_bridge::orders::submit_order_http` (síncrono), que chega a `submit_testnet_spot_market_order` → `ccxt_runtime().block_on(...)`. No Tokio isso provavelmente entra em pânico ("Cannot start a runtime from within a runtime") em **toda** submissão `live_exchange`/testnet, e o mesmo vale para o poll de reconciliação. O pânico acontece depois do claim e antes de `release_claim` (`state.rs:566-571`), então a key fica ocupada para sempre (ver F-ORD-16 e F-ORD-17); o poll de reconciliação morre pelo mesmo pânico (F-ORD-18). Não verificado dinamicamente; o teste vermelho de SEC-ORD-17 é o primeiro passo. Terceiro `block_on` do mesmo tipo em `live_reconciliation_pg_mirror.rs:37` (chamado de `monitor/controllers/supervisor.rs:84`, resultado descartado); contexto de execução não verificado. | `binance_spot_testnet_submit.rs:20-28, 166-167`; `binance_spot_testnet_reconcile.rs:64`; `state.rs:560-573`; nenhum `spawn_blocking`/`block_in_place` em `presentation/` ou `modules/` | Port async (`execute` → `async fn`) ou `spawn_blocking` com claim liberado/marcado `unknown` em qualquer saída, inclusive pânico. | SEC-ORD-17, 20, 21, 22 |
| F-ORD-12 | Baixo | Texto bruto de exchange/driver no corpo HTTP; redação só dos valores testnet por substituição exata. | `binance_spot_testnet_submit.rs:35-41`; `core/config/exchanges/credentials.rs:27-37`; `error.rs:195-214` | Códigos genéricos + redator abrangente. | SEC-ORD-15 |
| F-ORD-13 | Baixo | `paper_fill_unit_price` controlado pelo cliente. | `http_bridge/orders.rs:29-31` | Restringir a paper/dev. | SEC-ORD-18 |
| F-ORD-14 | Baixo | Símbolo só validado como não vazio. | `models/request.rs` (`validate`) | Allowlist. | SEC-ORD-19 |
| **F-ORD-16** | **Alto** | Claim ocupado tratado como pedido concluído e aceito. `order_idempotency_keys` tem só `client_order_id` e `recorded_at` (`0004:2-5`), e a mesma linha serve de claim e de "concluído". Assim, `pg.is_completed` devolve `true` para uma key ainda em voo (`state.rs:541-543`), e quando `try_claim` perde a corrida o código grava a key como concluída em memória e responde `accepted: true` (`state.rs:545-547`). Um retry durante a execução, ou depois de uma falha que não liberou o claim, recebe "aceito" sem ordem enviada; a partir daí aquela instância responde "aceito" pela memória (`state.rs:537-538`) mesmo que o claim seja liberado depois. | `state.rs:537-547`; `pg_idempotency.rs:23-31, 46-57`; `0004:2-5` | Coluna de estado (`in_flight`/`completed`/`unknown`); `in_flight` → **409**; memória só depois de conclusão real; órfão → `unknown`. | SEC-ORD-06, 08, 20 |
| **F-ORD-17** | **Alto** | Claim órfão sem mecanismo de recuperação. `try_claim` faz `INSERT` (`pg_idempotency.rs:46-57`) numa tabela que só tem `client_order_id` e `recorded_at` (`0004:2-5`): sem estado, lease, TTL nem varredura. `release_claim` só roda no caminho `Err` (`state.rs:566-571`); se o handler entra em pânico (F-ORD-11) ou o processo morre (crash, OOM, deploy, SIGKILL) entre o claim e o fim do fluxo, a linha fica para sempre. Todo retry cai em `pg.is_completed` (`state.rs:541-543`) e recebe `accepted: true` sem envio; se a ordem chegou a sair, nada a reconcilia, porque o registro de reconciliação só acontece depois de um submit bem-sucedido. | `pg_idempotency.rs:46-57`; `0004:2-5`; `state.rs:541-543, 566-571` | Estado + `claimed_at`/lease; varredura no boot e periódica `in_flight` vencido → `unknown`; reconciliação antes de qualquer reenvio; teste com crash real do processo, não só pânico injetado. | SEC-ORD-20, 21 |
| **F-ORD-18** | **Alto** | A task de reconciliação em background pode morrer em silêncio. `tokio::spawn` em `server.rs:33`, loop em `:36-45`, criada só quando o intervalo de poll está configurado e `live_exchange_wired` é verdadeiro (`server.rs:30-31`). O loop só trata `Err` com `tracing::warn`; o `JoinHandle` é descartado, e não há supervisão, reinício, log de saída, métrica nem efeito em `/readyz`. Um pânico termina a task: a cadeia `run_order_reconciliation_poll_once` (`state.rs:771`) → `reconcile_pending_orders_once` (`state.rs:724`) → `run_reconciliation_poll_once` (síncrono) → `observe_testnet_spot_order_by_client_id` → `ccxt_runtime().block_on` (`binance_spot_testnet_reconcile.rs:64`) entra em pânico pelo mesmo motivo de F-ORD-11 sempre que há credenciais testnet (sem elas, `:58-60` devolve `StillPending` antes). Envenenamento de mutex depois desse pânico não verificado. **Severidade:** o Critic propôs Médio; aqui fica **Alto** porque, com credenciais testnet, a morte é determinística no primeiro tick, e SEC-ORD-08/20/21 dependem da reconciliação para tirar keys de `unknown`: sem ela, ordens ambíguas nunca se resolvem e divergências com a exchange passam despercebidas. | `server.rs:30-45`; `state.rs:724, 771`; `binance_spot_testnet_reconcile.rs:58-64` | Supervisão do `JoinHandle` com reinício e backoff; log `error` na saída; métrica e `/readyz` não pronto com a task morta ou sem tick recente; alerta. | SEC-ORD-22, 17 |
| F-ORD-19 | Baixo | Erro devolvido para ordem que já foi executada: depois de `submit_order_http` voltar `Ok`, `confirm_exchange_order` (`state.rs:584`), `upsert_state` (`:592`) e `persist_idempotency_and_enqueue_graph_projection` (`:612`) usam `?` e devolvem erro ao cliente. O claim fica (não passa por `release_claim`), então o retry recebe `accepted: true`; o cliente viu erro para uma ordem que existe. | `state.rs:575-627` | Contrato de erro documentado: esses erros viram resposta de sucesso com aviso de reconciliação pendente, ou `202` com estado `reconciling`, nunca erro que convide a reenviar com outra key. | SEC-ORD-08, 15 |
| F-ORD-15 | Baixo | `quote_amount` em `f64` e `Decimal::from_f64_retain` (arredondamento/precisão). | `models/request.rs`; `binance_spot_testnet_submit.rs` (venda) | `Decimal` fim a fim com precisão do mercado. | SEC-ORD-11 |

**Contagem:** Crítico 0 · Alto 7 (F-ORD-01, 02, 03, 11, 16, 17, 18) · Médio 7 · Baixo 5.

### 7.1 Combinação F-ORD-16 + F-ORD-11 (integridade de ordens)

No caminho `live_exchange`/testnet, a primeira submissão com uma `client_order_id` grava o claim e, pela análise estática, entra em pânico no `block_on` antes de chegar à exchange. O pânico impede o `release_claim`, e a linha fica em `order_idempotency_keys` sem prazo de expiração. Todo retry com a mesma key cai em `pg.is_completed` e recebe `accepted: true`, e nenhuma ordem chega a ser enviada. Como a reconciliação só é registrada depois de um submit bem-sucedido (`state.rs:575-596`), nada no sistema indica que a ordem não existe.

**Veredito:** falha de **integridade de dados de ordens**. O sistema afirma ao cliente, de forma persistente, que uma ordem foi aceita quando ela nunca foi enviada. Severidade da combinação: **Alto** hoje (testnet/recording, sem dinheiro real, mas quebra qualquer contabilidade de posições e P&L feita a partir das respostas); **Crítico** em qualquer ambiente com fundos reais. Bloqueia G2 junto com F-ORD-01…03, e F-ORD-16 bloqueia também os modos `paper`/`dev_accept` com PG, porque a corrida entre requisições concorrentes não depende do pânico.

Credenciais de exchange/provider: ver [provider-credentials-plaintext](./provider-credentials-plaintext.md) (F-CRED-*). Auth admin fail-open: ver [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01).

## 8. Pontos positivos verificados (manter)

- Prod REST negado por `authorize_rest_use` (`rest.rs:31-50`) e builder testnet com `sandbox(true)` + endpoint exato (`binance.rs:21-45`, `account_file.rs:111-120`), com testes de allowlist contra host confuso (`@`, sufixo, porta, path, query).
- `try_claim` PG atômico (`INSERT … ON CONFLICT DO NOTHING RETURNING`) e 503 fail-closed quando o store falha.
- Risco é sempre chamado antes do port (`controllers/submit.rs`) — o problema é a **origem** dos limites.

## 9. Limitações

- O crate `ccxt_exchanges` não está vendorizado; F-ORD-08 exige teste. O pânico de F-ORD-11 é inferido do comportamento documentado do Tokio para `Runtime::block_on` dentro de um contexto de runtime; nenhum teste foi executado.
- Não verificado: configuração de deploy (bind, token), permissões das chaves testnet, logs do servidor.

## 10. Bloqueios e próximos passos

- F-ORD-01…03, 11, 16, 17 e 18 (Alto) bloqueiam o fechamento do checklist G2 e qualquer habilitação além de testnet/recording. Ordem de trabalho sugerida: testes vermelhos de SEC-ORD-17, 20, 21 e 22 antes de qualquer correção.
- Qualquer mainnet exige ainda: owner auth P1, integração com `org` (capability + epoch), SEC-ORD-10…13 e SEC-CRED (credenciais) aprovados por Critic.

## 11. Registro do ciclo 2 (follow-ups O1-O3 do Critic)

| Item | O que mudou | Seção |
|---|---|---|
| O1 | Novo achado **F-ORD-16** (Alto): claim ocupado vira `accepted` (`state.rs:545-547`, e também `:541-543`); novo abuso AB-O16; novo critério SEC-ORD-20 (409 para `in_flight`, órfão → `unknown`, teste de exatamente uma vez) | §5, §6, §7 |
| O2 | Coluna "Dep." (agora / pós-P1 / pós-org) na tabela SEC-ORD, com variante pré-P1 em SEC-ORD-05, 12, 13, 16 | §6 |
| O3 | F-ORD-11 reescrito com a cadeia de chamadas e sem `spawn_blocking`; severidade **Médio → Alto**; SEC-ORD-17 exige teste vermelho antes da correção; linhas corrigidas (`:166-167`, `:64`) e terceiro `block_on` registrado | §5 AB-O12, §6 SEC-ORD-17, §7 |
| O1+O3 | Combinação avaliada como falha de integridade de dados de ordens: **Alto** hoje, **Crítico** com fundos reais | §7.1 |

SDDs de correção criados por outra sessão, em draft e **não revisados** aqui: [wave0-12-order-ambiguous-claim-sdd](../sdd/wave0-12-order-ambiguous-claim-sdd.md) (F-ORD-03; não verificado se cobre F-ORD-16) e [wave0-13-orders-block-on-sdd](../sdd/wave0-13-orders-block-on-sdd.md) (F-ORD-11). A revisão deles contra SEC-ORD-08, 17 e 20 fica para o próximo pedido.

## 12. Registro do ciclo 3 (Critic parte 1 e Orquestrador)

| Item | O que mudou | Seção |
|---|---|---|
| W0-12 (b) | F-ORD-03 restaurado com força total: erro depois do envio libera o claim → ordem duplicada; evidência em `create_order` e no bridge (`http_bridge/orders.rs:166-170`); **Alto**, **Crítico** com fundos reais; SEC-ORD-08 endurecido (nunca liberar após tentativa de envio) | §5 AB-O6, §6 SEC-ORD-08, §7 |
| W0-12 (a) | Novo **F-ORD-17** (Alto): claim órfão sem lease/TTL/varredura; novo AB-O17; novo SEC-ORD-21 com teste de crash real do processo | §5, §6, §7 |
| Poll | Novo **F-ORD-18** (Alto; o Critic propôs Médio, divergência justificada no achado): task de reconciliação morre em silêncio; novo AB-O18; novo SEC-ORD-22 | §5, §6, §7 |
| O3 | SEC-ORD-17: o teste vermelho precisa de credenciais `BINANCE_TESTNET_*` falsas, não de transporte ccxt fake, porque o pânico vem antes de qualquer requisição (não verificado) | §6 SEC-ORD-17 |
| SEC-ORD-20 (b) | Escopo restrito ao caminho `LiveExchange` → `ExchangeSpotExecutor` (`order_execution.rs:108-119`); novo (b′): outros modos nunca chegam ao executor de exchange | §6 SEC-ORD-20 |
| Baixo | Novo F-ORD-19: erro devolvido para ordem já executada (`state.rs:584, 592, 612`), contrato de erro a documentar | §7 |
| Nota | `pg_idempotency.rs` foi modificado no working tree por outra sessão depois da linha 115; as linhas citadas (23-31, 46-57, 60-61) não mudaram | — |
