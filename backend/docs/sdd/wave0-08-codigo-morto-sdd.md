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

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1).
- **Plano:** W0-08 em [master-plan](../planning/master-plan.md) §4.1.

## Itens (evidência no código, HEAD `72eb471d`)

| Item | Evidência | Decisão | Alternativa |
|---|---|---|---|
| Cliente NVIDIA NIM | `core/providers/nvidia_nim.rs:1` com `#![allow(dead_code)]`; `NvidiaNimClient` e `NimLlmProvider` sem nenhum uso fora do arquivo | Remover cliente e trait. **Manter** `resolve_nim_base_url` e `providers.nim_base_url`, que alimentam `GET /providers/status` (`http_bridge/providers.rs:6-29`), e os ids `nvidia`/`ngc` das credenciais | Remover também config, os campos `nim_base_url_configured`/`resolved_nim_base_url` e os ids: muda contrato HTTP e dados de `provider_credentials`; só com decisão do owner |
| `StubChannelNotifier` | `core/notifications/stub.rs`; só usado em teste (`core/notifications/mod.rs:76`) e reexportado (`:8`) | Remover tipo, reexport e teste. `LogNotifier` fica (usado em `supervisor.rs`) | Manter como exemplo de canal: rejeitado, canais de conversa têm SDD próprio na Onda 4 |
| Futures | capacidades Futures declaradas em `modules/exchanges/capabilities.rs:41-44` e `markets` com Futures (`:35`); `_futures_registered` calculado e descartado (`modules/exchanges/preflight.rs:98`); `bootstrap.rs:72-73` e `resources.rs:86,99` listam Futures; `[accounts.futures]` em `core/config/exchanges/binance.toml:10-15`; parsing em `adapters/account_file.rs:60,78` | Não há fatia futures planejada: remover capacidades, `_futures_registered`, `futures_accounts`, `[accounts.futures]` e o parsing de `futures` (arquivo com `futures` passa a ser erro claro) | Manter `MarketType::Futures` e o parsing, removendo só capacidades e o valor descartado; menor mudança, mas mantém config que não faz nada |
| `notes` de capacidades | `capabilities.rs:48`: "Streams and order lifecycle are planned" — spot já tem WS e ordens testnet | Corrigir o texto junto com a remoção de futures | — |
| `0003_vector_scaffold.sql` | cria `knowledge_embedding_scaffold`; nenhuma referência em Rust | **Manter** e registrar em [core-database-sdd](./core-database-sdd.md) como reservado para memória (pgvector) | Dropar: exige migração nova na próxima sequência livre e a memória provavelmente vai usar pgvector de novo |

## Seams públicos

- Nenhum no caminho recomendado. Se o owner escolher remover os campos NIM de `/providers/status`, é mudança de contrato HTTP (OpenAPI e teste `status_resolves_default_nim_base_url`).
- Arquivo de contas: `futures` deixa de ser aceito. É config local, não API; a mensagem de erro deve dizer que futures não é suportado.

## Critérios de aceite

- A1. `rg -n "NvidiaNimClient|NimLlmProvider|StubChannelNotifier|_futures_registered|FuturesOrders" backend/src` sem resultado.
- A2. Nenhum `#![allow(dead_code)]` novo; os removidos não voltam.
- A3. `GET /providers/status` inalterado (teste existente passa).
- A4. Config com `[accounts.futures]` falha com mensagem clara; o `binance.toml` distribuído não tem mais a seção.
- A5. `0003` intacta; nota "reservado" no SDD de database.

## Dependências

- Nenhuma para começar. W0-13 e W0-12 mexem em `exchanges`/`orders`; se estiverem em andamento, fazer a parte de futures depois deles para evitar conflito.

## Riscos

- `core-providers-nim-sdd.md` e `core-database-sdd.md` têm mudanças não commitadas de outra sessão no momento desta redação; o Builder atualiza os dois quando estiverem limpos.
- Alguém com `[accounts.futures]` num arquivo local passa a ter erro no boot; é o comportamento desejado (antes era ignorado em silêncio).

## Validação

- `backend/scripts/verify-backend-gates.sh`.

## Rollout / rollback

- Rollout: próximo build. Rollback: reverter o commit; sem migração.
