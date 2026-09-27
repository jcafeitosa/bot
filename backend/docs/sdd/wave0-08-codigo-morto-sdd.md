---
title: SDD W0-08 — remoção de código morto (NIM, stub de notificação, futures, 0003)
description: Fatia Onda 0 que remove código sem chamador de produção e marca a migração 0003 como reservada, sem mudar comportamento do monitor e do serve
tags:
  - sdd
  - backend
  - cleanup
  - wave0
status: draft
---

# SDD W0-08 — código morto

- **Estado:** draft, ajustes do follow-up do G1 ciclo 1 aplicados. Nenhum gate aprovado aqui.
- **Plano:** W0-08 em [master-plan](../planning/master-plan.md) §4.1.

## Itens (evidência no código, reconferida no HEAD `d42b71a5`; `git diff 72eb471d d42b71a5 -- backend/src` não toca nenhum arquivo citado)

| Item | Evidência | Decisão | Alternativa |
|---|---|---|---|
| Cliente NVIDIA NIM | `core/providers/nvidia_nim.rs:1` com `#![allow(dead_code)]`; `NvidiaNimClient` e `NimLlmProvider` sem nenhum uso fora do arquivo | **Decidido (conservador):** remover só cliente e trait. **Manter** `resolve_nim_base_url`, `providers.nim_base_url`, os campos de `GET /providers/status` (`http_bridge/providers.rs:6-29`) e os ids `nvidia`/`ngc` das credenciais; contrato HTTP não muda | Remover também config, os campos `nim_base_url_configured`/`resolved_nim_base_url` e os ids (muda contrato HTTP e dados de `provider_credentials`). Reversível pelo owner em fatia própria |
| `StubChannelNotifier` | `core/notifications/stub.rs`; só usado em teste (`core/notifications/mod.rs:76`) e reexportado (`:8`) | Remover tipo, reexport e teste. `LogNotifier` fica (usado em `supervisor.rs:20,475`) | Manter como exemplo de canal: rejeitado, canais de conversa têm SDD próprio na Onda 4 |
| Futures | capacidades Futures em `modules/exchanges/capabilities.rs:41-44` e `markets` com Futures (`:35`); `_futures_registered` calculado e descartado (`preflight.rs:98`); `bootstrap.rs:72-73` põe Futures nos markets logados; `resources.rs:86` (`supported_markets`, usado só em log em `preflight.rs:28`); `registry.rs:48-59` (`futures_accounts` e conta Futures em `default_dev_accounts`); `[accounts.futures]` em `core/config/exchanges/binance.toml:10-15`; parsing em `adapters/account_file.rs:60,78` | **Decidido:** remover capacidades, `_futures_registered`, `futures_accounts`, a conta Futures de `default_dev_accounts`, `[accounts.futures]` e o parsing de `futures` (arquivo com `futures` passa a ser erro claro). **`MarketType::Futures` também sai:** conferido com `rg`, além desses pontos só aparece em testes (`resources.rs:99`, `rest.rs:181`, `mod.rs:38`, `capabilities.rs:62`) e no `Display` (`models/mod.rs:34`); não há outro uso real | Manter `MarketType::Futures` e o parsing, removendo só capacidades e o valor descartado; menor mudança, mas mantém config que não faz nada. Reversível pelo owner |
| `notes` de capacidades | `capabilities.rs:48`: "Streams and order lifecycle are planned" — spot já tem WS e ordens testnet | Corrigir o texto junto com a remoção de futures | — |
| `0003_vector_scaffold.sql` | cria `knowledge_embedding_scaffold`; nenhuma referência em Rust | **Manter** e registrar em [core-database-sdd](./core-database-sdd.md) como reservado para memória (pgvector) | Dropar: exige migração nova na próxima sequência livre e a memória provavelmente vai usar pgvector de novo |

## Seams públicos

- NIM: nenhum (`/providers/status` não muda).
- **Futures muda um contrato HTTP público.** `GET /api/v1/exchanges/catalog` devolve `capabilities::catalog()` (`http_bridge/exchanges.rs:42-46`). `MarketType` é `Serialize` + `ToSchema` com `rename_all = "lowercase"` (`modules/exchanges/models/mod.rs:20-25`), e o OpenAPI registra `Capability` (`presentation/http/openapi.rs:9`). A resposta perde `"futures"` em `markets` e os quatro valores `Futures*` em `capabilities`; o schema OpenAPI de `MarketType` e de `Capability` perde os mesmos valores.
  - Hoje o único teste da rota só confere o status 200 (`presentation/http/server.rs:192`); nada prende o corpo.
  - **Nota de contrato público / changelog** (entra no mesmo commit, na seção de mudanças da API do README): "`GET /api/v1/exchanges/catalog`: removidos `futures` de `markets` e `FuturesRestMarketData`, `FuturesStreamMarketData`, `FuturesOrders`, `FuturesUserData` de `capabilities`; nenhum deles era suportado."
  - É remoção de promessa falsa, mas quem lê o catálogo vê a diferença.
- Arquivo de contas: `futures` deixa de ser aceito. É config local, não API; a mensagem de erro deve dizer que futures não é suportado.

## Critérios de aceite

- A1. `rg -n "NvidiaNimClient|NimLlmProvider|StubChannelNotifier|_futures_registered|FuturesOrders|MarketType::Futures" backend/src` sem resultado.
- A2. Nenhum `#![allow(dead_code)]` novo; os removidos não voltam.
- A3. `GET /providers/status` inalterado (teste existente passa).
- A3b. **Teste de snapshot do corpo** de `GET /api/v1/exchanges/catalog`: compara o JSON inteiro com um arquivo esperado versionado (só `spot`, sem `Futures*`, `notes` corrigido). O teste é escrito **antes** da remoção, com o corpo atual (RED ao remover), e atualizado junto com a nota de changelog. O schema OpenAPI de `MarketType` e `Capability` é conferido no mesmo teste ou no teste de OpenAPI existente.
- A4. Config com `[accounts.futures]` falha com mensagem clara; o `binance.toml` distribuído não tem mais a seção.
- A5. `0003` intacta; nota "reservado" no SDD de database.

## Dependências

- Nenhuma para começar. W0-13 e W0-12 mexem em `exchanges`/`orders`; se estiverem em andamento, fazer a parte de futures depois deles para evitar conflito.

## Docs que vão envelhecer

Achados com `rg -il futures backend/docs backend/README.md` (fora de `graphify-out`, dos `wave0-*` e do master plan). O Builder atualiza no mesmo commit:

- `backend/README.md:69` ("The configured futures account is registered but is not used by the monitor").
- [module-catalog](../architecture/module-catalog.md) linha 171 (`capabilities`: "Declarar Futures ou ordens não as habilita").
- [backend-module-reference](../architecture/backend-module-reference.md) linha 76 (`registry`: "incluindo Futures registrados mas não selecionados").
- [backend-module-map-sdd](./backend-module-map-sdd.md) linha 31 (tabela dos submódulos de `exchanges`: "Futures pode estar registrado, mas o monitor seleciona Spot").

Não precisam mudar: [rest-redirect-sdd](./rest-redirect-sdd.md) linha 27 e [backend-corrections-sdd](./backend-corrections-sdd.md) linhas 19 e 78 (futures como não objetivo) e [legacy-file-cleanup-sdd](./legacy-file-cleanup-sdd.md) linha 27 (histórico).

## Riscos

- `core-providers-nim-sdd.md` e `core-database-sdd.md` têm mudanças não commitadas de outra sessão no momento desta redação; o Builder atualiza os dois quando estiverem limpos.
- Alguém com `[accounts.futures]` num arquivo local passa a ter erro no boot; é o comportamento desejado (antes era ignorado em silêncio).

## Validação

- `backend/scripts/verify-backend-gates.sh`.

## Rollout / rollback

- Rollout: próximo build. Rollback: reverter o commit; sem migração.
