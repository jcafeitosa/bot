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

1. **Modo "PG obrigatório":** `BOT_PG_INTEGRATION_REQUIRED=1`, lido em `core/config` (exigido pelo guard de env de `verify-backend-gates.sh`). Nesse modo, o helper **falha o teste** (panic com mensagem estável, sem URL) quando falta `BOT_PG_TEST_DATABASE_URL` ou identidade/manifest válido, a conexão falha, a versão é < 18, a migração falha ou o marcador de teste (item 2) não existe; `DATABASE_URL` nunca é fallback. Quando tudo dá certo, imprime uma linha estável (nome proposto `PG_INTEGRATION_HELPER_OK`). Sem o modo, o `cargo test` local continua pulando, como hoje. O script e a CI sempre ligam o modo. **Limite conhecido:** o modo obrigatório não faz falhar um teste que retorna cedo **antes** de chamar o helper (ex.: checagem de Neo4j ou de credencial antes do helper); por isso C1 exige a linha estável do helper, e não só "1 passed".
2. **DB de teste isolado por marcador no banco**, não só pelo nome: tabela `bot_test_database_marker` com uma linha, criada **fora** das migrações e **fora** do binário.
   - **Na CI:** um passo do job `postgres-integration` em `backend-ci.yml`, depois do health check do service container e antes de `run-pg-integration-tests.sh`, cria o marcador com `psql "$DATABASE_URL"` a partir do runner (o `ubuntu-latest` traz cliente `psql`; se não trouxer, o mesmo SQL roda via `docker exec` no container do service). **Quem cria: o workflow**, e só ele.
   - **Local:** um script de setup de PG descartável (nome proposto `backend/scripts/prepare-pg-test-db.sh`) roda o mesmo SQL; o desenvolvedor o executa de propósito contra um banco descartável.
   - O helper recusa migrar/rodar sem o marcador.
   - **Runtime com marcador recusa subir, num lugar só:** a checagem fica em `PostgresDatabase::connect_from_url` (`postgres.rs:55-70`), logo depois de `assert_server_version` (`:65`), e devolve um erro novo (nome proposto `DatabaseError::TestDatabaseMarkerPresent`). Como os seis pontos de entrada do Contexto passam por essa função, todos ficam cobertos sem código repetido. O helper de teste usa um construtor `#[cfg(test)]` próprio que faz a checagem inversa (exige o marcador). Esse erro é **fatal** em todos os pontos de entrada: nos dois caminhos de `bundle.rs` que hoje só dão `warn` e seguem sem PG, essa variante encerra o processo com exit ≠ 0 em vez de seguir sem banco. (Decisão por recomendação: o runtime nunca roda contra banco de teste.)
3. **Manifesto verificado estaticamente, com remoção de entradas sem função:** `assert-pg-integration-manifest.sh` (já em Python) passa a comparar o **conjunto** de funções de teste que chamam `database_for_integration_test()` com o conjunto `PG_TESTS` (método de descoberta em "Descoberta do conjunto PG", abaixo); qualquer diferença falha o gate e lista os nomes. Exceções só numa lista `PG_TEST_EXCEPTIONS` no próprio script, uma por linha, cada uma com justificativa. `EXPECTED_PG_INTEGRATION_TESTS` deixa de ser número digitado à mão (vira derivado do manifesto ou sai).
   - **Entrada sem função é removida por W0-02 (o código é o fato).** Isso vale para `persist_dataset_rejects_conflicting_manifest_for_same_id` e para qualquer outra entrada do manifesto sem função de teste correspondente em `backend/src` no momento da implementação (inclusive `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`, se a entrada da cópia de trabalho chegar ao `main` sem a função). Se o owner restaurar o teste de `persist_dataset` no [W0-09](./wave0-09-v18-verificacao-sdd.md) (item 5), o C0 obriga a readicioná-lo ao manifesto no mesmo PR, porque a função passa a chamar o helper. Com isso o GREEN de W0-02 não depende da decisão de W0-09.
4. **Cada teste roda exatamente uma vez:** o script resolve cada nome do manifesto pelo `cargo test --bin bot -- --list` (exatamente um caminho terminando em `::<nome>`, senão falha), roda com `--exact <caminho completo> --nocapture` e `BOT_PG_INTEGRATION_REQUIRED=1`, e exige na saída daquele teste `1 passed; 0 failed; 0 ignored` **e** a linha `PG_INTEGRATION_HELPER_OK` uma vez. "1 passed" com a linha do helper significa "executou contra PG".
5. **Imagem compatível na CI (gate em aberto):** o job deve usar uma imagem fixada por digest que forneça PostgreSQL 18+ e extensões `timescaledb` e `vector` disponíveis e instaladas, e que inicialize como service container do GitHub com credenciais/configuração requeridas e `POSTGRES_DB=trading_bot`. Não foi provado que o digest do compose (`timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`) é compatível com esse uso nem que contém ambas as extensões. Antes da implementação, registrar evidência verificável de compatibilidade da imagem exata (digest, inicialização do service, versão, extensões); se não compatível, selecionar e validar outra imagem por digest. C4 é bloqueante para configuração/aceite do job GitHub em G4; não bloqueia a aprovação G1 nem o aceite do runner local quando a identidade/digest e as extensões locais estão comprovados. A execução GitHub ainda é requisito obrigatório para aceitar a integração de CI.
6. **Toolchain fixado** em `rust-toolchain.toml` (versão e componentes `rustfmt`, `clippy`), usado local e na CI.

### Descoberta do conjunto PG (C0)

- **Parser AST syn proposto; ver seção de adendo G1.: parser estático + proibição de wrapper.** O `assert-pg-integration-manifest.sh` deve descobrir, com uma estratégia robusta para a sintaxe Rust usada pelo repositório, todos os testes que chamam `database_for_integration_test()` fora do próprio helper e comparar o conjunto exato com `PG_TESTS`. Um chamador que não seja função de teste deve falhar com arquivo/linha; teste chamador ausente do manifesto ou entrada sem função correspondente também falha. Confirmar na revisão G1 que o parser proposto consegue associar chamadas ao `fn` correto em todos os casos (atributos, funções async, módulos aninhados, comentários e formatação); não tratar uma busca textual simplista como prova.
- **O que o estático não prova, e quem cobre:** que o teste de fato chega ao helper em tempo de execução. Isso é coberto por C1 (linha `PG_INTEGRATION_HELPER_OK` obrigatória por teste do manifesto).
- **Alternativa considerada:** descoberta dinâmica (rodar todos os testes do bin com `BOT_PG_INTEGRATION_REQUIRED=1` e sem `DATABASE_URL`; os que falham com a mensagem estável do helper formam o conjunto PG). Pega wrappers sem regra extra, mas não pega teste que retorna cedo antes do helper e roda a suíte inteira no gate. Rejeitada como método principal; fica como verificação manual opcional.

**Alternativa considerada:** banco de teste com outro nome (ex.: `trading_bot_test`). Mais visível, mas exige afrouxar o guard de nome do runtime (`postgres.rs:57-58`), que hoje é uma proteção. Rejeitada nesta fatia.

## Seams (aprovados pelo owner; revisão técnica independente G1 pendente)

O owner aprovou os seams públicos abaixo para orientar um desenho proporcional e os testes comportamentais. Isso não é aprovação técnica do SDD e não autoriza implementação; G1 continua pendente.

| Seam aprovado | Proposta de contrato (sujeita a G1) |
|---|---|
| Versão/extensões PG | PostgreSQL 18 ou superior, com extensões `timescaledb` e `vector` disponíveis e instaladas; compatibilidade da imagem concreta continua sendo gate C4. |
| Variável de modo | `BOT_PG_INTEGRATION_REQUIRED=1` exige também `BOT_RUN_PG_INTEGRATION=1`, `BOT_PG_TEST_DATABASE_URL` e manifest/identidade válidos; falhas de conexão, versão, migração, marcador ou identidade causam falha estável sem URL/segredo. `DATABASE_URL` não é fallback. |
| Marcador e propriedade | Marcador dedicado do banco de teste, criado apenas pelo setup explícito local ou pelo workflow de CI, nunca por migração nem pelo helper/runtime. Testes exigem o marcador; conexões de runtime o rejeitam. O marcador não substitui isolamento por credenciais/instância descartável. |
| Manifesto | Comparar `PG_TESTS` com as funções de teste reais que chamam o helper; diferenças em qualquer direção falham. Remover entradas sem função real. Validar a estratégia de descoberta na G1. |
| Execução | Resolver cada entrada a exatamente um teste e invocá-lo com seleção exata; falhar se faltar, duplicar ou houver erro/ignorados. Exigir saída individual provando uma execução e a linha `PG_INTEGRATION_HELPER_OK` exatamente uma vez. |
| Toolchain | Fixar em `rust-toolchain.toml` toolchain e componentes usados pela CI e pelos comandos locais relevantes. |

## Critérios de aceite (conclusão da Onda 0)

Critérios de T-CI-01 (toolchain e job `rust`) e T-CI-02 (PG 18 e manifesto), copiados aqui para este SDD ser autossuficiente:

- **C0 (manifesto completo).** `assert-pg-integration-manifest.sh` falha se o conjunto de testes que chamam o helper (parser estático acima) diferir de `PG_TESTS`, e falha se alguma chamada ao helper estiver num `fn` sem atributo de teste. Provas registradas na entrega: (a) um teste fora do manifesto faz o script falhar; (b) um wrapper sem atributo de teste faz o script falhar; (c) uma entrada sem função (a fantasma do HEAD) faz o script falhar. Os dez testes listados no Contexto entram no manifesto ou em `PG_TEST_EXCEPTIONS` com justificativa; a recomendação é que os dez entrem, porque todos precisam de PG ligado (os `graph_admin_*` também exigem Neo4j desligado, que é o padrão da CI). As entradas sem função são removidas (Decisão 3).
- **C1 (T-CI-02).** Cada entrada do manifesto resolve a exatamente um identificador canônico de teste a partir da listagem do binário; resolver zero ou mais de um falha antes da execução. Executar com seleção exata (`--exact`) e modo obrigatório. Para cada invocação individual, validar status de sucesso e evidência não ambígua de exatamente um teste passado, nenhum falho/ignorado e `PG_INTEGRATION_HELPER_OK` exatamente uma vez; saída ausente, repetida, agregada de forma ambígua ou sem helper falha o script. O agregado de execuções aprovadas deve ser igual ao tamanho do manifesto. Testes adversariais provam RED para os dois nomes fantasmas observados e para nome com correspondência parcial.
- **C2.** Em modo obrigatório, ausência de `BOT_PG_TEST_DATABASE_URL`, manifest/identidade inválidos, falha de conexão, PostgreSQL < 18, falha de qualquer migração ou ausência/divergência de marcador causam falha do teste/job; mensagem estável não pode revelar URL nem segredo. `DATABASE_URL` sozinho nunca seleciona nem habilita PG integration e não é usado como fallback. Demonstrar RED para cada caso de erro e GREEN no caminho válido em ambiente PG 18+ com extensões disponíveis/instaladas e marcador. Guardar logs/assertions reproduzíveis.
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

- Antes do TDD: fechar G1 com Critic independente, incluindo identidade e digest local comprovados e parsing confiável do manifesto; até lá, sem alteração de workflow/helper/script. A configuração/aceite do service container GitHub fica separada e bloqueada até G4/C4, que exige evidência do próprio runner GitHub.
- Após autorização G1, aplicar TDD por seam público e fatias verticais: RED comportamental reproduzível antes da implementação, GREEN mínimo, preservar regressões. Cobrir modo obrigatório (URL ausente, conexão/versão/migração/extensão/marcador inválido), marcador rejeitado pelo runtime, comparação completa do manifesto (teste omitido/entrada fantasma/wrapper) e seleção/execução exata com prova individual do helper. Depois executar `verify-backend-gates.sh`, `assert-pg-integration-manifest.sh`, `run-pg-integration-tests.sh` em PostgreSQL descartável 18+ com extensões `timescaledb` e `vector` e marcador. Registrar comandos e resultados, incluindo a asserção de baseline 520/519 e sua resolução.

## Rollout / rollback

- Rollout: mudança de CI/scripts/helpers somente após aprovação G1 e autorização de implementação; nenhuma publicação/deploy. Ativar o job com banco descartável, marcador aplicado pelo workflow/setup explícito, e confirmar logs de versão, extensões, manifesto e execução individual. Só aceitar quando ambos jobs de CI estiverem verdes.
- Rollback: reverter o commit completo workflow + helpers/scripts para o último estado conhecido. Isso remove a nova recusa de marcador em runtime também; até lá, não apontar runtime para o banco marcado. A reversão não deve manter workflow e helper em estados incompatíveis. Nenhuma implantação de aplicação está prevista.

## Revisão G1 ciclo 3 — contratos revisados C0/C1/C4

> Este adendo complementa as descrições anteriores e prevalece em caso de conflito. Mantém status `draft`; não aprova G1 nem autoriza implementação.

### C4 — imagem e evidência reproduzível

Imagem candidata: `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`, digest fixado em `docker-compose.bot.yml`. Nesta revisão, `docker inspect` do container local `bot-agents-postgres` confirmou digest e image ID iguais; consulta SQL ao banco retornou `server_version_num=180006`, `timescaledb=2.30.1` e `vector=0.8.6`. Isso comprova o container local, não a inicialização no GitHub Actions.

Para passar C4 de CI em G4, o workflow deve iniciar o digest exato como service container com `POSTGRES_USER=postgres`, `POSTGRES_PASSWORD=postgres`, `POSTGRES_DB=trading_bot`, porta 5432 e health `pg_isready -U postgres -d trading_bot` (intervalo 5s, timeout 5s, 10 tentativas). Após healthy, registrar saída/exit code sem segredo e link da execução para `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc 'SHOW server_version_num'` e `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc "SELECT extname || '=' || extversion FROM pg_extension WHERE extname IN ('timescaledb','vector') ORDER BY extname"`. Gate: versão ≥ 180000 e as duas extensões instaladas. Se disponíveis, mas não instaladas, executar `CREATE EXTENSION IF NOT EXISTS timescaledb; CREATE EXTENSION IF NOT EXISTS vector;` e consultar novamente; falha é bloqueante. CI real ainda não foi executado; C4 fica pendente até evidência do runner GitHub.

### C0 — checker Rust AST

Criar binário checker independente em `backend/scripts/pg-manifest-checker/`, com versões/features de `syn` fixadas no lockfile, usando `syn::parse_file` e `syn::visit::Visit`. Percorrer arquivos `.rs` rastreados sob `backend/src`, módulos inline, `ItemFn`, blocos e `ExprCall`; extrair nome qualificado, caminho relativo e linha/coluna das chamadas diretas ao helper cujo último segmento do caminho seja `database_for_integration_test`. Comparar conjunto exato extraído com `PG_TESTS`, reportando divergências e origens. Reconhecer somente `#[test]` e `#[tokio::test]` (formas observadas), também em `async fn`; novos atributos exigem suporte e fixture explícitos.

Além do manifesto, o checker proíbe em funções `#[test]`/ `#[tokio::test]` conexões PostgreSQL diretas por `PostgresDatabase::connect_from_env`, `PostgresDatabase::connect_from_url`, `Database::connect_from_env`, `Database::connect_from_url`, criação/conexão de `PgPoolOptions` e APIs equivalentes identificadas no inventário de conexões. Todas as integrações passam por `database_for_integration_test`, que é o único connector normal de teste e lê apenas o alvo dedicado verificado. A única exceção permitida é o teste canônico `core::database::postgres::tests::connect_refuses_marked_test_database`, que invoca uma vez `PostgresDatabase::connect_from_url` para provar a recusa runtime do marcador e afirma especificamente `DatabaseError::TestDatabaseMarkerPresent`. A exceção é exata por caminho qualificado e função; não se estende a helpers, wrappers, `connect_from_env`, pools diretos ou outros testes.

Fail closed para wrappers, aliases/imports, indireção por ponteiro/closure, `include!`, módulos externos via `#[path]`, macros contendo tokens de conexão/helper e código gerado; erros trazem arquivo/linha e chamador. Não expandir proc-macros nem inferir fluxo. Fixtures sem banco: teste Tokio no módulo inline chamando helper passa; a exceção exata com chamada e assertion do erro passa; função sem atributo, wrapper, teste chamador omitido, entrada fantasma, macro/include e qualquer chamada direta de conexão fora da exceção falham. Fixture adversarial com `connect_from_env`/ `connect_from_url` em teste PG fora da allowlist deve ser RED; fixture equivalente usando o helper deve ser GREEN. Menção em comentário/string é ignorada. Limite: AST não prova expansão/semântica de macros ou alcançabilidade runtime; essas formas ficam proibidas e C1 cobre execução.

### C1 — chegada ao helper e término das assertions

`PG_INTEGRATION_HELPER_OK` comprova conexão/migração, não conclusão do teste. Cada teste PG termina com `pg_integration_assertions_complete!("<nome_qualificado>")`, após operações e assertions PostgreSQL. A macro emite `PG_INTEGRATION_ASSERTIONS_OK:<nome_qualificado>`. O checker AST exige exatamente uma chamada direta ao helper e exatamente uma chamada de completion como última statement; retorno antecipado antes dela não emite a linha e causa falha do runner.

Para cada invocação isolada `--exact`, runner exige exit zero, `1 passed`, zero falhas/ignorados, exatamente uma linha `PG_INTEGRATION_HELPER_OK` e uma linha `PG_INTEGRATION_ASSERTIONS_OK` com o mesmo nome. Fixture adversarial com `return` após helper e antes do marcador deve falhar por ausência de completion; fixture com assertions seguidas pelo marcador final deve passar. Revisão de código verifica o marcador depois das assertions; a linha prova o protocolo, não a qualidade semântica das assertions.

### Estado

C0/C1 ficam especificados para revisão independente. A evidência local do digest permite avaliar o alvo de desenvolvimento local, mas C4 de CI ainda precisa da execução real do service container GitHub. Sem re-review independente, G1 permanece pendente.

## Revisão G1 ciclo 4 — isolamento por identidade e evidência por ambiente

> Este adendo registra como tratados os achados do Critic G1 e prevalece sobre trechos anteriores conflitantes. O documento continua em `draft`; a revisão independente G1 e a aprovação do owner continuam pendentes. Este adendo não autoriza implementação.

### Runner local PG: identidade antes do marcador

O runner local cria um container PostgreSQL novo por execução, usando o digest aprovado para desenvolvimento, UUID aleatório único em nome/label, credencial aleatória, database `trading_bot`, volume descartável e porta aleatória publicada somente em loopback. Não aceita container, volume ou database preexistente como alvo. O banco persistente da aplicação não é alvo nem fallback.

A sequência obrigatória é:

1. Criar o container efêmero e registrar em um manifest restrito (permissão `0600`) o UUID da execução, container ID, digest da imagem, porta publicada, database e referência do manifest. O manifest não escreve credencial ou URL completa em logs nem em artefatos compartilhados.
2. Antes de criar qualquer marcador, confirmar que o Docker CLI usa contexto local cujo endpoint é socket Unix local (não Docker remoto/TCP) e consultar a API desse daemon para validar container em execução, ID, label/UUID, imagem/referência pelo digest fixado e publicação de `5432/tcp` em `127.0.0.1` com porta host aleatória. O helper valida, antes de criar cliente ou conexão PG, que o host de `BOT_PG_TEST_DATABASE_URL` é `localhost`, `127.0.0.1` ou `::1`, que a porta é exatamente a porta host inspecionada, e que o path seleciona `trading_bot`; também confirma que o manifest, o UUID e o container pertencem ao mesmo contexto Docker local validado. Em seguida, o runner valida pelo endpoint recém-criado que `current_database()` é exatamente `trading_bot` e que versão/extensões atendem ao contrato. Qualquer divergência encerra antes do marker, migration ou teste. Endpoint remoto, hostname, endereço de bridge, host/porta não correspondentes e Docker context remoto devem ser rejeitados antes de abrir conexão.
3. Só após essas validações, criar fora das migrations a tabela `bot_test_database_marker` e uma única linha contendo o UUID, container ID, digest, database e modo do alvo. Não criar nem reparar marcador a partir do helper.
4. Passar o endpoint somente por `BOT_PG_TEST_DATABASE_URL` e os dados de identidade/manifest por campos dedicados do manifest. O helper de teste exige `BOT_RUN_PG_INTEGRATION=1`, URL dedicada e manifest válido; valida novamente o Docker context local, container ativo e o vínculo digest/UUID/host loopback/porta host/database **antes de criar client, resolver ou abrir conexão PostgreSQL**. `DATABASE_URL` é ignorada como fonte e sua presença nunca habilita o gate nem serve de fallback.
5. Após a verificação Docker pré-conexão, conectar ao endpoint dedicado, validar exatamente uma linha de marcador contra todos os campos de identidade e só então migrar. Ausência/divergência do marcador, manifest incompleto, container parado, erro do Docker, identidade diferente ou falha de consulta termina antes da migration. O marcador é defesa em profundidade; isoladamente, ele não prova que o banco é descartável.

O modo obrigatório continua fail-loud. `BOT_PG_INTEGRATION_REQUIRED=1` exige `BOT_RUN_PG_INTEGRATION=1`, `BOT_PG_TEST_DATABASE_URL` e a identidade descrita acima; não exige nem lê `DATABASE_URL`. Sem modo obrigatório, a suíte default retorna sem conectar. Se o gate estiver ligado e algum requisito faltar ou falhar, o teste falha com mensagem estável sem URL, credencial ou conteúdo do manifest.

### Semântica do marcador em runtime

A tabela é criada fora das migrations, portanto sua ausência é normal para databases de runtime ainda não marcados. A checagem runtime consulta primeiro `to_regclass('public.bot_test_database_marker')`, que distingue ausência do objeto sem transformar falhas arbitrárias numa ausência presumida. Registra um dos resultados estáveis `UNMARKED_RUNTIME`, `TEST_DATABASE_MARKER_PRESENT` ou `MARKER_LOOKUP_FAILED`:

- relação ausente (resultado SQL `NULL`): classificar como `UNMARKED_RUNTIME`; conexão runtime pode prosseguir, pois o marcador externo ainda não foi provisionado;
- relação presente sem linha: classificar como `UNMARKED_RUNTIME`; conexão runtime pode prosseguir;
- exatamente uma linha válida de marcador: classificar como `TEST_DATABASE_MARKER_PRESENT` e recusar a conexão runtime com erro dedicado `TestDatabaseMarkerPresent`;
- múltiplas linhas, esquema/colunas incompatíveis ou erro de permissão, timeout, conexão interrompida, erro SQL, resultado ambíguo ou qualquer falha de lookup: classificar como `MARKER_LOOKUP_FAILED` e falhar fechado. Nunca converter falha de leitura em “sem marcador”.

No helper de integração, relação/linha ausente, múltipla ou identidade divergente sempre falha antes de migration. Essa diferença preserva databases normais de runtime sem permitir que o caminho de teste migre um alvo não atestado.

### Evidência local e gate de CI separados

A revisão registra para o digest `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840` a evidência local descrita acima: container local com digest e image ID correspondentes; `server_version_num=180006`, `timescaledb=2.30.1` e `vector=0.8.6`. Essa evidência pode sustentar o G1/aceite do runner Docker local quando a identidade efêmera for comprovada. Não afirma que GitHub Actions consegue iniciar a mesma imagem como service container.

O gate C4 de CI permanece pendente até execução real do workflow com o digest exato, inicialização/health check bem-sucedidos, PostgreSQL 18+, e ambas as extensões disponíveis e instaladas. A validação CI deve registrar a execução e os resultados redigidos de versão/extensões; evidência local nunca substitui esse gate. Não alterar workflow ou marcar CI como aprovada nesta etapa documental.

### Alternativas, riscos e validação

- **Alternativa considerada — marcador sem identidade Docker:** rejeitada; uma tabela pode existir em um banco persistente e não prova descartabilidade.
- **Alternativa considerada — usar `DATABASE_URL` e verificar apenas o nome `trading_bot`:** rejeitada; nome e URL não distinguem o banco de aplicação. URL dedicada, manifest e identidade ativa do container são gates independentes.
- **Alternativa considerada — tratar qualquer erro de lookup do marcador como ausência:** rejeitada; falhas de permissão/rede poderiam abrir o runtime. Só SQLSTATE de tabela inexistente é ausência inequívoca; os demais erros fecham.
- **Risco operacional:** falha de cleanup pode deixar container/volume efêmero local órfão. UUID único impede reuso; o runner reporta o identificador para limpeza manual, sem tocar containers ou volumes de aplicação.
- **Validação TDD futura:** provar `DATABASE_URL` sozinho resulta em zero conexão/migration; gate ligado sem URL dedicada/manifest falha antes de conexão; hostname remoto, Docker context remoto, host que não seja loopback, host port divergente do mapeamento, database diferente, identity/digest/UUID divergente falham antes de qualquer conexão ou marker; marker ausente/divergente falha antes de migration; runtime classifica tabela ausente/vazia como `UNMARKED_RUNTIME`, recusa marker presente e falha fechado em erro inesperado de lookup. Caminho verde exige container efêmero identificado, manifest válido e marker correspondente antes da migration. C4 requer ainda execução real no GitHub; nada nesta revisão documental fornece essa evidência.

### Próxima ação de governança

Solicitar ao Critic independente nova revisão G1 deste adendo. Até o veredito registrado, W0-02 permanece `draft`, G1 pendente e implementação bloqueada.

## Revisão G1 ciclo 5 — conexões de teste centralizadas no helper

> Este adendo trata o finding bloqueante do Critic de que C0 enumerava chamadas do helper, mas não impedia um teste de abrir conexão PG por outra API. Prevalece sobre descrições anteriores de C0; status continua `draft`, sem aprovação G1 ou autorização de implementação.

### Regra para conexões PostgreSQL em testes

Toda conexão usada por testes de integração PostgreSQL deve passar por `database_for_integration_test`, o connector dedicado que valida gate, endpoint loopback, manifesto/identidade do container local e marcador antes de migrate. Chamadas de teste a `PostgresDatabase::connect_from_env`, `PostgresDatabase::connect_from_url`, `Database::connect_from_env`, `Database::connect_from_url`, criação/conexão de `PgPoolOptions` ou qualquer API equivalente são proibidas fora do connector dedicado.

A única exceção é o teste unitário canônico `core::database::postgres::tests::connect_refuses_marked_test_database`. Ele precisa atravessar o connector runtime `PostgresDatabase::connect_from_url` para afirmar que runtime recusa um banco marcado; o runner fornece para esse caso somente um PG efêmero local identificado e marcado. A allowlist contém apenas esse caminho/função exato, permite uma chamada direta e exige assertion de `DatabaseError::TestDatabaseMarkerPresent`. A exceção não permite `connect_from_env`, pools diretos, wrappers, outros testes, endpoint não descartável ou alteração genérica da allowlist. O manifest o contabiliza em uma categoria de teste de guarda runtime; ele não é tratado como integração comum via helper.

### C0 — detecção e fixtures adversariais

C0 verifica duas propriedades de conjunto e fronteira:

1. o conjunto exato de testes que chamam o helper dedicado coincide com o manifest de integrações PG, conforme C0/C1;
2. nenhuma função de teste abre conexão PostgreSQL por API direta fora do helper, exceto a função única explicitamente allowlisted acima.

O AST checker deve percorrer módulos/arquivos rastreados e atribuir cada chamada ao teste qualificado. Deve detectar conexão direta em teste mesmo que o teste não chame o helper; alias/import, wrapper, ponteiro/closure, macro/include, módulo externo `#[path]` ou código gerado que impeça prova resulta em falha fechada. Nenhum `DATABASE_URL` ou outra URL runtime legitima uma chamada direta. A exceção é comparada por caminho e nome completos, não por substring/nome curto.

Fixtures determinísticas sem banco demonstram:
- **RED:** um teste sem helper que chama `connect_from_env`, `connect_from_url` ou cria pool direto falha no checker;
- **RED:** qualquer outro teste, wrapper ou alias que tente a conexão runtime falha;
- **GREEN:** teste de integração que chama diretamente o helper dedicado entra uma vez no manifest;
- **GREEN restrito:** somente o teste canônico allowlisted que chama uma vez `connect_from_url` e afirma `TestDatabaseMarkerPresent` é aceito;
- **RED:** nome/path semelhante mas não idêntico à allowlist, ausência da assertion esperada, segunda chamada direta ou chamada `connect_from_env` dentro da exceção falham.

C1 continua exigindo execução exata e evidência individual; para o teste allowlisted, o runner exige comprovação de que o alvo foi o container efêmero local marcado e que a recusa ocorreu antes de qualquer efeito de schema/migration. O checker AST demonstra cobertura de chamadas visíveis na fonte, não expansão semântica de macros; formas que escondem chamadas ficam proibidas.

### Decisão, alternativas e risco

A alternativa de permitir conexões diretas em testes e confiar apenas no nome `trading_bot` ou no marker foi rejeitada: não garante URL dedicada nem impede um teste não listado de selecionar o banco persistente. Conectar tudo pelo helper preserva uma única checagem de identidade e reduz bypass; a exceção específica mantém uma prova do guard runtime sem reabrir uma API genérica para integração.

Risco residual: AST estático não expande macros/proc-macros nem prova fluxo dinâmico. Checker deve falhar fechado nesses constructos quando houver indício de conexão/helper, e fixtures cobrem as formas suportadas; o wrapper isolado e o runner efêmero fornecem camadas adicionais, sem substituir G3/G4.

### Estado e próxima ação

O finding foi traduzido em regra, allowlist estreita e provas RED/GREEN planejadas. Isso não é evidência de execução das fixtures. Requer nova revisão independente G1 antes da implementação; W0-02 permanece `draft` e implementação bloqueada.

## Revisão G1 ciclo 6 — cobertura sintática do checker e allowlist estreita

> Este adendo trata o follow-up de G1 sobre como C0 prova a regra acima. Prevalece sobre descrições anteriores do parser. O status fica `draft`; ainda requer verificação independente e não autoriza implementação.

### Algoritmo e sintaxe coberta por C0

O checker lista fontes rastreadas em `backend/src` e `backend/tests`, falha em erro de leitura/parsing e parseia cada arquivo com `syn::parse_file`. A travessia visita módulos inline, `ItemFn`, blocos, statements, expressões, closures, atributos e macro invocations; acompanha o caminho qualificado de cada módulo e cada função. Reconhece raízes de teste pelos atributos aprovados `#[test]`/`#[tokio::test]`, arquivos `backend/tests` e módulos com `#[cfg(test)]`. Um atributo de teste/codegen não reconhecido, `#[path]` em subtree de teste, `include!` de código ou fonte gerada que não possa ser examinada causa erro com arquivo/linha, em vez de ser ignorado.

Para chamadas PostgreSQL, C0 mantém catálogo explícito dos símbolos proibidos (constructors/connectors `PostgresDatabase`, `Database`, `PgPoolOptions` e APIs adicionadas ao inventário W0-02). Resolve paths totalmente qualificados e imports `use` explícitos, inclusive `as` e árvores agrupadas, até seu símbolo canônico. Import wildcard em subtree de teste falha fechado quando não for possível provar que não expõe um símbolo do catálogo. O checker registra cada referência/call de connector, não só a chamada do helper.

O checker constrói grafo de chamadas para chamadas de função estáticas resolvíveis. Parte de cada raiz de teste e percorre wrappers/helpers até os conectores; qualquer caminho alcançável a uma API proibida fora do helper dedicado ou da função allowlisted reprova. Referência a função connector como valor (ponteiro de função), chamada dinâmica/método cuja resolução não possa ser provada, alias não resolvido ou caminho indireto sem alvo conhecido reprova em subtree alcançável por teste. Qualquer arquivo/expressão/macros em que C0 não consiga estabelecer essas propriedades reprova fechado. A API de conexão interna ao helper dedicado é a única saída normal permitida; C0 não exige que o teste enxergue o connector interno.

Macro invocations em teste são visitadas pelo token stream. `include!` e macros/atributos opacos capazes de gerar código de teste/conexão são proibidos, salvo atributos de teste explicitamente suportados e versionados com fixture. Se tokens de macro contêm símbolo de connector/helper ou o checker não consegue classificar seu efeito, falha fechado. Proc-macros que geram código de teste não são expandidas nem presumidas seguras; exigem alteração explícita do contrato C0 e fixtures antes de uso. Menções em comentários/strings não contam como chamadas.

### Fixtures adversariais C0 (sem banco)

A implementação C0 deve fornecer fixtures unitárias isoladas e demonstrar:
- **GREEN:** teste com chamada direta ao helper, módulo inline `#[cfg(test)]`, `#[tokio::test]`, caminho qualificado, e import explícito alias-free percorre o helper e é listado uma vez;
- **GREEN restrito:** somente `core::database::postgres::tests::runtime_marker_guard_against_marked_database`, uma chamada direta a `PostgresDatabase::connect_from_url` com assertion específica `TestDatabaseMarkerPresent`, passa na allowlist exata;
- **RED:** conexão direta por `connect_from_env`, `connect_from_url` ou `PgPoolOptions` dentro do teste, inclusive via `use ... as`, árvore agrupada, chamada em wrapper alcançável, closure ou ponteiro de função;
- **RED:** alias/import wildcard não resolvido, chamada dinâmica, `include!`, módulo de teste `#[path]`, macro com token de connector, atributo/proc-macro de geração desconhecido ou erro de parse;
- **RED:** allowlist com nome/prefixo parecido, caminho de módulo diferente, sem assertion requerida, mais de uma chamada ou uso de `connect_from_env`;
- **RED:** função sem atributo de teste que chama helper, teste omitido/fantasma no manifest e qualquer diferença do conjunto exato.

As fixtures não conectam a Docker/PG e validam o checker, não comportamento do connector. A regra de conexão por helper continua coberta separadamente por testes TDD e pelo runner descartável.

### Nome e escopo da exceção

O nome canônico da exceção passa a ser `core::database::postgres::tests::runtime_marker_guard_against_marked_database` (“teste de guarda runtime/integration”). O propósito e limite permanecem: testar o connector runtime contra somente um database efêmero local marcado e afirmar `DatabaseError::TestDatabaseMarkerPresent`. Atualizar o nome não amplia a allowlist; nomes anteriores neste documento são superseded por este identificador exato.

### Estado

Cobertura algorítmica, limites de sintaxe, fixtures adversariais e nome da exceção estão especificados. O checker ainda não foi implementado nem validado pelas fixtures. **Veredito G1 ciclo 6: APROVADO COM FOLLOW-UP pelo Critic independente**, sobre a versão `5f474e3dc90052be5a7142f94b650de7f2f62f1c`. Follow-ups obrigatórios para G3: implementar o checker junto das fixtures comportamentais e demonstrar rejeição fail-closed de imports e paths externos indiretos que não possam ser resolvidos. O veredito aprova o desenho, não a implementação nem a execução; nenhum teste/DB foi executado. O status geral do SDD permanece `draft` enquanto os follow-ups G3 e os gates posteriores não forem concluídos.

## Proposta de revisão G1 ciclo 7 — remover a exceção de conexão runtime

> Adendo proposto pelo Orquestrador após escalonamento de três ciclos G3. Ele prevalece sobre as allowlists de guarda runtime nos ciclos anteriores se for aprovado. O documento continua `draft`; a revisão independente G1 está pendente. Este adendo não autoriza implementação nem altera o veredito G1 anterior.

### Regra proposta

C0 não terá exceção para teste de guarda runtime/integration. Toda chamada direta a connector PostgreSQL em função de teste ou em wrapper alcançável por uma raiz de teste reprova, incluindo `PostgresDatabase::connect_from_url`, `connect_from_env`, `PgPool::connect`, `PgConnection::connect`, `PgPoolOptions`, métodos genéricos `.connect()` não provados como não-PG, aliases, wrappers, closures e chamadas dinâmicas. A única conexão permitida no grafo de teste é a implementação interna do helper dedicado `database_for_integration_test`, que valida gate, endpoint, identidade, manifesto e marker antes de migrar.

O teste unitário antes descrito como `runtime_marker_guard_against_marked_database` deixa de abrir conexão. O teste do runtime é reduzido a exercitar a validação pura do nome de database (`trading_bot`) sem construir pool ou fazer I/O. O checker não mantém allowlist por nome de função, path, assertion ou fluxo de dados; ausência da allowlist remove também a exceção de dataflow do resultado de conexão.

### Impacto em C0, C1 e C3

- C0 compara o manifesto de helpers e proíbe qualquer connector PostgreSQL direto alcançável a partir de teste; não há função allowlisted.
- A análise alcança chamadas a método `.connect()` dentro de wrappers e fecha a execução quando o receiver/método não puder ser provado não-PG. Exceções não-PG precisam ser expressas por símbolo qualificado e cobertas por fixture (por exemplo, `Neo4jGraph::connect`), sem exceção genérica por nome curto.
- C1 não executa um teste de guarda runtime que conecte diretamente. Os testes PG de integração seguem pelo helper dedicado com identidade efêmera e marcador.
- C3 deve substituir a prova de conexão runtime direta por testes unitários puros das decisões de validação (nome do DB e presença do marker), sem abrir pool/conectar. O contrato de runtime continua recusando banco marcado; a forma de manter cobertura observável dessa recusa sem qualquer conexão direta de teste é uma questão a ser decidida na revisão G1. Não se deve alegar teste end-to-end do caminho SQL até que exista um seam seguro revisado.
- A alteração do teste de nome de database não é uma autorização para remover as validações de runtime: `connect_from_url` continua aplicando o guard de nome, versão e marker antes do uso normal.

### Alternativas, riscos e validação

A exceção anterior permitia um connector runtime direto em teste e exigia provar por AST que a assertion se referia ao retorno daquela chamada. Ela foi removida do desenho porque aumentava a superfície da análise e exigia que a suíte acessasse banco fora do helper normal. O desenho uniforme reduz as regras especiais e mantém o isolamento num único connector.

Risco residual a revisar em G1: testes puros provam a decisão isolada, mas não provam, por si, que o caminho de conexão runtime a invoca. A revisão deve decidir se um seam com dependency injection pode provar a integração sem abrir socket, ou se esse caminho fica coberto apenas pelos testes do runner no container efêmero através do helper. A decisão deve preservar a regra de zero conexões diretas fora do helper.

Fixtures comportamentais exigidas antes de novo G3:
- RED: conector direto em teste, wrapper alcançado, alias e `.connect()` genérico;
- GREEN: conexão via helper dedicado;
- GREEN: `Neo4jGraph::connect` qualificado como não-PG;
- RED: path/nome falso ou tentativa de reintroduzir exceção de runtime;
- RED/GREEN: teste puro de validação de nome e marker sem pool/rede.

### Estado e próxima ação

Este desenho aguarda Critic independente G1. Até o veredito, status permanece `draft`, nenhuma alteração de implementação é autorizada, e o veredito G1 ciclo 6 continua sendo o último veredito técnico registrado (APROVADO COM FOLLOW-UP para o desenho anterior). O código G3 já existente permanece fora do escopo desta atualização documental e deve ser reavaliado somente depois da nova aprovação G1.
