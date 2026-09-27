---
title: Auditoria de completude — bots, orders, agents e HTTP
description: Estado verificável dos módulos alvo do goal, gaps, evidências de teste e próximos gates
tags:
  - planning
  - backend
  - modules
  - audit
---

# Auditoria de completude — bots, orders, agents e HTTP

> Revisão: 2026-09-26. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (175 unitários + integração).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | Fundação + ranking + store em memória + HTTP catalog/persist/snapshot | `modules/bots/tests.rs`, `presentation/http/server.rs` | PostgreSQL `BotCatalogStore`, runtime live |
| `modules/orders` | Seam fail-closed + HTTP 503 após risco | `modules/orders/tests.rs`, testes HTTP orders | Adapter exchange, idempotência, auth |
| `modules/agents` | IdentityOnly + HTTP lifecycle + hook monitor | `modules/agents/tests.rs`, rotas agents | Auth owner, PostgreSQL Gate 1 |
| `presentation/http` | OpenAPI ~30 paths, Scalar `/docs` | `openapi.rs`, `server.rs` | Authn/z transversal |

Execução live e produção permanecem bloqueadas até gates de segurança.

## Verificação local

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./scripts/check-import-direction.sh
```

Evidência: **175** testes no binário `bot`, **1** ignorado (`persist_dataset_round_trip`).

## Documentação relacionada

- [module-catalog.md](../architecture/module-catalog.md)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md)


## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Store bots em `ApiState`, HTTP orders/bots/agents | **Parcial** (sem PG/auth/live) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → 175 ok; clippy/fmt/import check | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Fechamento do goal (pendente)

Gate 1 PostgreSQL/auth owner ([SDD draft](../sdd/bots-catalog-persistence-gate1-sdd.md)), adapter real de orders com SDD aprovado, revisão independente AGENTS.md.
