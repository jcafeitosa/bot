---
title: SDD W0-12 — resultado ambíguo de ordem não libera o claim
description: Fatia Onda 0 que impede ordem duplicada quando a exchange pode ter aceitado a ordem mas a resposta falhou (F-ORD-03 / SEC-ORD-08)
tags:
  - sdd
  - backend
  - orders
  - security
  - wave0
status: draft
---

# SDD W0-12 — resultado ambíguo de ordem não libera o claim [SEGURANÇA]

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre os seams antes do primeiro teste.
- **Origem:** achado **F-ORD-03** (Alto) e abuso **AB-O6** em [orders-g2-threat-model](../security/orders-g2-threat-model.md); critério **SEC-ORD-08**. ID W0-12 é novo (não está em [master-plan](../planning/master-plan.md) §4.1); proposto logo após W0-01 por ser risco de perda financeira.
- **SDDs relacionados:** [orders-live-execution-gate2-sdd](./orders-live-execution-gate2-sdd.md), [orders-module-sdd](./orders-module-sdd.md). Este SDD é só o delta do claim.

## Contexto (evidência no código, HEAD `b2001a7c`)

- `backend/src/presentation/http/state.rs:512-571` (`submit_order_http`): com PG, faz `try_claim(key)` (`:545`) e, em **qualquer** `Err` do executor, chama `pg.release_claim(key)` (`:566-571`).
- `backend/src/modules/orders/adapters/pg_idempotency.rs:60-67`: `release_claim` apaga a linha de `order_idempotency_keys`. O retry do cliente com a mesma key executa de novo.
- `backend/src/modules/exchanges/adapters/binance_spot_testnet_submit.rs:158-173`: o envio à exchange é o `block_on(create_order…)`; qualquer erro (inclusive timeout/rede depois do envio) vira `OrdersError::InvalidRequest` via `map_bot_error` (`:35-37`). Não há distinção entre "não enviado" e "enviado, resposta perdida".
- `pg_idempotency.rs:23-32`: `is_completed` = "linha existe"; um claim em voo parece concluído (`state.rs:541-543`).
- `state.rs:545-548`: quando `try_claim` falha porque outra requisição está com a key em voo, o código chama `record_completed` na memória e responde `accepted: true` — sem ordem garantida. Se a primeira falhar antes do envio e liberar o claim, a memória deste processo já marcou a key como concluída e o retry nunca executa.
- Sem PG não há claim durável (`state.rs:536-549`; `http_bridge/orders.rs:144-171`): erro não grava nada e o retry reexecuta (AB-O4 / F-ORD-04).
- Migração `0004_order_idempotency_keys.sql`: só `client_order_id` e `recorded_at`; sem estado.
- Teste existente `pg_submit_order_idempotency_releases_claim_when_submit_fails` (`state.rs:1885`) cobre rejeição por risco (pré-envio). Esse comportamento continua correto e deve continuar passando.
- Impacto hoje: `live_exchange` testnet/recording, opt-in (padrão `orders.execution=""` → 503). Bloqueia G2 e qualquer mainnet.

## Contradição doc × código

- O comentário da tabela em `0004` diz "Completed client_order_id values", mas a linha é gravada no claim, antes da execução.

## Decisão (correção mais simples correta)

1. Separar o erro do executor em duas classes: **pré-envio** (validação, risco, modo desligado, não wired, credencial/registro ausente, erro ao montar cliente) e **pós-envio/ambíguo** (qualquer erro a partir da chamada `create_order`, inclusive timeout).
2. Só erro pré-envio libera o claim. Erro ambíguo **nunca** libera: a key fica em estado `unknown` e o ledger de reconciliação recebe `pending` para essa key.
3. Estados da key e resposta a uma nova requisição com a mesma key:

   | Estado | Significado | Resposta | Marca concluído? |
   |---|---|---|---|
   | `claimed` | outra requisição em voo | **409** `idempotency_in_flight` | não |
   | `unknown` | envio feito, resultado não confirmado | **409** `order_outcome_unknown` | não |
   | `completed` | ordem confirmada | replay `accepted: true` | já está |
   | (sem linha) | nunca usada ou liberada por erro pré-envio | executa | — |

   O executor nunca é chamado nos três primeiros casos. `record_completed` em memória só depois de `completed` no PG.
4. Saída de `unknown` só por reconciliação contra a exchange (poll existente usa `observe_testnet_spot_order_by_client_id`): ordem encontrada → `completed` + `reconciled`. Ordem não encontrada → continua `unknown` nesta fatia (sem liberação automática); liberação manual/automática fica como decisão do owner com o bot Segurança.
5. Estado explícito em `order_idempotency_keys` (coluna de estado `claimed | completed | unknown`) via migração na **próxima sequência livre no momento da implementação**.

**Alternativa considerada:** sem migração, derivar "unknown" de `order_reconciliation` (`state='pending'` e `exchange_order_id IS NULL`) e apenas remover o `release_claim` do caminho pós-envio. Menos schema, mas só funciona quando a reconciliação está ligada, mistura duas tabelas para uma decisão de idempotência e o replay continuaria respondendo `accepted: true` em vez de 409. Fica como plano B se o owner vetar migração agora.

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Erro de domínio | nova variante `OrdersError::OutcomeUnknown` (sem texto da exchange) produzida só pelo adapter após o envio |
| HTTP | **409** `idempotency_in_flight` (claim ocupado) e **409** `order_outcome_unknown` (resultado ambíguo); corpo sem detalhe da exchange (alinha SEC-ORD-15) |
| Store | `PgOrderIdempotencyStore`: `mark_unknown(key)`, `mark_completed(key)`, `state(key)`; `release_claim` só chamado para erro pré-envio |
| Schema | coluna de estado com `CHECK` em `order_idempotency_keys`; linhas existentes migram como `completed` |
| Política sem PG | `live_exchange` sem PG recusa boot (parte de SEC-ORD-06) — **precisa de decisão** (ver perguntas) |

## Critérios de aceite

- **SEC-ORD-08** de [orders-g2-threat-model](../security/orders-g2-threat-model.md): exchange fake que aceita e depois devolve timeout; após N retries com a mesma key, a fake registra **exatamente 1** ordem; retry durante `unknown` → **409** `order_outcome_unknown`.
- A0. Claim ocupado: com a primeira requisição em voo, a segunda com a mesma key recebe **409** `idempotency_in_flight` e nada é marcado como concluído (nem memória nem PG). Teste com duas requisições concorrentes e executor fake com latência: a primeira falha antes do envio (libera o claim); o retry depois disso executa **exatamente 1** vez.
- A1. Rejeição por risco ou modo desligado continua liberando o claim (teste `pg_submit_order_idempotency_releases_claim_when_submit_fails` segue verde).
- A2. Falha depois do ack (ex.: erro em `confirm_exchange_order` ou `upsert_state`) não deixa a key reexecutável: retry → 409, nunca nova chamada ao executor.
- A3. Reconciliação que encontra a ordem move a key para `completed` e o ledger para `reconciled`; teste PG isolado.
- A4. Migração nova: reaplicação idempotente; linhas antigas viram `completed`; teste de migração no manifesto PG.
- A5. Log do caso ambíguo sem segredo nem texto bruto da exchange.

## Dependências

- W0-01 recomendado antes (submit sem auth amplia o abuso), mas não bloqueia o desenho.
- [W0-13](./wave0-13-orders-block-on-sdd.md) antes: hoje o caminho testnet entra em pânico no `block_on` (depois do `try_claim`), e W0-13 reescreve a chamada `create_order` onde esta fatia classifica o erro. Arquivos compartilhados: `binance_spot_testnet_submit.rs`, `execution_port.rs`, `models/error.rs`, `http_bridge/orders.rs`, `state.rs`. Mesmo Builder ou sequência estrita.
- W0-02: os testes PG novos só valem como evidência quando falham sem PG.
- Relaciona-se com SEC-ORD-06/07/09 (F-ORD-04/05); ficam fora desta fatia, salvo decisão do owner.

## Riscos

- Claim `claimed` órfão (processo caiu ou pânico, ver W0-13) fica em 409 `idempotency_in_flight` até reconciliação ou ação manual; seguro, mas precisa de runbook.
- Classificação errada de um erro pré-envio como ambíguo: trava a key (seguro, mas exige ação manual). O inverso é o perigo; por isso tudo a partir do `create_order` é ambíguo.
- `fetch_order` por `client_order_id` no ccxt não é verificável estaticamente (crate não vendorizado); se não consultar por `origClientOrderId`, a key fica `unknown` para sempre. Teste com transporte fake e verificação manual em testnet.
- O ack global `LAST_SUBMIT_ACK` (F-ORD-05) continua; não piora com esta fatia.
- Monitor (`supervisor.rs:221-245`) chama `submit_order` direto, sem claim PG; o ramo testnet do monitor é inalcançável hoje (W0-11).

## Validação

- `backend/scripts/verify-backend-gates.sh`; o teste PG novo entra em `run-pg-integration-tests.sh` e o manifesto `EXPECTED_PG_INTEGRATION_TESTS` sobe junto (checado por `assert-pg-integration-manifest.sh`).
- Teste unitário com executor fake (sem rede) para o 409 e para "exatamente 1 chamada".

## Rollout / rollback

- Rollout: próximo build; só muda comportamento com `orders.execution=live_exchange`. Nenhum deploy autorizado.
- Rollback: reverter o código volta a liberar o claim (reabre F-ORD-03). A coluna nova pode ficar (compatível: código antigo ignora); remover exige outra migração na próxima sequência livre. Keys em `unknown` devem ser revisadas antes de qualquer rollback.
