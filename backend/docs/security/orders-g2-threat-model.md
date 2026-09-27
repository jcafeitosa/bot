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

- **Estado:** proposto — aguardando revisão do Critic independente. Não fecha o item "Threat model + revisão Critic" do checklist G2 e não autoriza prod REST nem trading live.
- **Tipo:** documentação apenas. Complementa (não substitui nem edita) a seção "Threat model" de [`sdd/orders-live-execution-gate2-sdd.md`](../sdd/orders-live-execution-gate2-sdd.md).
- **Autor (Builder):** agente executor de segurança/Blue Team. **Critic:** pendente de atribuição.
- **Data da revisão:** 2026-09-27. **Método:** leitura estática local; nenhuma chamada a exchange, testnet ou sistema vivo.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01, causa raiz de AB-O2), [provider-credentials-plaintext](./provider-credentials-plaintext.md), [org-module-threat-model](./org-module-threat-model.md).

## 0. Entradas revisadas

| Entrada | Estado |
|---|---|
| [`sdd/orders-live-execution-gate2-sdd.md`](../sdd/orders-live-execution-gate2-sdd.md) | `status: draft`, estado "parcial"; igual a HEAD `2d1e3863` no momento da revisão |
| [`sdd/orders-module-sdd.md`](../sdd/orders-module-sdd.md) | `status: draft`; com alterações não commitadas no working tree (outra sessão) |
| [`sdd/http-admin-auth-seam-sdd.md`](../sdd/http-admin-auth-seam-sdd.md) | `status: partial` |
| Código (HEAD `2d1e3863`; arquivos citados inalterados desde `0880cdd5`) | `presentation/http/{routes/orders.rs,state.rs (submit_order_http 512-630),order_execution.rs,error.rs,admin_auth.rs}`; `modules/http_bridge/orders.rs`; `modules/orders/{controllers/submit.rs,models/request.rs,adapters/{idempotency.rs,pg_idempotency.rs,spot_order_submit.rs,exchange_spot_executor.rs,exchange_order_gate.rs,spot_order_reconciliation_query.rs},threat_model_invariants.rs}`; `modules/exchanges/{rest.rs,adapters/{binance.rs,binance_spot_testnet_submit.rs,binance_spot_testnet_reconcile.rs,account_file.rs}}`; `core/config/{exchanges/credentials.rs,orders/file.rs,mod.rs}`; migrations `0004`, `0006` |

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
| AB-O6 | **Resultado ambíguo → ordem dupla:** exchange aceita, mas resposta falha/timeout; `release_claim` libera a key; retry do cliente reexecuta (ordem a mercado já executada não é deduplicada pela exchange). | `state.rs:566-571`; `pg_idempotency.rs` (`release_claim`). |
| AB-O7 | **Troca de acks entre requisições concorrentes:** ack guardado em `static LAST_SUBMIT_ACK` e lido depois por `take_last_spot_submit_ack()` → `client_order_id` A ligado ao `exchange_order_id` de B. | `spot_order_submit.rs:9, 33-39, 81`; `state.rs:575-587`. |
| AB-O8 | **Garantia ilusória de ambiente:** o executor real sempre passa uma conta Dev fixa ao gate; os testes "prod negado" exercitam uma conta Prod construída no teste, não o caminho real. A proteção efetiva é o builder testnet. | `exchange_spot_executor.rs:10-14`; `exchange_order_gate.rs:18-26`; `threat_model_invariants.rs:31-55`; `binance.rs:21-45`. |
| AB-O9 | **Dimensionamento incorreto:** compra a mercado envia `amount = 1` (base) + param `cost`; se o adapter ccxt ignorar `cost`, compra 1 unidade base (ex.: 1 BTC). | `binance_spot_testnet_submit.rs:76-101` (crate `ccxt_exchanges` não está vendorizado — não verificável estaticamente). |
| AB-O10 | **Reconciliação confia no status da exchange** sem comparar símbolo/lado/quantidade/custo; erro de fetch vira "pending" silencioso. | `binance_spot_testnet_reconcile.rs:32-43, 46-67`. |
| AB-O11 | **Sem kill switch/rate limit:** parar envio exige mudar env e reiniciar; sem limite de taxa/notional por janela. | `order_execution.rs` (modo fixado no boot); nenhuma checagem por request. |
| AB-O12 | **DoS/pânico:** `Runtime::block_on` de runtime current-thread chamado dentro de handler async. | `binance_spot_testnet_submit.rs:20-28, 165`; `binance_spot_testnet_reconcile.rs:63`. |
| AB-O13 | **Vazamento em erros:** texto bruto de exchange/driver volta ao cliente; redação só por substituição exata dos valores testnet (não cobre prod, assinatura HMAC, valores parciais/codificados). | `binance_spot_testnet_submit.rs:35-41`; `core/config/exchanges/credentials.rs:27-37`; `error.rs:195-214`. |
| AB-O14 | **Adulteração de P&L paper** via `paper_fill_unit_price` do corpo. | `http_bridge/orders.rs:29-31`. |
| AB-O15 | **Spoofing de resposta da exchange** (MITM/DNS). | Mitigado: HTTPS + host exato (`account_file.rs:111-120` e testes `dev_spot_rejects_urls_outside_exact_testnet_allowlist`); manter e não permitir override. |

## 6. Mitigações e critérios de aceite testáveis (SEC-ORD)

| ID | Mitigação | Critério de aceite testável |
|---|---|---|
| **SEC-ORD-01** | Limites de risco somente server-side (config/policy por conta/bot/org); DTO sem `limits` (`deny_unknown_fields`). | Body com `limits` → **400** `unknown_field`; com limite server-side 10 e `quote_amount` 50 → **422** `risk_rejected` e `RecordingExecutor::call_count() == 0`, independentemente do que o corpo declarar. |
| **SEC-ORD-02** | Exposição (`estimated_daily_loss`, `open_positions`) calculada do ledger/portfolio/reconciliação, não do cliente. | Ledger com `open_positions == max` e corpo declarando 0 → **422**; campo no corpo → **400**. |
| **SEC-ORD-03** | Autenticação obrigatória para qualquer modo que execute: startup falha se `BOT_ORDERS_EXECUTION ∈ {dev_accept, paper, live_exchange}` sem `BOT_HTTP_ADMIN_TOKEN` (ou bind não-loopback sem auth). Pós-P1/`org`: capability `orders.submit` do principal para a conta/org, com epoch verificado (SEC-ORG-11). | Teste de boot com execução `paper` e token vazio → erro de configuração; submit sem bearer → **401**; pós-org: principal sem capability → **403** e audit deny. |
| **SEC-ORD-04** | `GET /orders/reconciliation/{id}` autenticado e escopado por tenant. | Sem bearer → **401**; pedido de ordem de outra org → **404** idêntico a inexistente. |
| **SEC-ORD-05** | Escopo por tenant/conta nas tabelas: `PRIMARY KEY (org_id, account_id, client_order_id)` em idempotência e reconciliação. | Mesma `client_order_id` em duas orgs → duas ordens independentes; replay de A nunca responde por B. |
| **SEC-ORD-06** | Claim atômico também em memória (insert-if-absent antes de executar; estado `in_flight`); PG obrigatório em `live_exchange`. | 50 submits concorrentes com a mesma key (executor fake com latência) → `call_count() == 1`; demais recebem replay ou **409** `idempotency_in_flight`; boot `live_exchange` sem PG → erro. |
| **SEC-ORD-07** | Key amarrada ao hash canônico do payload. | Mesma key com `quote_amount` diferente → **409** `idempotency_key_reuse`, executor não chamado. |
| **SEC-ORD-08** | Resultado ambíguo não libera claim: estado `unknown` até reconciliação; retry bloqueado. Só liberar em falha **pré-dispatch** comprovada. | Fake exchange que aceita e depois devolve timeout: após N retries do cliente com a mesma key, a fake registra **exatamente 1** ordem; retry durante `unknown` → **409** `order_outcome_unknown`. |
| **SEC-ORD-09** | Ack retornado pelo port (`execute` → `OrderAck`), sem estado global. | 100 submits concorrentes (backend recording) → mapeamento `client_order_id ↔ exchange_order_id` é bijetivo e cada ack corresponde à própria requisição. |
| **SEC-ORD-10** | Gate de ambiente usa a conta efetivamente resolvida da config/registro (não conta Dev fixa); teste no caminho real do executor. | Registro com conta Spot `Prod` + seam `recording` → submit via `ExchangeSpotExecutor` retorna **503** `live_exchange_not_wired`, sem ack; teste de rede fake prova que nenhum host mainnet é contatado. |
| **SEC-ORD-11** | Dimensionamento explícito (`quoteOrderQty` para compra por quote; quantidade base calculada com `Decimal` e precisão de mercado) + teto de notional server-side por ordem e por janela. | Fake HTTP captura a requisição de compra: contém `quoteOrderQty == quote_amount` e **não** contém `quantity=1`; ordem acima do teto → **422** antes do adapter. |
| **SEC-ORD-12** | Kill switch persistente (flag PG verificada a cada submit, fail-closed se ilegível); alternar exige owner autenticado (P1) e gera audit. | Com flag ativa → **503** `orders_halted` já no próximo request; PG indisponível → **503**; toggle sem principal owner → **403**; evento de audit com actor e correlation. |
| **SEC-ORD-13** | Rate limit por principal/conta em submit e poll. | Mais de N submits/min (N configurado) → **429** e executor não chamado. |
| **SEC-ORD-14** | Reconciliação valida símbolo, lado, `client_order_id`, quantidade/custo executados (tolerância definida); erros de fetch contam tentativas e alertam. | Fake retorna `Closed` com lado/quantidade divergentes → estado `divergent` com razão categorizada; 3 erros consecutivos → métrica/alerta, sem marcar `reconciled`. |
| **SEC-ORD-15** | Erros mapeados para códigos genéricos; detalhe apenas em log redigido (chaves testnet e prod, `signature=`, `X-MBX-APIKEY`). | Fake error contendo chave, secret e `signature=…` → corpo HTTP sem nenhum deles; log capturado com `<redacted>`. |
| **SEC-ORD-16** | Auditoria append-only de submit (allow/deny): actor, org, conta, `client_order_id`, lado, quote, modo, resultado, correlation. | Cada submit (aceito, rejeitado por risco, 401/403, 503) gera 1 evento; role da app sem UPDATE/DELETE. |
| **SEC-ORD-17** | Execução assíncrona (sem `block_on` em contexto Tokio) ou `spawn_blocking`. | Teste em runtime multi-thread chamando o caminho testnet com transporte fake não entra em pânico e não bloqueia outro request concorrente além de X ms. |
| **SEC-ORD-18** | `paper_fill_unit_price` do corpo só em `paper` + ambiente dev; senão fonte server-side. | Em modo diferente de `paper` → **400**. |
| **SEC-ORD-19** | Allowlist de símbolos por conta/mercado. | Símbolo fora da lista → **400** antes do risco/adapter. |

## 7. Achados

| ID | Sev. | Achado | Local | Correção | Critérios |
|---|---|---|---|---|---|
| **F-ORD-01** | **Alto** | Limites de risco e exposição vêm do corpo do cliente; o gate de risco é contornável por quem pode chamar o submit. O threat model do SDD G2 marca "Bypass de risco" sem gap. | `http_bridge/orders.rs:18-32, 149`; `controllers/submit.rs`; SDD G2 tabela de riscos ("Bypass de risco") | Limites e exposição server-side; remover campos do DTO. | SEC-ORD-01, 02 |
| **F-ORD-02** | **Alto** | Sem principal/tenant em orders: submit protegido só por bearer compartilhado e **opcional** (fail-open com env vazio); reconciliação GET sem auth; keys e ledger globais. | `admin_auth.rs:91-94`; `routes/orders.rs:39-58, 94-105`; migrations `0004:2-5`, `0006:2-10` | Corrigir F-ADM-01 (SEC-ADM-01…03); auth obrigatória em modos que executam; escopo org/conta; futuramente capability via `org`. | SEC-ORD-03, 04, 05; SEC-ADM-01, 03, 05 |
| **F-ORD-03** | **Alto** | Resultado ambíguo libera o claim e permite reexecução → ordem duplicada (impacto atual limitado a testnet/recording; bloqueia fechamento G2 e qualquer mainnet). | `state.rs:566-571`; `pg_idempotency.rs` `release_claim` | Estado `unknown`; liberar só em falha pré-dispatch; reconciliar antes de novo envio. | SEC-ORD-08 |
| F-ORD-04 | Médio | Dedupe em memória não atômico (TOCTOU) e key sem vínculo ao payload. | `state.rs:537`; `http_bridge/orders.rs:144-148, 166-171`; `idempotency.rs` | Claim atômico + hash do payload; PG obrigatório em live. | SEC-ORD-06, 07 |
| F-ORD-05 | Médio | Ack global `LAST_SUBMIT_ACK` → correlação errada sob concorrência (integridade da reconciliação). | `spot_order_submit.rs:9, 33-39, 81`; `state.rs:575-587` | Ack no retorno do port. | SEC-ORD-09 |
| F-ORD-06 | Médio | Reconciliação aceita `Closed` sem validar lado/quantidade; erros viram `pending` silencioso. | `binance_spot_testnet_reconcile.rs:32-43, 63-66` | Validação de campos + contagem de falhas/alerta. | SEC-ORD-14 |
| F-ORD-07 | Médio | Conta Dev fixa no executor; invariantes "prod negado" não cobrem o caminho real (falsa garantia no checklist G2). | `exchange_spot_executor.rs:12`; `exchange_order_gate.rs:18-26`; `threat_model_invariants.rs:31-55` | Passar conta resolvida; teste do caminho real. | SEC-ORD-10 |
| F-ORD-08 | Médio | Compra a mercado depende de o ccxt honrar `cost` com `amount=1`; não verificável sem o crate `ccxt_exchanges`. | `binance_spot_testnet_submit.rs:76-101` | `quoteOrderQty` explícito + teste de payload + teto de notional. | SEC-ORD-11 |
| F-ORD-09 | Médio | Sem kill switch em runtime e sem rate limit. | `order_execution.rs` (modo no boot); rotas sem limitador | Flag PG + limitador. | SEC-ORD-12, 13 |
| F-ORD-10 | Médio | Sem trilha de auditoria de submit com actor; idempotência não guarda payload; projeção Neo4j é best-effort. | `state.rs:598-627`; `0004` | Tabela de auditoria append-only. | SEC-ORD-16 |
| F-ORD-11 | Médio | `block_on` de runtime próprio dentro de handler async (possível pânico "runtime within runtime"/bloqueio de worker) — não verificado dinamicamente. | `binance_spot_testnet_submit.rs:20-28, 165`; `binance_spot_testnet_reconcile.rs:63` | Port async ou `spawn_blocking`. | SEC-ORD-17 |
| F-ORD-12 | Baixo | Texto bruto de exchange/driver no corpo HTTP; redação só dos valores testnet por substituição exata. | `binance_spot_testnet_submit.rs:35-41`; `core/config/exchanges/credentials.rs:27-37`; `error.rs:195-214` | Códigos genéricos + redator abrangente. | SEC-ORD-15 |
| F-ORD-13 | Baixo | `paper_fill_unit_price` controlado pelo cliente. | `http_bridge/orders.rs:29-31` | Restringir a paper/dev. | SEC-ORD-18 |
| F-ORD-14 | Baixo | Símbolo só validado como não vazio. | `models/request.rs` (`validate`) | Allowlist. | SEC-ORD-19 |
| F-ORD-15 | Baixo | `quote_amount` em `f64` e `Decimal::from_f64_retain` (arredondamento/precisão). | `models/request.rs`; `binance_spot_testnet_submit.rs` (venda) | `Decimal` fim a fim com precisão do mercado. | SEC-ORD-11 |

**Contagem:** Crítico 0 · Alto 3 (F-ORD-01, 02, 03) · Médio 8 · Baixo 4.

Credenciais de exchange/provider: ver [provider-credentials-plaintext](./provider-credentials-plaintext.md) (F-CRED-*). Auth admin fail-open: ver [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01).

## 8. Pontos positivos verificados (manter)

- Prod REST negado por `authorize_rest_use` (`rest.rs:31-50`) e builder testnet com `sandbox(true)` + endpoint exato (`binance.rs:21-45`, `account_file.rs:111-120`), com testes de allowlist contra host confuso (`@`, sufixo, porta, path, query).
- `try_claim` PG atômico (`INSERT … ON CONFLICT DO NOTHING RETURNING`) e 503 fail-closed quando o store falha.
- Risco é sempre chamado antes do port (`controllers/submit.rs`) — o problema é a **origem** dos limites.

## 9. Limitações

- O crate `ccxt_exchanges` não está vendorizado; F-ORD-08 e parte de F-ORD-11 exigem teste. Nenhum teste foi executado.
- Não verificado: configuração de deploy (bind, token), permissões das chaves testnet, logs do servidor.

## 10. Bloqueios e próximos passos

- F-ORD-01…03 (Alto) bloqueiam o fechamento do checklist G2 e qualquer habilitação além de testnet/recording.
- Qualquer mainnet exige ainda: owner auth P1, integração com `org` (capability + epoch), SEC-ORD-10…13 e SEC-CRED (credenciais) aprovados por Critic.
