---
title: SDD W0-02 — CI verde, testes PG que falham sem banco e DB de teste isolado (stub)
description: Stub da fatia Onda 0 que referencia T-CI-01/T-CI-02 e fixa os critérios de conclusão da Onda 0 para CI e testes PostgreSQL
tags:
  - sdd
  - backend
  - ci
  - database
  - wave0
status: draft
---

# SDD W0-02 — CI verde + PG fail-loud + DB de teste isolado (stub)

- **Estado:** stub (proporcional). O trabalho de CI já está em andamento como **T-CI-01** e **T-CI-02**, rastreados fora do repositório. Este documento não substitui esses itens; só fixa o que precisa ser verdade para W0-02 contar como concluída. Nenhum gate aprovado.
- **Plano:** W0-02 em [master-plan](../planning/master-plan.md) §4.1.
- **Relacionados:** [core-database-sdd](./core-database-sdd.md), [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (item 3, integração PG isolada), P2 em [org-module-sdd](./org-module-sdd.md) (roles/migrations; fora desta fatia).

## Contexto (evidência no código, HEAD `b2001a7c`)

- `.github/workflows/backend-ci.yml:33`: serviço `timescale/timescaledb-ha:pg16`. `backend/src/core/database/postgres.rs:11, 65, 72-78`: o código exige PostgreSQL 18 (`MIN_SERVER_VERSION_NUM = 180_000`) e recusa conectar abaixo disso.
- `backend/src/core/persistence/pg_integration.rs:7-15`: `database_for_integration_test` devolve `None` em erro de conexão **ou** de migração; os testes PG fazem `return` cedo (38 usos). Com pg16 na CI, todo teste PG "passa" sem executar nada.
- `backend/scripts/run-pg-integration-tests.sh:13-16`: único guard é a URL conter `trading_bot` — mesmo nome do banco de runtime (`postgres.rs:58` só aceita esse nome). Nada impede rodar os testes contra o banco de dev/operacional.
- O script roda cada teste por filtro de nome (`cargo test --bin bot "<nome>"`); um filtro que casa 0 testes (renomeado) ou mais de 1 passa sem erro.
- `backend-ci.yml:22, 47`: `dtolnay/rust-toolchain@stable` sem `rust-toolchain.toml` no repo; o clippy muda com o toolchain.
- Imagem de dev pinada por digest em `docker-compose.bot.yml:12`; a CI não usa a mesma.

## Contradições doc × código

- Comentário do script diz "Runs 24 PostgreSQL domain integration tests"; o manifesto tem 29 (`EXPECTED_PG_INTEGRATION_TESTS=29`, `scripts/run-pg-integration-tests.sh:51`).
- Docs de status já citaram "PG 27/27" e "28"; o número não prova execução (ver critério abaixo).

## Decisão

1. Modo "PG obrigatório" ligado pelo script/CI (nome proposto `BOT_PG_INTEGRATION_REQUIRED=1`, lido em `core/config`): nesse modo, banco ausente, versão < 18 ou migração falha **faz o teste falhar** (panic com mensagem estável sem URL). Sem o modo, o `cargo test` local continua pulando, como hoje.
2. DB de teste isolado por **marcador no banco** (ex.: tabela/linha criada só pelo setup do banco descartável) e não só pelo nome: o helper de teste recusa migrar/rodar sem o marcador.
3. CI com a mesma imagem PG 18 + Timescale + pgvector do dev (digest), e toolchain fixado em `rust-toolchain.toml` (T-CI-01/T-CI-02).

**Alternativa considerada:** banco de teste com outro nome (ex.: `trading_bot_test`). Mais visível, mas exige afrouxar o guard de nome do runtime (`postgres.rs:58`), que hoje é uma proteção. Rejeitada nesta fatia.

## Seams para acordo antes do TDD

- Nome da variável de modo obrigatório e onde é lida (`core/config`, exigido pelo guard de `verify-backend-gates.sh`).
- Forma do marcador de DB de teste e quem o cria (script de setup do PG descartável).
- Comportamento do runtime (`serve`/monitor) ao encontrar o marcador: recusar subir (proposto) ou só avisar.

## Critérios de aceite (conclusão da Onda 0)

- C1 (T-CI-02). **Cada teste do manifesto roda exatamente uma vez e o script falha caso contrário:** o script confere, para cada nome, que o `cargo test` executou 1 teste (não 0, não mais de 1) e não pulou; não basta bater a contagem do manifesto.
- C2. Em modo obrigatório, sem `DATABASE_URL`, com PG < 18 ou com migração quebrada, o job PG falha (prova: execução registrada com cada caso).
- C3. Helper recusa banco sem marcador de teste (teste unitário sem PG real para a decisão; teste PG para o caminho feliz).
- C4. Os dois jobs de `backend-ci.yml` verdes em `main` com PG 18; link da execução registrado na entrega.
- C5. `rust-toolchain.toml` presente; `verify-backend-gates.sh` verde local e na CI com o mesmo toolchain.

## Dependências

- Nenhuma fatia W0 bloqueia. É pré-requisito de G4 para todas as outras fatias W0 com teste PG (W0-03, W0-05, W0-07, W0-09, W0-12).

## Riscos

- Testes que hoje "passam" podem começar a falhar de verdade quando o PG existir; isso é o objetivo, mas pode atrasar outras fatias.
- Mudar o comportamento do helper afeta 38 testes; mudança mecânica, revisada pelo Critic.

## Validação

- `backend/scripts/verify-backend-gates.sh` e `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável; `assert-pg-integration-manifest.sh` continua no gate.

## Rollout / rollback

- Rollout: só CI e scripts de teste; nenhum deploy. Rollback: reverter workflow/scripts volta ao estado atual (testes PG vazios).
