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

- **Estado:** draft, revisão do SDD após rejeição do Critic G1 ciclo 2. O owner aprovou os seams públicos listados abaixo; a revisão técnica independente G1 permanece pendente. Nenhum gate foi aprovado, e nenhuma implementação está autorizada por este documento.
- **Autossuficiente:** T-CI-01 e T-CI-02 são rastreados fora do repositório e não há documento deles aqui. Os critérios necessários estão copiados neste SDD (seção "Critérios de aceite"); este documento é a referência para W0-02 contar como concluída.
- **Plano:** W0-02 em [master-plan](../planning/master-plan.md) §4.1.
- **Relacionados:** [core-database-sdd](./core-database-sdd.md), [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (item 3, integração PG isolada), P2 em [org-module-sdd](./org-module-sdd.md) (roles/migrations; fora desta fatia).
- **Sem números fixos:** a quantidade de testes PG muda a cada commit. A fonte da verdade é o manifesto `PG_TESTS` de `backend/scripts/run-pg-integration-tests.sh`, verificado por `backend/scripts/assert-pg-integration-manifest.sh`. Este SDD não cita contagens.
- **Seams:** PostgreSQL 18+ com extensões `timescaledb` e `vector` obrigatórias; modo `BOT_PG_INTEGRATION_REQUIRED=1` fail-loud; marcador de teste criado só pelo setup local/CI e recusado em runtime; manifesto comparado com funções reais; cada item resolvido e executado exatamente uma vez com prova do helper. Owner aprovou esses seams; revisão técnica independente G1 pendente.

## Contexto (evidência no código, HEAD `d42b71a5`)

Entre `b8370a75` e `d42b71a5`, `postgres.rs` perdeu 2 linhas (as referências abaixo já estão ajustadas) e `run-pg-integration-tests.sh` mudou no revert. As linhas do script são as do HEAD (`git show HEAD:backend/scripts/run-pg-integration-tests.sh`), não as da cópia de trabalho.

- `.github/workflows/backend-ci.yml:33`: o serviço usa `timescale/timescaledb-ha:pg16`, incompatível com o mínimo exigido pelo código (PostgreSQL 18): `backend/src/core/database/postgres.rs:11` (`MIN_SERVER_VERSION_NUM = 180_000`), checado em `:65` (`assert_server_version` dentro de `connect_from_url`) e definido em `:72-81`. Atualizar apenas a imagem do workflow não é suficiente: o helper atualmente converte erros de conexão ou migração em `None`, e o manifesto/runner não comprovam seleção nem execução de todos os testes. A imagem exata aprovada para CI, seu digest e suporte simultâneo a service container, PostgreSQL 18+ e às extensões exigidas permanecem desconhecidos e são gate bloqueante (C4).
- `backend/src/core/persistence/pg_integration.rs:7-15`: `database_for_integration_test` devolve `None` em erro de conexão **ou** de migração; os testes PG fazem `return` cedo. Sob o serviço pg16 incompatível, os testes não alcançam uma integração bem-sucedida; a saída atual pode ainda reportar sucesso por retorno antecipado. O módulo só compila em teste (`core/persistence/mod.rs:6-7`, `#[cfg(test)] pub mod pg_integration`).
- **Testes que chamam o helper e estão fora do manifesto no HEAD observado** (não são selecionados pelo runner PG conforme o manifesto atual; conferido com `git grep` das chamadas em `backend/src` × `PG_TESTS`):
  - `fetch_stats_returns_zeros_on_empty_outbox` (`core/database/graph_projection_outbox_worker.rs:127`)
  - `pg_order_idempotency_store_unavailable_when_table_missing` (`modules/orders/adapters/pg_idempotency.rs:187`)
  - `graph_admin_list_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_supervision_chain_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_bots_for_agent_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_code_impact_returns_503_without_neo4j_when_postgres_wired` (`presentation/http/http_integration_tests.rs:1630`, `:1707`, `:1779`, `:1854`)
  - `orders_submit_pg_idempotency_store_unavailable_returns_order_store_unavailable` (`http_integration_tests.rs:243`)
  - `provider_credentials_admin_delete_removes_row`, `provider_credentials_admin_upsert_list_masked_never_returns_raw_secret` (`http_integration_tests.rs:1515`, `:1442`)
  - `promote_bot_http_rejects_when_postgres_without_owner_bootstrap` (`presentation/http/state.rs:1540`)
- **Entrada fantasma no manifesto (o contrário também acontece):** `PG_TESTS` lista `persist_dataset_rejects_conflicting_manifest_for_same_id` (`run-pg-integration-tests.sh:22` no HEAD), mas não existe função com esse nome em `backend/src` (a função saiu no revert `afe1f411`; `git grep "fn persist_dataset_rejects_conflicting_manifest_for_same_id" HEAD -- backend/src` vazio). Ou seja, nem todo nome do manifesto chama o helper. Como o filtro é substring e ninguém confere se algum teste executou, essa entrada passa com 0 testes. É o RED natural de C0 e C1.
- **Manifesto PG observado na sessão de revisão:** contém 31 entradas. Duas entradas são fantasmas, sem funções correspondentes: `persist_dataset_rejects_conflicting_manifest_for_same_id` e `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`. A entrada `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure` está presente apenas na cópia de trabalho staged de `pg_idempotency.rs`, não no HEAD observado. O verificador deve descobrir e comparar funções reais com o manifesto, falhando em qualquer divergência; entradas sem função não podem contar como testes selecionados/executados.
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

## Evidência de validação disponível (baseline desta revisão)

- `cargo check --all-targets` concluiu com sucesso neste ciclo de verificação.
- A gate canônica esperava 520 evidências, mas o registro encontrado indicava 519; `verify-backend-gates.sh` falhou na asserção de contagem. Isso não é falha atribuída ao desenho W0-02 nem valida o caminho PG; registrar/destravar a fonte do descompasso antes do G4.
- Smoke local de API passou para healthz/readyz/OpenAPI, incluindo 42 rotas. É evidência de baseline geral, não de integração PostgreSQL.
- Não há evidência aqui de que imagem compatível PG 18+ e extensões tenham sido iniciadas como service container GitHub; permanece gate C4. Nenhum destes resultados altera o status draft ou aprova G1.

## Decisão / desenho proposto (sujeito a aprovação G1)

A entrega de implementação é uma unidade coerente: mudança do workflow GitHub Actions e das Rust helpers/scripts de teste para fazer o job PG realmente exercitar testes e falhar de forma observável. Não alterar outros workflows nem fazer deploy. O runtime deve recusar marcador de teste por segurança de ambiente.

1. **Modo "PG obrigatório":** `BOT_PG_INTEGRATION_REQUIRED=1`, lido em `core/config` (exigido pelo guard de env de `verify-backend-gates.sh`). Nesse modo, o helper **falha o teste** (panic com mensagem estável, sem URL) quando falta `DATABASE_URL`, a conexão falha, a versão é < 18, a migração falha ou o marcador de teste (item 2) não existe; quando tudo dá certo, imprime uma linha estável (nome proposto `PG_INTEGRATION_HELPER_OK`). Sem o modo, o `cargo test` local continua pulando, como hoje. O script e a CI sempre ligam o modo. **Limite conhecido:** o modo obrigatório não faz falhar um teste que retorna cedo **antes** de chamar o helper (ex.: checagem de Neo4j ou de credencial antes do helper); por isso C1 exige a linha estável do helper, e não só "1 passed".
2. **DB de teste isolado por marcador no banco**, não só pelo nome: tabela `bot_test_database_marker` com uma linha, criada **fora** das migrações e **fora** do binário.
   - **Na CI:** um passo do job `postgres-integration` em `backend-ci.yml`, depois do health check do service container e antes de `run-pg-integration-tests.sh`, cria o marcador com `psql "$DATABASE_URL"` a partir do runner (o `ubuntu-latest` traz cliente `psql`; se não trouxer, o mesmo SQL roda via `docker exec` no container do service). **Quem cria: o workflow**, e só ele.
   - **Local:** um script de setup de PG descartável (nome proposto `backend/scripts/prepare-pg-test-db.sh`) roda o mesmo SQL; o desenvolvedor o executa de propósito contra um banco descartável.
   - O helper recusa migrar/rodar sem o marcador.
   - **Runtime com marcador recusa subir, num lugar só:** a checagem fica em `PostgresDatabase::connect_from_url` (`postgres.rs:55-70`), logo depois de `assert_server_version` (`:65`), e devolve um erro novo (nome proposto `DatabaseError::TestDatabaseMarkerPresent`). Como os seis pontos de entrada do Contexto passam por essa função, todos ficam cobertos sem código repetido. O helper de teste usa um construtor `#[cfg(test)]` próprio que faz a checagem inversa (exige o marcador). Esse erro é **fatal** em todos os pontos de entrada: nos dois caminhos de `bundle.rs` que hoje só dão `warn` e seguem sem PG, essa variante encerra o processo com exit ≠ 0 em vez de seguir sem banco. (Decisão por recomendação: o runtime nunca roda contra banco de teste.)
3. **Manifesto verificado estaticamente, com remoção de entradas sem função:** `assert-pg-integration-manifest.sh` (já em Python) passa a comparar o **conjunto** de funções de teste que chamam `database_for_integration_test()` com o conjunto `PG_TESTS` (método de descoberta em "Descoberta do conjunto PG", abaixo); qualquer diferença falha o gate e lista os nomes. Exceções só numa lista `PG_TEST_EXCEPTIONS` no próprio script, uma por linha, cada uma com justificativa. `EXPECTED_PG_INTEGRATION_TESTS` deixa de ser número digitado à mão (vira derivado do manifesto ou sai).
   - **Entrada sem função é removida por W0-02 (o código é o fato).** Isso vale para `persist_dataset_rejects_conflicting_manifest_for_same_id` e para qualquer outra entrada do manifesto sem função de teste correspondente em `backend/src` no momento da implementação (inclusive `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`, se a entrada da cópia de trabalho chegar ao `main` sem a função). Se o owner restaurar o teste de `persist_dataset` no [W0-09](./wave0-09-v18-verificacao-sdd.md) (item 5), o C0 obriga a readicioná-lo ao manifesto no mesmo PR, porque a função passa a chamar o helper. Com isso o GREEN de W0-02 não depende da decisão de W0-09.
4. **Cada teste roda exatamente uma vez:** o script resolve cada nome do manifesto pelo `cargo test --bin bot -- --list` (exatamente um caminho terminando em `::<nome>`, senão falha), roda com `--exact <caminho completo> --nocapture` e `BOT_PG_INTEGRATION_REQUIRED=1`, e exige na saída daquele teste `1 passed; 0 failed; 0 ignored` **e** a linha `PG_INTEGRATION_HELPER_OK` uma vez. "1 passed" com a linha do helper significa "executou contra PG".
5. **Imagem compatível na CI (gate em aberto):** o job deve usar uma imagem fixada por digest que forneça PostgreSQL 18+ e extensões `timescaledb` e `vector` disponíveis e instaladas, e que inicialize como service container do GitHub com credenciais/configuração requeridas e `POSTGRES_DB=trading_bot`. Não foi provado que o digest do compose (`timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`) é compatível com esse uso nem que contém ambas as extensões. Antes da implementação, registrar evidência verificável de compatibilidade da imagem exata (digest, inicialização do service, versão, extensões); se não compatível, selecionar e validar outra imagem por digest. C4 é bloqueante para aprovação G1 e aceite, não uma afirmação deste SDD.
6. **Toolchain fixado** em `rust-toolchain.toml` (versão e componentes `rustfmt`, `clippy`), usado local e na CI.

### Descoberta do conjunto PG (C0)

- **Proposta sujeita a revisão técnica: parser estático + proibição de wrapper.** O `assert-pg-integration-manifest.sh` deve descobrir, com uma estratégia robusta para a sintaxe Rust usada pelo repositório, todos os testes que chamam `database_for_integration_test()` fora do próprio helper e comparar o conjunto exato com `PG_TESTS`. Um chamador que não seja função de teste deve falhar com arquivo/linha; teste chamador ausente do manifesto ou entrada sem função correspondente também falha. Confirmar na revisão G1 que o parser proposto consegue associar chamadas ao `fn` correto em todos os casos (atributos, funções async, módulos aninhados, comentários e formatação); não tratar uma busca textual simplista como prova.
- **O que o estático não prova, e quem cobre:** que o teste de fato chega ao helper em tempo de execução. Isso é coberto por C1 (linha `PG_INTEGRATION_HELPER_OK` obrigatória por teste do manifesto).
- **Alternativa considerada:** descoberta dinâmica (rodar todos os testes do bin com `BOT_PG_INTEGRATION_REQUIRED=1` e sem `DATABASE_URL`; os que falham com a mensagem estável do helper formam o conjunto PG). Pega wrappers sem regra extra, mas não pega teste que retorna cedo antes do helper e roda a suíte inteira no gate. Rejeitada como método principal; fica como verificação manual opcional.

**Alternativa considerada:** banco de teste com outro nome (ex.: `trading_bot_test`). Mais visível, mas exige afrouxar o guard de nome do runtime (`postgres.rs:57-58`), que hoje é uma proteção. Rejeitada nesta fatia.

## Seams (aprovados pelo owner; revisão técnica independente G1 pendente)

O owner aprovou os seams públicos abaixo para orientar um desenho proporcional e os testes comportamentais. Isso não é aprovação técnica do SDD e não autoriza implementação; G1 continua pendente.

| Seam aprovado | Proposta de contrato (sujeita a G1) |
|---|---|
| Versão/extensões PG | PostgreSQL 18 ou superior, com extensões `timescaledb` e `vector` disponíveis e instaladas; compatibilidade da imagem concreta continua sendo gate C4. |
| Variável de modo | `BOT_PG_INTEGRATION_REQUIRED=1`; nesse modo, ausência/erro de conexão, versão incompatível, migração falha ou ausência de marcador causa falha do teste com mensagem estável sem URL/segredo. |
| Marcador e propriedade | Marcador dedicado do banco de teste, criado apenas pelo setup explícito local ou pelo workflow de CI, nunca por migração nem pelo helper/runtime. Testes exigem o marcador; conexões de runtime o rejeitam. O marcador não substitui isolamento por credenciais/instância descartável. |
| Manifesto | Comparar `PG_TESTS` com as funções de teste reais que chamam o helper; diferenças em qualquer direção falham. Remover entradas sem função real. Validar a estratégia de descoberta na G1. |
| Execução | Resolver cada entrada a exatamente um teste e invocá-lo com seleção exata; falhar se faltar, duplicar ou houver erro/ignorados. Exigir saída individual provando uma execução e a linha `PG_INTEGRATION_HELPER_OK` exatamente uma vez. |
| Toolchain | Fixar em `rust-toolchain.toml` toolchain e componentes usados pela CI e pelos comandos locais relevantes. |

## Critérios de aceite (conclusão da Onda 0)

Critérios de T-CI-01 (toolchain e job `rust`) e T-CI-02 (PG 18 e manifesto), copiados aqui para este SDD ser autossuficiente:

- **C0 (manifesto completo).** `assert-pg-integration-manifest.sh` falha se o conjunto de testes que chamam o helper (parser estático acima) diferir de `PG_TESTS`, e falha se alguma chamada ao helper estiver num `fn` sem atributo de teste. Provas registradas na entrega: (a) um teste fora do manifesto faz o script falhar; (b) um wrapper sem atributo de teste faz o script falhar; (c) uma entrada sem função (a fantasma do HEAD) faz o script falhar. Os dez testes listados no Contexto entram no manifesto ou em `PG_TEST_EXCEPTIONS` com justificativa; a recomendação é que os dez entrem, porque todos precisam de PG ligado (os `graph_admin_*` também exigem Neo4j desligado, que é o padrão da CI). As entradas sem função são removidas (Decisão 3).
- **C1 (T-CI-02).** Cada entrada do manifesto resolve a exatamente um identificador canônico de teste a partir da listagem do binário; resolver zero ou mais de um falha antes da execução. Executar com seleção exata (`--exact`) e modo obrigatório. Para cada invocação individual, validar status de sucesso e evidência não ambígua de exatamente um teste passado, nenhum falho/ignorado e `PG_INTEGRATION_HELPER_OK` exatamente uma vez; saída ausente, repetida, agregada de forma ambígua ou sem helper falha o script. O agregado de execuções aprovadas deve ser igual ao tamanho do manifesto. Testes adversariais provam RED para os dois nomes fantasmas observados e para nome com correspondência parcial.
- **C2.** Em modo obrigatório, ausência de `DATABASE_URL`, falha de conexão, PostgreSQL < 18, falha de qualquer migração ou ausência de marcador causam falha do teste/job; mensagem estável não pode revelar URL nem segredo. Demonstrar RED para cada caso de erro e GREEN no caminho válido em ambiente PG 18+ com extensões disponíveis/instaladas e marcador. Guardar logs/assertions reproduzíveis.
- **C3.** Helper recusa banco sem marcador (teste unitário sem PG real para a decisão; teste PG para o caminho feliz). Runtime recusa banco com marcador: teste PG chama `PostgresDatabase::connect_from_url` contra o banco com marcador e espera `TestDatabaseMarkerPresent`; teste do `serve`/`bootstrap_http_api` contra o mesmo banco espera exit ≠ 0 (não `warn` seguido de "sem PG"). Guard estático: `rg 'PgPoolOptions' backend/src` só em `core/database/postgres.rs`, para que nenhum ponto de entrada novo crie pool sem passar pela checagem.
- **C4 (T-CI-02; gate de compatibilidade da imagem).** Antes de escolher/configurar a imagem, documentar evidência para o digest exato de que (a) o GitHub Actions o inicia como service container com variáveis/health check requeridos, (b) `server_version_num` é ≥ 180000, e (c) `CREATE EXTENSION`/`pg_extension` confirma `timescaledb` e `vector`. Não presumir que o digest do compose cumpre qualquer ponto. Após aprovação da imagem e implementação, a execução real de CI deve mostrar versão/extensões e os jobs `rust` e `postgres-integration` verdes; registrar link da execução.
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

- Antes do TDD: fechar G1, inclusive prova de compatibilidade da imagem/digest concreta e decisão sobre parsing confiável do manifesto; até lá, sem alteração de workflow/helper/script.
- Após autorização G1, aplicar TDD por seam público e fatias verticais: RED comportamental reproduzível antes da implementação, GREEN mínimo, preservar regressões. Cobrir modo obrigatório (URL ausente, conexão/versão/migração/extensão/marcador inválido), marcador rejeitado pelo runtime, comparação completa do manifesto (teste omitido/entrada fantasma/wrapper) e seleção/execução exata com prova individual do helper. Depois executar `verify-backend-gates.sh`, `assert-pg-integration-manifest.sh`, `run-pg-integration-tests.sh` em PostgreSQL descartável 18+ com extensões `timescaledb` e `vector` e marcador. Registrar comandos e resultados, incluindo a asserção de baseline 520/519 e sua resolução.

## Rollout / rollback

- Rollout: mudança de CI/scripts/helpers somente após aprovação G1 e autorização de implementação; nenhuma publicação/deploy. Ativar o job com banco descartável, marcador aplicado pelo workflow/setup explícito, e confirmar logs de versão, extensões, manifesto e execução individual. Só aceitar quando ambos jobs de CI estiverem verdes.
- Rollback: reverter o commit completo workflow + helpers/scripts para o último estado conhecido. Isso remove a nova recusa de marcador em runtime também; até lá, não apontar runtime para o banco marcado. A reversão não deve manter workflow e helper em estados incompatíveis. Nenhuma implantação de aplicação está prevista.
