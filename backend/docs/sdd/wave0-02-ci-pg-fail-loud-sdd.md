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
- **C2.** Em modo obrigatório, ausência de `BOT_PG_TEST_DATABASE_URL`, manifest/identidade inválidos, falha de conexão, PostgreSQL < 18, falha de qualquer migração ou ausência/divergência de marcador causam falha do teste/job; mensagem estável não pode revelar URL nem segredo. `DATABASE_URL` sozinho nunca seleciona nem habilita PG integration e não é usado como fallback. Os casos que exigem PostgreSQL seguem o ciclo definido em T-W0-02b: teste congelado em G3, RED observado em G4 antes da correção, correção em G3 e GREEN observado em nova execução G4; casos offline seguem RED/GREEN em G3. Guardar logs/assertions separados e reproduzíveis.
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

## Proposta de revisão G1 ciclo 8 — provar guard runtime sem conexão direta de teste

> Este adendo responde ao veredito REPROVADO do Critic G1 ciclo 7. Prevalece sobre o ciclo 7 e sobre critérios C3 anteriores em conflito. Mantém o status geral em `draft`; G1 está pendente e nenhuma implementação é autorizada por esta proposta.

### Seam de runtime proposto

O guard do marcador será separado em duas operações injetáveis: (1) lookup de estado do marcador e (2) continuação de conexão/bootstrap. A política pura recebe um resultado tipado do lookup e decide: marcador presente retorna `TestDatabaseMarkerPresent` sem chamar o connector; lookup inconclusivo ou erro inesperado falha fechado sem chamar o connector; ausência inequívoca da relação ou relação válida sem linha classifica como `UNMARKED_RUNTIME` e permite continuar pelo connector injetado. A relação ausente é identificada por `to_regclass` retornando SQL NULL; erros de query, permissão, timeout, conexão, formato incompatível, múltiplas linhas ou resultados ambíguos são `MARKER_LOOKUP_FAILED`, nunca ausência.

A injeção deve existir na fronteira interna de conexão/bootstrap, sem expor uma API pública para contornar o guard. Os testes unitários fornecem lookup falso e connector falso com contadores/valores observáveis: presença produz `TestDatabaseMarkerPresent` e zero chamadas ao connector; erro produz falha fechada e zero chamadas; ausência invoca o connector falso e segue o caminho normal simulado. A validação deve cobrir tanto a função que coordena a decisão de conexão quanto os entrypoints `serve`/`bootstrap_http_api`: marker presente ou lookup com erro impede que o bootstrap conclua/inicie o serviço; ausência continua pelo bootstrap injetado. Não abrir socket, não ler `DATABASE_URL`, não usar banco da aplicação nesses testes.

O seam só é válido se o connector real de produção chamar o lookup real antes de criar/usar pool e se os entrypoints runtime compartilharem esse caminho; testes de injeção devem invocar a mesma função de coordenação que produção usa. A revisão de implementação verificará ausência de um bypass por configuração default/flag de teste e que as dependências fake não são expostas em builds de produção.

### Teste comportamental real, restrito ao helper

A camada de query SQL do marcador precisa de uma prova em PostgreSQL descartável. Um teste de integração cria e remove os estados de fixture por SQL através da conexão entregue por `database_for_integration_test` somente: tabela ausente, tabela presente sem linha, exatamente uma linha válida, linha/identidade inválida ou múltiplas linhas. Ele chama a função real de lookup/classificação e verifica os resultados estáveis `UNMARKED_RUNTIME`, `TEST_DATABASE_MARKER_PRESENT` ou `MARKER_LOOKUP_FAILED` conforme o estado. Erros inesperados de query devem ser provocados por fixture controlada (por exemplo, relação com schema/colunas incompatíveis ou permissão negada) e não convertidos em ausência.

Este teste comprova a query SQL e a classificação usando o helper seguro; não chama `PostgresDatabase::connect_from_url`, `connect_from_env`, `PgPool::connect` nem qualquer connector runtime direto. A ligação entre classificação e recusa de conexão runtime é comprovada separadamente pelo seam injetável e pelo mesmo coordenador usado na produção. Nenhum teste acessa o banco persistente da aplicação.

### C3 — critérios substitutivos completos

C3 passa a exigir todos os itens abaixo; trechos anteriores que pedem teste de conexão runtime direta são substituídos:

1. **Classificador puro:** casos para relação ausente e tabela vazia → `UNMARKED_RUNTIME`; exatamente um marker válido → `TEST_DATABASE_MARKER_PRESENT`; dados inválidos/múltiplos/ambíguos/erro de lookup → `MARKER_LOOKUP_FAILED`. Asserções cobrem a variante de erro e a decisão sem rede.
2. **Coordenador da conexão:** com lookup fake presente, retorna `TestDatabaseMarkerPresent` antes do connector (contador do connector permanece zero); com lookup fake em erro, falha fechado antes do connector (contador zero); com lookup fake ausente, chama uma vez o connector fake e retorna seu resultado. Uma falha do connector fake é propagada sem URL/segredo.
3. **Entry points runtime:** testar a função de conexão utilizada por produção e os caminhos `serve`/`bootstrap_http_api` através do seam injetável, verificando que marker presente e erro bloqueiam startup antes de publicar/servir estado; marker ausente permite que bootstrap continue com as dependências fake. Esses casos não usam `DATABASE_URL`, socket, container, nem `PostgresDatabase::connect_from_url` diretamente no teste.
4. **SQL real via helper:** em container PostgreSQL local efêmero validado pelo runner, passar pelas formas de schema/linha listadas em “Teste comportamental real”; exigir resultado SQL real e classificação correta. O runner continua exigindo marker/identidade antes das migrations e o helper é o único ponto de conexão PG de teste.
5. **Regra do checker:** nenhuma exceção de runtime guard. Qualquer connector PG direto em raiz de teste ou wrapper alcançável falha; a única conexão permitida permanece a implementação interna de `database_for_integration_test`. Método genérico `.connect()` só pode ser permitido quando o símbolo qualificado for demonstravelmente não-PG e coberto por fixture. Nenhuma allowlist por path, nome de teste, assertion ou dataflow.

### Matriz TDD proposta

| Comportamento | RED sem I/O | GREEN esperado | Regressão de segurança |
|---|---|---|---|
| Resultado do lookup | Fixtures puras para ausência, presença, erro, formato inválido e multiplicidade | Classificador retorna os três estados tipados corretamente | Erro nunca é interpretado como ausência |
| Conexão de runtime | Fake lookup + fake connector com contador | Presença → erro dedicado/0 connect; erro → fail-closed/0 connect; ausência → connector fake chamado uma vez | Produção e teste usam o mesmo coordenador; nada de allowlist direta |
| `serve`/`bootstrap_http_api` | Bootstrap com lookup/connector fake | Marker/erro impedem startup; ausência permite prosseguir pelo seam | Nenhuma conexão ou leitura de env nos testes |
| Query SQL marker | Teste de integração pelo helper dedicado, primeiro falhando para cada estado SQL | Tabela ausente/vazia, marker válido e estados inválidos classificam conforme contrato | Runner valida container/identity/marker antes de migration; sem connector PG fora do helper |
| Manifest checker | Fixtures com connector direto e wrappers alcançáveis | Rejeita todos; helper interno segue permitido | Checker canônico no gate propaga exit code não-zero |

Para seams comportamentais offline, G3 registra RED reproduzível antes da implementação e GREEN após a correção mínima. Para o lookup SQL, G3 escreve e congela o teste/fixture sem serviço; G4 observa RED pré-implementação no runner efêmero aprovado, retorna a G3 para a correção, e reexecuta em G4 para GREEN. Manter essas evidências separadas. Nunca usar DB da aplicação, Docker de terceiros ou exchange.

### Alternativas e riscos

- **Exceção para o teste de guard conectar diretamente:** rejeitada; reintroduziria a superfície que causou os ciclos G3 e exigiria exceção AST/dados difíceis de provar.
- **Somente classificador puro, sem seam no caminho runtime:** rejeitada; provaria a regra isolada, mas não que `connect_from_url` ou bootstrap a invoca.
- **Somente integração via helper, sem injeção:** rejeitada; provaria SQL/helper, mas abriria conexão direta de runtime pelo teste ou deixaria os entrypoints runtime sem teste observável.
- **Risco residual:** os testes fake comprovam a chamada ao coordenador e a propagação das decisões, não a infraestrutura de rede real de cada entrypoint. A query SQL real pelo helper fecha a lacuna de parsing/estado do marcador; revisão de código deve confirmar que toda construção de pool runtime passa pelo coordenador. O checker continua necessário como gate contra bypasses.
- **Risco de fixture destrutiva:** a tabela/linhas de marker são alteradas somente no banco identificado como efêmero pelo runner; cada teste restaura o estado ou usa transação isolada. Cleanup limitado ao container do run. Falha de setup ou identidade encerra antes de SQL destrutivo/migration.

### Rollout, rollback e gates

A implementação só começa após Critic independente aprovar este desenho G1. Em G3, primeiro entram fixtures offline RED/GREEN do classificador, coordinator e bootstrap; também se escreve/congela o teste SQL do marker via helper, sem PostgreSQL. Critic G3 revisa a implementação e o teste congelado antes da execução de integração. Em G4, no runner efêmero seguro, executa-se o teste congelado uma vez contra o estado pré-correção para observar RED; esse resultado retorna a G3 para a correção mínima. Após a revisão G3, G4 executa o mesmo teste para GREEN. Cada falha comportamental de G4 reabre G3 e requer nova execução G4; falha de setup não conta como RED. Integrar fixtures offline ao checker/gate canônico e registrar evidências de G3 e G4 separadamente.

O rollout local permanece opt-in e limitado ao container efêmero com identity/manifest/marker validados; C4 do service container GitHub continua gate separado, exigindo evidência real do workflow. Nenhuma inicialização do serviço da aplicação, alteração de workflow, execução CI, migração do banco persistente, Testnet ou deploy faz parte desta revisão documental.

Rollback da implementação futura reverte coordenador/injeção, query e testes como unidade, sem alterar banco de aplicação. Se a execução efêmera deixar container órfão, remover somente o container/volume com UUID do run depois de validar sua identidade; nunca aplicar cleanup por nome de banco genérico.

### Estado e próxima ação

Esta é uma proposta para responder aos dois findings G1 ciclo 7: seam sem socket que conecta decisão e entrypoints runtime, e reconstrução de C3 sem connector direto em teste, complementada por SQL real via helper. O status permanece `draft`; nenhum finding é declarado fechado até Critic independente rever e aprovar o ciclo 8. O último veredito registrado segue sendo REPROVADO no ciclo 7. Implementação, PG, Docker e Testnet permanecem bloqueados até os gates correspondentes.

## Proposta de revisão G1 ciclo 9 — preflight SQL antes do pool e composição única

> Este adendo responde ao follow-up do Critic G1 ciclo 8: o ciclo 8 descrevia um seam fake, mas não fixava como a conexão runtime obtém o estado SQL antes de criar o pool da aplicação nem como os entrypoints provam que não o contornam. Prevalece sobre o ciclo 8 nesses pontos. Mantém status `draft`; não aprova G1 nem autoriza implementação.

### Preflight SQL de runtime

Quando a configuração pede PostgreSQL, o único coordenador runtime de banco executa esta sequência, na ordem indicada:

1. valida a configuração/URL e a política de nome/versionamento já definida, sem iniciar pool;
2. abre uma conexão SQLx transitória `PgConnection` com a mesma URL e timeout limitado, executa somente o lookup do marcador e fecha a conexão;
3. classifica o resultado: marker válido presente → `TestDatabaseMarkerPresent`; tabela ausente ou presente vazia → `UNMARKED_RUNTIME`; qualquer erro inesperado, schema incompatível, dado inválido/múltiplo ou resposta ambígua → `MARKER_LOOKUP_FAILED` e aborta;
4. somente para `UNMARKED_RUNTIME`, cria o `PgPool` da aplicação e então prossegue com migrations/bootstrap já previstos.

O preflight não cria nem migra schema, não executa query de aplicação e não usa `PgPool`. Ele consulta `to_regclass('public.bot_test_database_marker')` para distinguir ausência inequívoca e lê o marker apenas quando a relação existe. Timeout, perda de conexão, permissão negada e qualquer erro SQL não podem ser convertidos em marker ausente. A conexão transitória é sempre fechada antes do pool; erro ao fechar também é registrado como falha de preflight e não libera a criação do pool.

**Limite de concorrência:** o marker é um sinal de classificação do alvo no bootstrap, não uma defesa contra um principal privilegiado que altere simultaneamente o marker. O banco runtime usa papel sem permissão de criar/alterar/remover a tabela/linha; apenas setup externo controlado pode provisionar marker. A janela entre preflight e pool fica limitada a um único bootstrap e ao timeout configurado. Se G1 considerar esse modelo insuficiente, alternativa a avaliar é preservar o preflight e obter lock compartilhado de inicialização, sem abrir exceção de connector direto em testes. Não se presume que o marker sozinho prova identidade/descartabilidade: a prova do runner local continua sendo endpoint dedicado + identidade do container + digest/UUID/porta/database + marker.

### Boundary única para runtime e testes injetáveis

Há um só coordenador privado que implementa a sequência de preflight, classificação e criação do pool. A implementação de produção fornece dois adaptadores concretos: lookup SQL via conexão transitória e factory do pool. `serve` e `bootstrap_http_api` chamam esse coordenador; nenhum entrypoint cria pool diretamente, consulta marker após pool ou continua bootstrap com pool quando preflight falhou.

O seam de teste injeta lookup e pool factory nesse mesmo coordenador. Os fakes não abrem rede. A composição real do entrypoint fornece sempre os adaptadores de produção e não lê flag/config para desativar guard; a injeção é privada ao módulo e disponível apenas aos testes/fixtures de unidade, sem exportação em build público. Testes de `serve`/`bootstrap_http_api` invocam a mesma função comum de inicialização com fakes e afirmam: marker presente e lookup error retornam erro antes da factory do pool e antes de montar/servir API; marker ausente chama a factory uma vez e continua o bootstrap simulado. Teste da função de conexão exercita essa mesma ordenação.

Para provar que os entrypoints não contornam a boundary, o C0/checker deve impor a fronteira estrutural: catálogo de criação de pool PostgreSQL inclui `PgPoolOptions::connect`, `PgPool::connect`, `PgConnection::connect`/equivalentes; em fontes de runtime, criação de pool só é permitida no factory/coordenador autorizado; a conexão transitória `PgConnection::connect` só é permitida no adaptador de preflight autorizado; `serve`/bootstrap só podem alcançar o coordenador. Em código de teste, qualquer conector PG continua proibido fora do corpo interno do helper `database_for_integration_test`; não se reintroduz a exceção do teste de guarda. Método genérico `.connect()` em código alcançável por teste deve ser resolvido como não-PG qualificado e coberto por fixture, caso contrário falha fechado. Se o checker não consegue provar o módulo/call graph/import, ele falha fechado com localização. A lista exata de módulos autorizados é definida e versionada no manifesto, sem allowlist por substring.

### C3 — substituição e matriz TDD atualizada

C3 ciclo 9 substitui qualquer redação antiga que sugira chamar connector runtime direto em teste e completa os critérios do ciclo 8:

- **Lookup SQL transitório:** integração PostgreSQL verifica a query de marker por todos os estados (tabela ausente, vazia, uma linha válida, identidade/dados inválidos, múltiplas linhas, schema incompatível e falha controlada de permissão/query). Toda conexão e SQL de fixture passam por `database_for_integration_test`; o teste chama a função real do lookup/classificador e não abre connector runtime.
- **Ordenação do coordenador:** fakes registram chamadas. Presente → erro dedicado, zero pool factory; erro SQL/lookup → fail-closed, zero pool factory; ausente → pool factory uma vez e resultado propagado; falha ao criar pool → propagada sem expor URL. Nenhum pool é criado antes do lookup.
- **Entry points:** `serve` e `bootstrap_http_api` devem seguir o mesmo coordenador privado. Testes com fakes provam que presente/erro bloqueiam startup antes de estado HTTP/serving e ausente alcança bootstrap depois de exatamente uma factory. Um teste estrutural/C0 comprova que os entrypoints não referenciam connector/factory alternativo.
- **Produção:** teste/inspeção do coordenador confirma que o adaptador concreto executa o preflight com timeout, fecha a conexão transitória e só então constrói o pool; preflight error nunca aciona factory. Esse critério não será alegado como testado por uma conexão direta de teste; é coberto pelo desenho do coordenador injetável, consulta SQL via helper e regra estrutural do checker.
- **Checker:** fixtures RED para pool/conector em `serve`, `bootstrap_http_api`, wrappers/import alias e factory alternativa; GREEN para ambos entrypoints delegando exclusivamente ao coordenador; RED para conexão PG direta em qualquer raiz de teste; GREEN restrito para SQL de teste pelo helper.

| Fatia | RED / verificação adversarial | GREEN / prova | Gate de segurança |
|---|---|---|---|
| SQL do marker | Estado SQL inválido ou consulta falhando não pode resultar em `UNMARKED_RUNTIME` | Lookup/classificador real reconhece ausente/vazio/presente corretamente via helper | Container efêmero validado antes de SQL; sem runtime connector em teste |
| Preflight/coordinator | Present/error deve demonstrar contador de pool factory zero | Ausente faz lookup → close → factory exatamente nessa ordem | Falha no lookup/close interrompe antes de pool/migration |
| Entry points | Fixture de bypass/factory alternativa deve falhar C0; fake presente/error não inicia API | `serve` e `bootstrap_http_api` usam o coordinator e fake absent continua bootstrap | Nenhum endpoint runtime chama pool ou pula guard diretamente |
| Checker estrutural | Conectores diretos em testes ou caminho runtime alternativo falham | Boundary de production connector é única e explicitamente qualificada | Fail-closed em unresolved call/import/path |

Para cada comportamento executável, TDD registra RED primeiro e GREEN mínimo depois. Suites do checker e fakes são locais e não iniciam serviço. Só teste SQL real usa runner PG, e apenas após G1 e G3 revisarem a segurança do runner; nunca usar database persistente de aplicação.

### Alternativas, riscos e rollback

- **Criar pool e consultar marker através dele:** rejeitada; viola o requisito de consulta antes de criar/usar pool da aplicação.
- **Usar conexão runtime direta dentro do teste para provar recusa:** rejeitada; viola a regra uniforme do checker e cria endpoint bypassável.
- **Pré-flight não falível (erro tratado como ausente):** rejeitada; timeout, permissão ou rede falha poderia liberar acesso a banco marcado.
- **Risco residual:** conexão transitória e pool são conexões distintas. Marker/ACL não deve ser mutável pelo papel da aplicação; se existir possibilidade normal de mutação entre preflight e pool, G1 deve exigir serialização de bootstrap/lock revisado antes de implementação. O marker é adicional à identidade descartável do runner, nunca sua substituição.
- **Rollback futuro:** reverter adaptador de preflight, coordinator e wiring de entrypoints juntos. Não relaxar regra de checker nem deixar `serve` em caminho parcialmente antigo; rollback não toca schema, marcador ou dados do banco da aplicação.

### Estado e próxima ação

Este ciclo 9 propõe um mecanismo concreto: conexão transitória SQL de preflight, classificação fail-closed antes do pool, coordinator privado compartilhado e adaptadores fake só no seam de teste, mais prova estrutural de que `serve`/`bootstrap_http_api` não contornam a boundary. O status permanece `draft`. O veredito ciclo 8 permanece APROVADO COM FOLLOW-UP; este follow-up técnico requer Critic G1 independente. Nenhum código, teste, banco, Docker, CI ou exchange foi executado nesta atualização documental.

## Item escalado T-W0-02b — lookup SQL comportamental e opt-ins PG centralizados

> Novo item de design após o escalonamento do W0-02 no limite de ciclos anterior. Não é “ciclo 4” e não substitui nem renumera os vereditos históricos. Responde somente aos dois blockers listados abaixo. Este adendo prevalece quando o texto anterior deixar indefinidos o teste do lookup/classificador real ou a origem das configurações de opt-in. Status geral continua `draft`; G1 independente está pendente. Não autoriza G3.

### Problema e limites

O desenho anterior define estados de marcador e pede lookup SQL via helper, mas ainda precisa fixar uma matriz comportamental que invoque o lookup/classificador real para todos os estados apontados pelo Critic. Também descreve flags de integração sem definir uma única fronteira de leitura de ambiente; atualmente `pg_integration.rs` contém `std::env::var`, incompatível com `verify-backend-gates.sh`, que restringe essas leituras à camada de configuração.

Este item não muda o contrato de isolamento, o manifesto, o runner, a semântica runtime do marcador, os comandos de CI, nem a autoridade de G3/G4. Não editar código, testes, workflow ou configuração executável neste item. Nenhum PostgreSQL, Docker, CI ou exchange será acessado nesta revisão documental.

### Lookup/classificador real: seam e fixture comportamental

O teste PostgreSQL chama a implementação real da consulta e classificação do marcador usando exclusivamente uma conexão obtida por `database_for_integration_test()`. Não basta testar o classificador puro, duplicar a query na fixture ou afirmar cobertura a partir dos testes fake do coordenador. A função exercitada deve ser a mesma usada pelo preflight runtime; o seam de teste só fornece a conexão já segura e a fixture. O teste não chama connector runtime nem cria outro pool.

Usar uma transação por cenário dentro do banco efêmero identificado pelo runner. Dentro da transação, ajustar o estado do marcador e chamar o lookup real na mesma conexão; em seguida fazer rollback para restaurar o marcador canônico provisionado pelo runner. A fixture deve falhar antes de executar SQL destrutivo se a conexão não veio do helper seguro. Não usar nome de banco genérico nem executar fixture no banco de aplicação. Se o driver/conexão do helper não permitir executar lookup e fixture na mesma transação/conexão, o desenho deve voltar à G1 com um seam de conexão transacional equivalente; não duplicar a query nem usar um connector alternativo.

| Estado preparado na transação efêmera | Ação observável do lookup real | Resultado contratual |
|---|---|---|
| Relação ausente | remover a relação canônica somente dentro da transação | `UNMARKED_RUNTIME` |
| Relação correta sem linhas | criar/limpar a relação conforme o schema canônico | `UNMARKED_RUNTIME` |
| Uma linha válida e completa | inserir a identidade esperada pelo contrato do marcador | `TEST_DATABASE_MARKER_PRESENT` |
| Uma linha com identidade/dado inválido | usar valor que viole o contrato de identidade | `MARKER_LOOKUP_FAILED` |
| Duas ou mais linhas | inserir multiplicidade deliberada | `MARKER_LOOKUP_FAILED` |
| Relação com schema/colunas incompatíveis | criar a relação com estrutura incompatível com a query real | `MARKER_LOOKUP_FAILED` |
| Erro de query reproduzível | provocar falha SQL controlada (por exemplo, permissão negada na relação) | `MARKER_LOOKUP_FAILED` |

As fixtures usam o DDL/colunas canônicos já definidos para o marcador e o mesmo identificador de relação que a query de produção. Cenários que removem ou alteram a relação são transacionais e restaurados por rollback; a limpeza nunca é feita por conexão separada. Cada caso afirma o estado tipado, não apenas “não panicou”. Para o erro SQL, afirmar que a falha foi observada e convertida no estado de erro estável, sem mensagem contendo URL, credencial ou SQL sensível. O runtime segue fail-closed: somente ausência inequívoca da relação ou relação válida vazia permite `UNMARKED_RUNTIME`; lookup falho nunca libera a factory/pool.

A matriz também mantém testes unitários sem I/O para o classificador sobre resultados tipados e testes com lookup/factory falsos para a ordenação do coordenador. Estes complementam, mas não substituem, os sete casos do lookup SQL real.

### Configuração centralizada dos opt-ins PG

Toda leitura de variáveis de ambiente ocorre na camada central `core/config` / parser de ambiente já reconhecida por `verify-backend-gates.sh`. O módulo `core/persistence/pg_integration.rs` não chama `std::env::var`, `std::env::var_os` nem lê ambiente indiretamente; recebe um snapshot imutável e tipado de configuração PG criado pela fronteira central. O helper e as funções internas de resolução recebem esse snapshot explicitamente, permitindo unit tests determinísticos sem modificar ambiente global do processo.

O snapshot imutável deve incluir todas as entradas consumidas em `core/persistence/pg_integration.rs`: `BOT_PG_INTEGRATION_REQUIRED`, `BOT_RUN_PG_INTEGRATION`, `BOT_PG_TEST_DATABASE_URL`, `BOT_PG_TEST_TARGET_MANIFEST`, presença de `DOCKER_HOST`/`DOCKER_CONTEXT`, `BOT_RUN_BINANCE_TESTNET_ORDER` e `BOT_RUN_NEO4J_INTEGRATION`. A origem de cada valor é exclusivamente um reader/parser central em `core/config` (por exemplo, `env_parse.rs`), autorizado pelo guard estático; nenhum `std::env::var`, `var_os` ou acesso equivalente permanece no módulo de persistência, nem sequer encapsulado por helper local. O helper obtém um `PgIntegrationSettings` tipado da fronteira central; as funções de PG, Testnet e Neo4j recebem esse snapshot/configuração como entrada e não leem ambiente. URL de teste e caminho do manifest são dados privados e redigidos, nunca impressos. `DATABASE_URL` não é fallback e não participa da resolução. Nenhum default ativa PG/Testnet/Neo4j integration. O guard não ganha allowlist para persistence.

Contrato do resolver: sem opt-in, o helper mantém o comportamento local de skip existente. Com required ligado, ausência de opt-in explícito, URL dedicada, manifest ou identidade válidos falha alto e antes de connector/migration. Com os dois opt-ins e configuração válida, prossegue apenas no runner/target já validado pelos critérios C0–C4. A composição de produção não deve permitir caller omitir silenciosamente required/opt-in; qualquer caminho de teste continua passando pelo mesmo helper público de integração e configuração central.

O gate estático existente `verify-backend-gates.sh` é critério executável: nenhuma leitura de processo (`env::var`, `env::var_os` ou equivalente reconhecido pelo guard) em `src` fora da camada de configuração permitida. Não ampliar a allowlist para `core/persistence`. Os opt-ins Binance Testnet e Neo4j acima fazem parte do snapshot central porque são consumidos pela mesma unidade PG; não constituem exceção de caminho nem novo opt-in externo. A semântica atual permanece: PG só é obrigatório quando `BOT_PG_INTEGRATION_REQUIRED=1` e exige `BOT_RUN_PG_INTEGRATION=1`; ausência/`0` opt-out mantém skip; valores PG inválidos falham com mensagem estável. Binance Testnet e Neo4j permanecem opt-in somente para o valor exato `1` e só resolvem credenciais/configuração após esse opt-in; ausente/`0`/outro valor não habilita integração e mantém false/skip, conforme contratos atuais.

### TDD e critérios observáveis

G1 aprova o seam público de configuração e teste descrito neste SDD; G3 implementa cada fatia com RED/GREEN antes de refatorar. Testes unitários com snapshot tipado usam resoluções puras e fábricas fake, sem ler ambiente, abrir socket ou iniciar serviço.

| Fatia pública | RED exigido antes da implementação | GREEN / evidência exigida |
|---|---|---|
| Snapshot de configuração PG | valores ausente, vazio, formatos de opt-in não aceitos e configuração parcial não produzem resolução válida | parser central produz snapshot tipado, sem segredos em Debug/Display/logs |
| Resolução do helper | sem opt-in mantém skip; required sem opt-in/url/manifest falha antes do connector | configuração válida chega ao lookup apenas após os gates já definidos |
| Lookup real via helper | Em G3, escrever/congelar o teste SQL dos sete cenários e asserções esperadas, sem executá-lo; em G4 pré-implementação, runner efêmero observa RED para a falha real | após retorno a G3 e correção mínima, G4 reexecuta o mesmo teste no runner e observa GREEN; cada fixture é revertida na transação |
| Fail-closed do coordenador | marker presente ou erro SQL não deve chamar factory; teste verifica contador igual a zero | somente ausência inequívoca chama factory uma vez; erro de factory é propagado sem URL |
| Guard estrutural de ambiente | Fixture de source tree injeta `std::env::var` e `std::env::var_os` em `core/persistence/pg_integration.rs`, inclusive para cada opt-in | o mesmo predicado usado por `verify-backend-gates.sh` rejeita todos; fixture com leitura em `core/config/env_parse.rs` é aceita; nenhuma allowlist de persistência |
| Compatibilidade | opt-in desligado preserva comportamento de skip para testes locais atuais | testes de unidade provam compatibilidade sem PostgreSQL e sem variável global compartilhada |

A prova do lookup SQL é integração e pertence exclusivamente a G4 no runner efêmero seguro aprovado. Em G3, o Builder escreve e congela o teste e as fixtures SQL com expectativas observáveis, mas não executa PostgreSQL real; Critic G3 revisa essa fatia antes de G4. Primeiro, G4 executa o teste congelado contra a implementação ainda sem a correção e registra RED observado. Esse RED retorna o item à G3: o Builder implementa a correção mínima e executa as provas offline permitidas; o Critic revisa o diff/teste. Então G4 reexecuta o mesmo teste SQL e registra GREEN. Qualquer falha ou inconclusão em G4 reabre G3 para correção e exige nova execução G4 até GREEN; problema de provisionamento/identidade do runner é falha de setup, não evidência RED do comportamento. A mera aprovação G1 não autoriza Docker, banco, CI nem exchange. G4 mantém registro separado do teste SQL (comando, resultado, identity/manifest efêmero) e das evidências offline de G3 (testes unitários/fakes e gate `verify-backend-gates.sh`). Não alegar execução nesta revisão documental.

### Alternativas, riscos e rollback

- **Só testar o classificador puro:** rejeitada como cobertura única; não prova que a query real lê ausente/vazio/presente nem converte erro SQL/schema/multiplicidade.
- **Duplicar SQL de produção dentro do teste:** rejeitada; teste poderia continuar verde com query de produção quebrada.
- **Conectar diretamente por runtime connector ou URL do ambiente no teste:** rejeitada; contorna helper/manifest/identity e contraria C0/C3.
- **Manter `std::env::var` no helper e ampliar allowlist do gate:** rejeitada; perpetua configuração dispersa e invalida a proteção que o script pretende oferecer.
- **Ler ambiente dentro de cada teste:** rejeitada; torna estado dependente de execução paralela e impede snapshot determinístico. A camada central lê uma vez e injeta estado imutável.
- **Risco de fixture/concorrência:** alteração transacional do marcador só é segura no banco efêmero exclusivo e na conexão validada pelo helper. O runner deve continuar isolando cada job; fixtures não podem ser paralelizadas se partilharem uma relação sem isolamento transacional correto. Se o banco ou conexão não suportarem restauração transacional, abortar o teste sem persistir alteração.
- **Risco de compatibilidade:** move a fonte de flags do helper para config central, mas preserva nomes/semântica do opt-in externo; testes locais sem opt-in continuam em skip. Não muda DATABASE_URL nem comportamento de produção.
- **Rollback futuro:** reverter em conjunto o snapshot/parsing central, injeção no helper e fixtures. Não reintroduzir env reads em persistence nem relaxar `verify-backend-gates.sh`; restaurar a versão anterior do gate e helper no mesmo commit caso a implementação precise ser revertida. Sem migração persistente ou alteração de dados de aplicação.

### Gates e estado

A próxima ação é revisão independente de G1 deste item escalado. Critérios de aprovação observáveis: a mesma função real de lookup é coberta pelos sete estados SQL através do helper; os testes são transacionais e não abrem connector próprio; config tipada é lida centralmente e injetada; nenhuma exceção de `verify-backend-gates.sh` permite env reads em persistence; RED/GREEN e fronteira G3/G4 estão explícitos.

Até G1 aprovar, permanecem bloqueados código, testes, execução PG, Docker, CI e alteração de workflow. Se aprovado, G3 fica limitado a testes unitários/fakes, implementação e verificação estática; somente G4 no runner seguro executa SQL real. Nada neste adendo declara blockers resolvidos antes do parecer Critic.

## T-W0-02b re-review delta — snapshot completo e gate estrutural

> Este delta responde ao blocker do Critic sobre opt-ins omitidos. Prevalece sobre o trecho anterior que limitava o snapshot ao PG e sobre qualquer redação que isente opt-ins de subsistemas. Mantém o item escalado T-W0-02b; não o numera como ciclo 4. Sem aprovação G1 ainda.

### Fronteira única de ambiente

A classe/configuração central em core/config (reader autorizado pelo gate, como env_parse.rs) é a única unidade que lê o processo. Ela cria um snapshot tipado imutável, conceitualmente PgIntegrationSettings, cobrindo todos os valores consumidos direta ou indiretamente pelo módulo core/persistence/pg_integration.rs:

| Entrada de ambiente | Valor do snapshot | Semântica preservada |
|---|---|---|
| BOT_PG_INTEGRATION_REQUIRED | opt-in obrigatório | Required=1 exige BOT_RUN_PG_INTEGRATION=1; falha com mensagem estável se não habilitado |
| BOT_RUN_PG_INTEGRATION | estado do opt-in PG | ausente/0 mantém skip; 1 habilita; outro valor é configuração inválida |
| BOT_PG_TEST_DATABASE_URL | URL dedicada de integração | usada somente após opt-in; nunca DATABASE_URL |
| BOT_PG_TEST_TARGET_MANIFEST | caminho de identidade do runner | lido/usado somente após opt-in |
| DOCKER_HOST e DOCKER_CONTEXT | indicadores de override presente/ausente | qualquer override bloqueia a prova de daemon local |
| BOT_RUN_BINANCE_TESTNET_ORDER | estado de opt-in testnet | somente valor exato 1 tenta resolver credenciais; credenciais ausentes ou inválidas preservam false; ausente/0/outro valor mantém false sem resolução |
| BOT_RUN_NEO4J_INTEGRATION | estado de opt-in Neo4j | somente valor exato 1 tenta resolver configuração do stack; stack ausente/inválido preserva false; ausente/0/outro valor mantém false sem resolução |

A construção do snapshot não habilita os efeitos. Opt-ins são avaliados pelas funções de domínio já existentes usando os valores do snapshot; resolução de credenciais Testnet e leitura/validação de configuração Neo4j só ocorre após opt-in exato. Valores secretos (PG URL, credenciais) ficam em campos privados redigidos. Snapshot e resolvers de teste são passados explicitamente às funções puras para evitar leitura de estado global em testes paralelos.

O módulo pg_integration.rs não lê ambiente por mecanismo algum: nenhum std::env::var/var_os, env!, macro, reader encapsulado localmente, nem chamada a helper local que consulte processo. Seus entrypoints solicitam o snapshot à única fábrica de core/config; os resolvers internos recebem-no como parâmetro e fazem parse/decisão sem efeitos. Assim todos os caminhos e os dois opt-ins antes omitidos usam a mesma fonte central. O static guard não ganha exceção para core/persistence.

### Prova estrutural de verify-backend-gates.sh

O guard estático examina toda a árvore src e aceita leituras de ambiente somente na allowlist central de configuração existente. Ele deve reconhecer pelo menos env::var e env::var_os, inclusive caminhos qualificados std::env::..., sem filtrar por nome da variável nem isentar pg_integration.rs. O teste estrutural exercita o mesmo predicado/guard com uma source tree fixture isolada:

- RED: var e var_os em core/persistence/pg_integration.rs falham; cobrir um exemplar por variável opt-in (inclusive BOT_RUN_BINANCE_TESTNET_ORDER e BOT_RUN_NEO4J_INTEGRATION), mais uma fixture com chamada indireta proibida.
- GREEN: a leitura central equivalente sob core/config/env_parse.rs é permitida; nenhum arquivo em persistence recebe exceção.
- O teste prova tanto que o guard detecta novos acessos como que o allowlist legítimo permanece restrito. Não precisa de ambiente real, credenciais, DB, Docker ou exchange.
- Depois de autorizado G3, verify-backend-gates.sh deve executar esse teste estrutural e passar na árvore real sem qualquer leitura ambiental fora de core/config. Não relaxar nem ampliar regex/allowlist para tornar o gate verde.

### TDD, compatibilidade e gates

G1 aprova o contrato do snapshot e a semântica das três famílias de opt-in antes de alteração de código/teste. Em G3, RED/GREEN offline comprova: PG required/opt-in; Binance Testnet somente opt-in literal 1 mais credenciais configuradas; Neo4j somente opt-in literal 1 mais stack ativo; ambos permanecem falsos e não resolvem configuração quando o opt-in não é 1; resolução via snapshot equivale ao comportamento atual. Um spy de resolver confirma zero chamadas de credenciais/config quando opt-out. O teste estrutural do guard roda offline e usa fixtures sintéticas.

G3 não executa conexão PG, inicia Docker, workflow/CI externo ou exchange. A execução SQL real permanece exclusivamente G4 no runner efêmero aprovado, pelos sete casos documentados acima. Compatibilidade opt-out é demonstrada em teste puro, sem ler ou modificar variáveis globais.

Alternativa rejeitada: mover somente os opt-ins PG e manter Binance/Neo4j lendo std::env em pg_integration.rs; isso deixa o gate quebrado e mantém uma única unidade com várias fontes de configuração. Também rejeitado ampliar a exceção do verify-backend-gates.sh para esse arquivo: o gate deve demonstrar que todos os env reads ficaram centralizados, não autorizar a dispersão.

O status continua draft/G1 pendente. Critérios observáveis para fechar o finding: snapshot inclui as oito entradas acima; comportamento exato de opt-in preservado; nenhum acesso direto/indireto local à env em pg_integration.rs; teste estrutural prova as negativas para var e var_os e só autoriza core/config; verify-backend-gates.sh fica verde sem exceção nova. Nenhuma implementação ou execução do gate/testes ocorreu nesta atualização documental.
## T-W0-02b G1 re-review — correção dos findings atuais

> Esta revisão documental responde aos findings G1 do Critic sobre marker vazio, opt-ins solicitados mas inválidos e herança de `DATABASE_URL`. Ela prevalece sobre qualquer trecho anterior deste SDD em conflito, inclusive propostas antigas que tratavam relação vazia como ausente ou convertiam falhas pós-opt-in em `false`/skip. O estado geral continua `draft`, G1 deste item escalado pendente e implementação proibida. O veredito G1 ciclo 6/hash `5f474e3dc90052be5a7142f94b650de7f2f62f1c` cobre outro escopo histórico e não aprova esta revisão. W0-06 continua dependência conforme já definida; este delta não altera sua fronteira nem seu estado.

### Marcador: ausência requer identidade independente; vazio falha fechado

A função real de lookup/classificação continua sendo chamada pelos sete casos SQL transacionais via `database_for_integration_test()` e é a mesma função usada no preflight runtime. A identidade do alvo runtime deve ser validada independentemente do conteúdo do marker antes de qualquer abertura de conexão SQLx; o marker não prova identidade. Identidade não verificada resulta em falha antes do connector, pool, migration e startup.

| Estado retornado pelo lookup real | Classificação | Decisão runtime antes de pool/migration |
|---|---|---|
| Relação ausente (`to_regclass` retorna SQL NULL) com identidade independente verificada | `UNMARKED_RUNTIME` | prosseguir somente com o alvo já validado |
| Relação ausente sem identidade independente válida | `MARKER_LOOKUP_FAILED` | fail-closed; zero connector, pool, migration e startup |
| Relação presente, mas vazia | `MARKER_LOOKUP_FAILED` | fail-closed; zero pool, migration e startup |
| Exatamente uma linha com marker completo e válido | `TEST_DATABASE_MARKER_PRESENT` | recusar runtime; zero pool, migration e startup |
| Linha inválida, múltiplas linhas, schema incompatível, permissão/timeout/conexão/query error ou resultado ambíguo | `MARKER_LOOKUP_FAILED` | fail-closed; zero pool, migration e startup |

A tabela anterior que agrupava ausência e relação vazia como `UNMARKED_RUNTIME` fica substituída por esta. Nos sete cenários SQL, a relação ausente é exercitada com identidade verificada (resultado unmarked) e identidade ausente/inválida (falha sem connector); a relação presente vazia agora deve produzir `MARKER_LOOKUP_FAILED`; os outros cinco cenários mantêm suas classificações fail-closed/presente conforme o contrato. As alterações de fixture seguem na transação do banco efêmero validado, com rollback; nunca são feitas no DB da aplicação.

### Opt-out separado de opt-in inválido

O snapshot tipado `PgIntegrationSettings`, criado somente em `core/config`, inclui as flags PG, `BOT_RUN_NEO4J_READ_INTEGRATION`, `BOT_RUN_NEO4J_WRITE_INTEGRATION`, `BOT_RUN_BINANCE_TESTNET_ORDER`, estado de credenciais/configuração necessárias, presença de `DOCKER_HOST`/`DOCKER_CONTEXT` e `database_url_present`. `BOT_RUN_NEO4J_INTEGRATION` fica superseded por gates READ e WRITE independentes. `core/persistence/pg_integration.rs` continua sem leituras diretas/indiretas de ambiente; o gate estrutural não recebe allowlist nova.

Para cada gate, ausência ou valor explícito `0` significa opt-out: skip/ignored sem resolver credenciais, construir client/driver, conectar ou executar query/assertion. O valor literal `1` significa que o efeito foi solicitado e então obriga validação completa: config presente e válida, target efêmero/identidade válidos, credenciais completas, conexão correta e permissões adequadas. Se qualquer requisito falhar após opt-in, o resultado é erro não-zero; jamais rebaixar para `false`, `None` ou skip silencioso. Qualquer outro valor do gate é erro de configuração não-zero.

| Integração | Opt-out | Opt-in `=1` com requisito inválido |
|---|---|---|
| Neo4j READ (`BOT_RUN_NEO4J_READ_INTEGRATION`) | sem config/credencial/driver/conexão; teste ignored | config, endpoint/identidade, credencial, conexão ou prova read-only inválidos/inconclusivos → erro não-zero antes de query/assertion |
| Neo4j WRITE (`BOT_RUN_NEO4J_WRITE_INTEGRATION`) | sem config/credencial/driver/conexão; teste ignored | target efêmero, credencial, conexão ou permissão de escrita inválidos → erro não-zero antes do teste; gate READ não autoriza WRITE |
| Binance Spot Testnet (`BOT_RUN_BINANCE_TESTNET_ORDER`) | sem resolver credencial, client, transport ou submit | chave/secret Testnet ausente/parcial/inválida, target ou conexão inválidos → erro não-zero; nunca usar chaves/endpoint Live |

READ conserva o seam de segurança definido em W0-06: pré-flight comprova negação de escrita no servidor com o mesmo principal/driver/sessão antes das queries/assertions. Falha de permissão, conexão, identidade ou resultado inconclusivo após READ opt-in é erro e não skip. Binance continua limitada ao Spot Testnet. O estado de W0-06 permanece uma dependência separada e inalterada.

### `DATABASE_URL` herdada e URL dedicada

A camada central lê apenas a presença de `DATABASE_URL` e armazena no snapshot um bit booleano; não copia, faz parse, exibe ou usa seu valor. Se a variável estiver presente no processo do runner PG, mesmo vazia e mesmo que `BOT_PG_TEST_DATABASE_URL` esteja configurada, a resolução falha antes de qualquer connector SQLx, pool, migration ou query. A única URL de destino permitida para integração é `BOT_PG_TEST_DATABASE_URL`, após opt-in e identidade/manifest válidos. `DATABASE_URL` jamais é fallback, seletor ou valor alternativo.

### TDD e fronteira G3/G4

Em G3, testes puros do snapshot/resolver demonstram: `database_url_present=false` permite apenas a URL dedicada; presença `true` resulta em erro e zero connector; opt-out Neo4j READ/WRITE e Binance dá skip com zero chamadas a resolver/client/driver; cada opt-in `=1` com config, credencial, identidade, permissão ou conexão inválida falha não-zero sem skip. Fakes provam identidade runtime antes do connector e que tabela vazia/presente/erro não chama pool factory. O gate estrutural prova centralização de env reads sem ampliar allowlist.

Os sete estados do lookup SQL real são escritos/congelados em G3 e executados somente em G4 pelo runner PG efêmero validado. G4 registra RED pré-correção, retorna a G3 para correção/revisão e reexecuta o mesmo teste em G4 para GREEN; falha de setup não conta como RED. Evidência SQL G4 permanece separada de evidência offline G3. Nenhum PostgreSQL, Docker, CI, Neo4j ou exchange foi executado nesta revisão documental.

### Alternativas, riscos, rollout e rollback

Rejeitados: permitir runtime pool para marker table vazia; inferir identidade apenas do marker/URL/nome; converter falha de config/credenciais/permissões/conectividade após opt-in em skip; aceitar `DATABASE_URL` herdada como inofensiva ou fallback; combinar Neo4j READ e WRITE num gate. Isso pode liberar banco sem prova de identidade ou mascarar runner inválido como teste ignorado.

O rollout futuro continua condicionado a G1 independente deste SDD, G3 TDD/revisão e G4 SQL no runner seguro; a dependência W0-06 permanece como já documentada. Rollback futuro reverte snapshot, validação de presence, gates, resolver e fixtures como unidade; não toca DB persistente, não afrouxa fail-closed e não adiciona exceção ao gate ambiental. Até G1, nenhuma implementação ou efeito externo é autorizado.

### Estado atual e precedência

Esta correção é a redação vigente de T-W0-02b para os três findings atuais. O artefato continua `draft/G1 pendente`; nenhum veredito histórico, inclusive o ciclo 6 no hash citado, é tratado como aprovação do presente escopo. As propostas antigas permanecem como histórico de desenho, mas cedem a esta seção em qualquer conflito. Próximo passo: readback e revisão independente G1; não executar código, testes, DB, Docker, CI ou exchange.
