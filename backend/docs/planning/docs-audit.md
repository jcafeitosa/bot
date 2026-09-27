---
title: Auditoria completa de docs
description: Inventário físico e OpenKnowledge da documentação, conexões, duplicidades e artefatos
tags:
  - audit
  - documentation
  - backend
  - openknowledge
---

# Auditoria completa de `docs/`

> Revisão: 2026-09-27. A auditoria combina a árvore física de `backend/docs` com a validação do OpenKnowledge.

## Resultado

- Auditoria OpenKnowledge: **50 documentos, zero problemas de links**.
- Grafo: **zero links mortos e zero órfãos**.
- Conflitos rastreados: **zero**.
- Índice canônico: `backend/docs/index.md`.
- Categorias canônicas: arquitetura, operações, planejamento, propostas, referência, pesquisa, fontes externas e SDD.

## Estrutura canônica

- `architecture/`: visão de módulos, catálogo completo e integrações.
- `operations/`: runbook operacional.
- `planning/`: roadmap, plano de execução, auditorias e lacunas de módulos.
- `proposals/`: propostas arquiteturais ainda sujeitas a decisão.
- `reference/`: CLI/configuração e matriz de testes.
- `research/`: pesquisa de capacidades de agentes.
- `sdd/`: especificações de design, gates e correções.
- `.codex/skills/graphify/`: skill operacional, mantida fora da documentação de produto.
- `external-sources/`: capturas preservadas de fontes externas.
- `AGENTS.md`: governança do projeto.
- `.codex/skills/graphify/`: skill operacional, mantida fora da documentação de produto.

## Artefatos verificados

### Fontes externas

As capturas preservadas de Grok Bot, OpenBot, OpenClaw e Meta Model API estão conectadas ao índice por links sob `external-sources/`. O HTML da Meta é um artefato de fonte preservada, não uma página de produto do backend.

### Duplicidade física (resolvida)

A árvore acidental `backend/docs/backend/docs/` (cópias de pesquisa, proposta, fontes externas e HTML duplicado da Meta) foi removida. As capturas Grok/OpenBot/OpenClaw foram consolidadas em `backend/docs/external-sources/`; links do índice apontam para essa pasta canônica.

## Correções aplicadas nesta auditoria

- Conectadas as fontes preservadas ao índice.
- Corrigidos os caminhos das fontes para a árvore canônica.
- Sincronizados os status de T-05, T-07, T-10 e T-15 no roadmap.
- Atualizada a contagem da documentação para 50 documentos.
- Corrigidas sete referências quebradas em README, scripts, `.env`, providers e SDD HTTP admin.
- Confirmada a remoção da árvore duplicada `backend/docs/backend/docs/`.
- Mantidas as referências entre catálogo, matriz de testes, integrações, runbook, SDDs e planejamento.

## Critério de saúde

A documentação está saudável quando:

1. todo documento canônico aparece no índice ou é alcançado por um documento indexado;
2. não há links mortos;
3. não há órfãos;
4. SDD, roadmap, catálogo e matriz de testes têm o mesmo status;
5. fontes externas preservadas mantêm proveniência e não são confundidas com implementação;
6. cópias físicas fora da árvore canônica são removidas por operação autorizada e verificável.

## Auditoria do backend (2026-09-27)

- OpenKnowledge auditou **51 documentos** depois da atualização desta auditoria; nenhum link quebrado foi encontrado. A auditoria de lint individual retornou `ran: []`, portanto só há evidência de links.
- Graphify foi atualizado após identificar grafo desatualizado. A extração Rust foi refeita; 10 arquivos SQL ficaram fora do grafo porque `tree_sitter_sql` não está instalado.
- Gate canônico `./scripts/verify-backend-gates.sh`: **passou** (snapshot 2026-09-27) — **492** testes no bin `bot`, **0** ignorados; `assert-completeness-evidence.sh` + manifesto PG **26**; `cargo test --bin bot http_integration -- --test-threads=1` → **60** passed. Evidência machine-readable: [modules-completeness-evidence.json](./modules-completeness-evidence.json).
- Parecer de segurança: `BOT_HTTP_ADMIN_TOKEN` é opcional por contrato. Sem token, as rotas mutantes permitem requests sem bearer; isso está documentado como comportamento dev/local, mas deployments acessíveis por rede precisam configurar o token ou restringir o listener.
- **Alto — idempotência PG (mitigado 2026-09-27):** `PgOrderIdempotencyStore::try_claim` grava a chave antes de `submit_order_http`; em falha de execução, `release_claim` permite retry. Ver `state.rs` + teste `pg_order_idempotency_try_claim_and_release`.
- **Médio — persistência de dataset pode confirmar conteúdo divergente** (`src/core/persistence/mod.rs:51-82`): manifesto e candles usam `ON CONFLICT DO NOTHING`; não há comparação de valores existentes antes de retornar sucesso.
- **Mitigado (2026-09-27) — falha PG de idempotência:** `orders_pg_store_error` → `OrdersError::StoreUnavailable` → HTTP **503** `order_store_unavailable` (`error.rs`, `pg_store_error.rs`); teste HTTP `orders_submit_pg_idempotency_store_unavailable_returns_order_store_unavailable`.
- PG de domínio: `run-pg-integration-tests.sh` exige `DATABASE_URL` com `trading_bot` e manifesto **22/22** (`EXPECTED_PG_INTEGRATION_TESTS=22`); CI job `postgres-integration`. Gate padrão `verify-backend-gates.sh` não executa PG (só `assert-pg-integration-manifest.sh`).
- Baseline bin `bot` e `http_integration` devem seguir a linha `OK:` de `verify-backend-gates.sh` e `cargo test --bin bot http_integration -- --test-threads=1` (ver [test-matrix](../reference/test-matrix.md), [modules-completeness-audit](./modules-completeness-audit.md)).
- CRUD HTTP admin `/api/v1/admin/provider-credentials*` (mascarado, bearer quando `BOT_HTTP_ADMIN_TOKEN`; **503** sem PG) — [provider-credentials-db-sdd](../sdd/provider-credentials-db-sdd.md); testes `provider_credentials_admin_*` em `http_integration_tests.rs`. `provider_credentials.secret` é armazenado como texto; criptografia em repouso segue como follow-up e exige proteção de acesso/backup.
- O seed SQL de exemplo executa upsert substituindo segredos existentes; as instruções avisam que é arquivo de exemplo e deve ser executado somente após substituir placeholders localmente.
- Inventário: **709 arquivos Git** sob `backend` (inclui 104 arquivos vendor e 309 itens Graphify). Código próprio e documentação foram analisados por categorias; vendor não foi revisado linha a linha. Graphify não processou SQL.
- Alterações locais já existentes foram preservadas; esta auditoria atualizou apenas o documento via OpenKnowledge e os artefatos Graphify gerados. Nenhuma correção de código foi aplicada.
