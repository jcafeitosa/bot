---
title: SDD W0-02 — CI verde, testes PG que falham sem banco e DB de teste isolado
description: Fatia Onda 0 que fixa, de forma autossuficiente, os critérios de CI e de testes PostgreSQL (inclui os critérios de T-CI-01/T-CI-02) para a Onda 0
tags:
  - sdd
  - backend
  - ci
  - database
  - wave0
status: draft
---

# SDD W0-02 — CI verde + PG fail-loud + DB de teste isolado

- **Estado:** draft, ciclo 3 do Builder depois do Critic G1 ciclo 2 (REPROVADO). Último ciclo antes de escalar ao owner. Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre os seams antes do primeiro teste.
- **Autossuficiente:** T-CI-01 e T-CI-02 são rastreados fora do repositório e não há documento deles aqui. Os critérios necessários estão copiados neste SDD (seção "Critérios de aceite"); este documento é a referência para W0-02 contar como concluída.
- **Plano:** W0-02 em [master-plan](../planning/master-plan.md) §4.1.
- **Relacionados:** [core-database-sdd](./core-database-sdd.md), [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (item 3, integração PG isolada), P2 em [org-module-sdd](./org-module-sdd.md) (roles/migrations; fora desta fatia).
- **Sem números fixos:** a quantidade de testes PG muda a cada commit. A fonte da verdade é o manifesto `PG_TESTS` de `backend/scripts/run-pg-integration-tests.sh`, verificado por `backend/scripts/assert-pg-integration-manifest.sh`. Este SDD não cita contagens.
- **Seams:** todos **a acordar com o owner** (tabela "Seams"). Nenhum foi acordado ainda.

## Contexto (evidência no código, HEAD `d42b71a5`)

Entre `b8370a75` e `d42b71a5`, `postgres.rs` perdeu 2 linhas (as referências abaixo já estão ajustadas) e `run-pg-integration-tests.sh` mudou no revert. As linhas do script são as do HEAD (`git show HEAD:backend/scripts/run-pg-integration-tests.sh`), não as da cópia de trabalho.

- `.github/workflows/backend-ci.yml:33`: serviço `timescale/timescaledb-ha:pg16`. O código exige PostgreSQL 18: `backend/src/core/database/postgres.rs:11` (`MIN_SERVER_VERSION_NUM = 180_000`), checado em `:65` (`assert_server_version` dentro de `connect_from_url`) e definido em `:72-81`.
- `backend/src/core/persistence/pg_integration.rs:7-15`: `database_for_integration_test` devolve `None` em erro de conexão **ou** de migração; os testes PG fazem `return` cedo. Com pg16 na CI, todo teste PG "passa" sem executar nada. O módulo só compila em teste (`core/persistence/mod.rs:6-7`, `#[cfg(test)] pub mod pg_integration`).
- **Testes que chamam o helper e estão fora do manifesto** (nunca rodam contra PG, nem na CI; conferido com `git grep` das chamadas em `backend/src` × `PG_TESTS` do HEAD):
  - `fetch_stats_returns_zeros_on_empty_outbox` (`core/database/graph_projection_outbox_worker.rs:127`)
  - `pg_order_idempotency_store_unavailable_when_table_missing` (`modules/orders/adapters/pg_idempotency.rs:187`)
  - `graph_admin_list_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_supervision_chain_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_bots_for_agent_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_code_impact_returns_503_without_neo4j_when_postgres_wired` (`presentation/http/http_integration_tests.rs:1630`, `:1707`, `:1779`, `:1854`)
  - `orders_submit_pg_idempotency_store_unavailable_returns_order_store_unavailable` (`http_integration_tests.rs:243`)
  - `provider_credentials_admin_delete_removes_row`, `provider_credentials_admin_upsert_list_masked_never_returns_raw_secret` (`http_integration_tests.rs:1515`, `:1442`)
  - `promote_bot_http_rejects_when_postgres_without_owner_bootstrap` (`presentation/http/state.rs:1540`)
- **Entrada fantasma no manifesto (o contrário também acontece):** `PG_TESTS` lista `persist_dataset_rejects_conflicting_manifest_for_same_id` (`run-pg-integration-tests.sh:22` no HEAD), mas não existe função com esse nome em `backend/src` (a função saiu no revert `afe1f411`; `git grep "fn persist_dataset_rejects_conflicting_manifest_for_same_id" HEAD -- backend/src` vazio). Ou seja, nem todo nome do manifesto chama o helper. Como o filtro é substring e ninguém confere se algum teste executou, essa entrada passa com 0 testes. É o RED natural de C0 e C1.
- **Cópia de trabalho (outra sessão, não commitada; este SDD não edita o script):** `run-pg-integration-tests.sh` está modificado sem stage, com duas entradas novas e `EXPECTED_PG_INTEGRATION_TESTS=31`. Uma delas, `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`, é **outra entrada fantasma**: não há função com esse nome nem no HEAD nem na cópia de trabalho. A outra, `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure`, só existe no `pg_idempotency.rs` que está em stage (`:267` da cópia de trabalho), não no HEAD. C0 pega os dois casos e a regra de remoção (Decisão 3) cobre a entrada sem função.
- `run-pg-integration-tests.sh:13-16`: único guard é a URL conter `trading_bot`, mesmo nome do banco de runtime (`postgres.rs:57-58` só aceita esse nome). Nada impede rodar os testes contra o banco de dev/operacional.
- O script roda cada teste por filtro de **substring** (`cargo test --locked --bin bot "${test_name}"`, laço `:57-60` no HEAD); um filtro que casa 0 testes (renomeado) ou mais de 1 passa sem erro. O `assert-pg-integration-manifest.sh` só compara o tamanho de `PG_TESTS` com `EXPECTED_PG_INTEGRATION_TESTS`.
- **Pontos de entrada de conexão PG em produção** (todos chegam a `PostgresDatabase::connect_from_url`, `postgres.rs:55-70`, que é o único `PgPoolOptions::new()` de `backend/src`, `:60`; `connect_from_env`, `:48-53`, delega a ele; `Database::connect_from_*`, `core/persistence/mod.rs:27-33`, também):
  - `core/database/bundle.rs:53` (`bootstrap_http_api`, usado pelo `serve`) e `:123` (`optional_postgres_for_monitor_supervisor_snapshot`, que [W0-03](./wave0-03-monitor-opt-out-sem-pg-sdd.md) remove);
  - `core/database/monitor_bootstrap.rs:36` (`connect_postgres_for_monitor`) e `:68` (`postgres_for_cli_persist`);
  - `core/database/graph_projection_cli.rs:79` (drain do outbox);
  - `modules/orders/cli.rs:68` (purge de retenção).
  - Hoje `bundle.rs:53-64` e `:121-142` só registram `warn` em erro de conexão e seguem sem PG.
- `backend-ci.yml:22, 47`: `dtolnay/rust-toolchain@stable` sem `rust-toolchain.toml` no repo; o clippy muda com o toolchain.
- Imagem de dev pinada por digest: `docker-compose.bot.yml:12` = `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840` (comentário `:3`: PostgreSQL 18.6 / TimescaleDB 2.30.1). A CI não usa essa imagem. O compose usa `POSTGRES_DB=bot_agents` e `PGDATA` próprio; a CI precisa de `POSTGRES_DB=trading_bot`.

## Contradições doc × código

- O comentário de `run-pg-integration-tests.sh:2` cita um número de testes diferente de `EXPECTED_PG_INTEGRATION_TESTS`; o número muda a cada commit, por isso este SDD não o repete.
- Docs de status já citaram contagens PG; contagem não prova execução (ver C1).

## Decisão

1. **Modo "PG obrigatório":** `BOT_PG_INTEGRATION_REQUIRED=1`, lido em `core/config` (exigido pelo guard de env de `verify-backend-gates.sh`). Nesse modo, o helper **falha o teste** (panic com mensagem estável, sem URL) quando falta `DATABASE_URL`, a conexão falha, a versão é < 18, a migração falha ou o marcador de teste (item 2) não existe; quando tudo dá certo, imprime uma linha estável (nome proposto `PG_INTEGRATION_HELPER_OK`). Sem o modo, o `cargo test` local continua pulando, como hoje. O script e a CI sempre ligam o modo. **Limite conhecido:** o modo obrigatório não faz falhar um teste que retorna cedo **antes** de chamar o helper (ex.: checagem de Neo4j ou de credencial antes do helper); por isso C1 exige a linha estável do helper, e não só "1 passed".
2. **DB de teste isolado por marcador no banco**, não só pelo nome: tabela `bot_test_database_marker` com uma linha, criada **fora** das migrações e **fora** do binário.
   - **Na CI:** um passo do job `postgres-integration` em `backend-ci.yml`, depois do health check do service container e antes de `run-pg-integration-tests.sh`, cria o marcador com `psql "$DATABASE_URL"` a partir do runner (o `ubuntu-latest` traz cliente `psql`; se não trouxer, o mesmo SQL roda via `docker exec` no container do service). **Quem cria: o workflow**, e só ele.
   - **Local:** um script de setup de PG descartável (nome proposto `backend/scripts/prepare-pg-test-db.sh`) roda o mesmo SQL; o desenvolvedor o executa de propósito contra um banco descartável.
   - O helper recusa migrar/rodar sem o marcador.
   - **Runtime com marcador recusa subir, num lugar só:** a checagem fica em `PostgresDatabase::connect_from_url` (`postgres.rs:55-70`), logo depois de `assert_server_version` (`:65`), e devolve um erro novo (nome proposto `DatabaseError::TestDatabaseMarkerPresent`). Como os seis pontos de entrada do Contexto passam por essa função, todos ficam cobertos sem código repetido. O helper de teste usa um construtor `#[cfg(test)]` próprio que faz a checagem inversa (exige o marcador). Esse erro é **fatal** em todos os pontos de entrada: nos dois caminhos de `bundle.rs` que hoje só dão `warn` e seguem sem PG, essa variante encerra o processo com exit ≠ 0 em vez de seguir sem banco. (Decisão por recomendação: o runtime nunca roda contra banco de teste.)
3. **Manifesto verificado estaticamente, com remoção de entradas sem função:** `assert-pg-integration-manifest.sh` (já em Python) passa a comparar o **conjunto** de funções de teste que chamam `database_for_integration_test()` com o conjunto `PG_TESTS` (método de descoberta em "Descoberta do conjunto PG", abaixo); qualquer diferença falha o gate e lista os nomes. Exceções só numa lista `PG_TEST_EXCEPTIONS` no próprio script, uma por linha, cada uma com justificativa. `EXPECTED_PG_INTEGRATION_TESTS` deixa de ser número digitado à mão (vira derivado do manifesto ou sai).
   - **Entrada sem função é removida por W0-02 (o código é o fato).** Isso vale para `persist_dataset_rejects_conflicting_manifest_for_same_id` e para qualquer outra entrada do manifesto sem função de teste correspondente em `backend/src` no momento da implementação (inclusive `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`, se a entrada da cópia de trabalho chegar ao `main` sem a função). Se o owner restaurar o teste de `persist_dataset` no [W0-09](./wave0-09-v18-verificacao-sdd.md) (item 5), o C0 obriga a readicioná-lo ao manifesto no mesmo PR, porque a função passa a chamar o helper. Com isso o GREEN de W0-02 não depende da decisão de W0-09.
4. **Cada teste roda exatamente uma vez:** o script resolve cada nome do manifesto pelo `cargo test --bin bot -- --list` (exatamente um caminho terminando em `::<nome>`, senão falha), roda com `--exact <caminho completo> --nocapture` e `BOT_PG_INTEGRATION_REQUIRED=1`, e exige na saída daquele teste `1 passed; 0 failed; 0 ignored` **e** a linha `PG_INTEGRATION_HELPER_OK` uma vez. "1 passed" com a linha do helper significa "executou contra PG".
5. **CI com a imagem do dev:** o service usa o mesmo digest de `docker-compose.bot.yml:12`, com `POSTGRES_DB=trading_bot`. O Critic **não verificou** se essa imagem serve para service container do GitHub (usuário, `PGDATA`, extensões habilitadas); C4 exige a prova.
6. **Toolchain fixado** em `rust-toolchain.toml` (versão e componentes `rustfmt`, `clippy`), usado local e na CI.

### Descoberta do conjunto PG (C0)

- **Escolhido (o mais simples que é correto): parser estático + proibição de wrapper.** O `assert-pg-integration-manifest.sh` varre `backend/src/**/*.rs`, acha cada chamada a `database_for_integration_test()` fora de `pg_integration.rs`, sobe até o `fn` que a contém e lê os atributos logo acima dele. Se o `fn` tem `#[test]` ou `#[tokio::test…]`, o nome entra no conjunto PG. Se **não** tem atributo de teste (ou seja, é um wrapper), o script **falha** e lista o arquivo e a linha. Assim nenhum teste PG escapa por wrapper, e testes com retorno cedo antes do helper também entram no conjunto, porque a análise é estática. No HEAD, toda chamada ao helper está dentro de uma função de teste (conferido com `git grep -B6`), então a proibição não quebra nada.
- **O que o estático não prova, e quem cobre:** que o teste de fato chega ao helper em tempo de execução. Isso é coberto por C1 (linha `PG_INTEGRATION_HELPER_OK` obrigatória por teste do manifesto).
- **Alternativa considerada:** descoberta dinâmica (rodar todos os testes do bin com `BOT_PG_INTEGRATION_REQUIRED=1` e sem `DATABASE_URL`; os que falham com a mensagem estável do helper formam o conjunto PG). Pega wrappers sem regra extra, mas não pega teste que retorna cedo antes do helper e roda a suíte inteira no gate. Rejeitada como método principal; fica como verificação manual opcional.

**Alternativa considerada:** banco de teste com outro nome (ex.: `trading_bot_test`). Mais visível, mas exige afrouxar o guard de nome do runtime (`postgres.rs:57-58`), que hoje é uma proteção. Rejeitada nesta fatia.

## Seams (a acordar com o owner antes do TDD)

| Seam | Proposta |
|---|---|
| Variável de modo | `BOT_PG_INTEGRATION_REQUIRED=1`, lida em `core/config` |
| Linha estável do helper | `PG_INTEGRATION_HELPER_OK` (impressa só em modo obrigatório, sem URL) |
| Marcador | tabela `bot_test_database_marker` (1 linha), criada pelo workflow na CI e por `prepare-pg-test-db.sh` local; nunca por migração |
| Runtime com marcador | recusa subir; checagem única em `PostgresDatabase::connect_from_url`; erro `DatabaseError::TestDatabaseMarkerPresent`, fatal em todos os pontos de entrada |
| Manifesto | igualdade de conjuntos (parser estático, sem wrapper) × `PG_TESTS`, com `PG_TEST_EXCEPTIONS` justificadas; entrada sem função é removida |
| Execução | `--list` + `--exact --nocapture` + parse de `1 passed; 0 failed; 0 ignored` e da linha do helper, por teste |

## Critérios de aceite (conclusão da Onda 0)

Critérios de T-CI-01 (toolchain e job `rust`) e T-CI-02 (PG 18 e manifesto), copiados aqui para este SDD ser autossuficiente:

- **C0 (manifesto completo).** `assert-pg-integration-manifest.sh` falha se o conjunto de testes que chamam o helper (parser estático acima) diferir de `PG_TESTS`, e falha se alguma chamada ao helper estiver num `fn` sem atributo de teste. Provas registradas na entrega: (a) um teste fora do manifesto faz o script falhar; (b) um wrapper sem atributo de teste faz o script falhar; (c) uma entrada sem função (a fantasma do HEAD) faz o script falhar. Os dez testes listados no Contexto entram no manifesto ou em `PG_TEST_EXCEPTIONS` com justificativa; a recomendação é que os dez entrem, porque todos precisam de PG ligado (os `graph_admin_*` também exigem Neo4j desligado, que é o padrão da CI). As entradas sem função são removidas (Decisão 3).
- **C1 (T-CI-02).** Cada teste do manifesto roda exatamente uma vez, com `BOT_PG_INTEGRATION_REQUIRED=1`, `--exact`, `--nocapture`, `1 passed; 0 failed; 0 ignored` e a linha `PG_INTEGRATION_HELPER_OK` conferidos por teste; nome que resolve para 0 ou mais de 1 teste falha o script; teste que termina sem a linha do helper (retorno cedo antes do helper) falha o script. Além disso, o total de testes executados com `1 passed` é igual ao número de entradas do manifesto. Prova de RED: no HEAD, a entrada fantasma `persist_dataset_rejects_conflicting_manifest_for_same_id` faz C0 e C1 falharem.
- **C2.** Em modo obrigatório, sem `DATABASE_URL`, com PG < 18, com migração quebrada ou sem marcador, o job PG falha (prova: execução registrada de cada caso).
- **C3.** Helper recusa banco sem marcador (teste unitário sem PG real para a decisão; teste PG para o caminho feliz). Runtime recusa banco com marcador: teste PG chama `PostgresDatabase::connect_from_url` contra o banco com marcador e espera `TestDatabaseMarkerPresent`; teste do `serve`/`bootstrap_http_api` contra o mesmo banco espera exit ≠ 0 (não `warn` seguido de "sem PG"). Guard estático: `rg 'PgPoolOptions' backend/src` só em `core/database/postgres.rs`, para que nenhum ponto de entrada novo crie pool sem passar pela checagem.
- **C4 (T-CI-02).** Job `postgres-integration` com a imagem `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`; o log da execução mostra `server_version_num` ≥ 180000 e as extensões exigidas instaladas. Os dois jobs de `backend-ci.yml` verdes em `main`; link da execução registrado na entrega.
- **C5 (T-CI-01).** `rust-toolchain.toml` presente; `cargo fmt --check` e `cargo clippy -D warnings` limpos; `verify-backend-gates.sh` verde local e na CI com o mesmo toolchain.

## Dependências

- Nenhuma fatia W0 bloqueia. O GREEN não depende de W0-09 (Decisão 3). É pré-requisito de G4 para todas as outras fatias W0 com teste PG (W0-03, W0-05, W0-07, W0-09, W0-12).

## Riscos

- Testes que hoje "passam" podem começar a falhar de verdade quando o PG existir, inclusive os dez que entram no manifesto; é o objetivo, mas pode atrasar outras fatias.
- Mudar o helper afeta todos os testes que o chamam; mudança mecânica, revisada pelo Critic.
- A proibição de wrapper obriga futuros testes PG a chamar o helper direto; é intencional (mantém o parser simples).
- Tornar o erro de marcador fatal em `bundle.rs` muda o comportamento "segue sem PG" só para esse erro; os outros erros de conexão continuam como hoje.
- A imagem do dev pode não subir como service container (usuário/`PGDATA`); se não subir, C4 falha e o Builder registra a imagem PG 18 alternativa (também por digest) com a justificativa.
- O parse da saída do `cargo test` depende do formato do libtest; fixado pelo toolchain (C5).

## Validação

- `backend/scripts/verify-backend-gates.sh` e `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável com marcador; `assert-pg-integration-manifest.sh` continua no gate.

## Rollout / rollback

- Rollout: só CI e scripts de teste (mais a recusa do runtime com marcador); nenhum deploy. Rollback: reverter workflow/scripts volta ao estado atual (testes PG vazios).
