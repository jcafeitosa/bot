

## G1 revisão C0/C1/C4

C4 candidata local `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`; `docker inspect` confirmou digest/image ID e SQL no container retornou PG 18.6 (`180006`), TimescaleDB 2.30.1 e vector 0.8.6. Isso não prova GitHub service startup. CI precisa inicializar esse digest, health `pg_isready -U postgres -d trading_bot`, então consultar `SHOW server_version_num` e `pg_extension` para as duas extensões; exigir versão >= 180000, ambas instaladas, guardar link e output sem segredos. C4 segue pendente até execução CI real.

C0 checker proposto como binário Rust independente `backend/scripts/pg-manifest-checker/` com `syn::parse_file` + `syn::visit::Visit`, dependências/features fixadas no lockfile. Percorre fontes Rust rastreadas em `backend/src`, módulos inline, ItemFn/blocos/ExprCall; extrai nome qualificado, arquivo/linha/coluna de chamadas diretas ao helper e exige igualdade com PG_TESTS. Reconhece apenas `#[test]`, `#[tokio::test]`; outras formas requerem fixture. Fail closed para wrapper, alias, chamada indireta, `include!`, `#[path]`, macros com token do helper e código gerado; não expande proc-macro nem infere fluxo. Fixtures cobrindo async em módulo inline, função sem atributo, wrapper, ausente/fantasma, macro/include e ocorrência inerte em comentário/string.

C1: linha do helper só prova conexão/migração. Cada teste manifesta `pg_integration_assertions_complete!("<nome_qualificado>")` como última statement após assertions PG; emite `PG_INTEGRATION_ASSERTIONS_OK:<nome>`. Checker exige exatamente uma chamada ao helper e marcador final único; retorno antecipado deixa marcador ausente. Runner exige por execução `--exact`: sucesso, 1 passed, zero falhas/ignorados, um marcador helper e um completion com o mesmo nome. Fixture return após helper antes do marcador falha; assertion seguida de marcador passa. Critic deve confirmar desenho. Estado continua draft; sem autorização até G1.

## Revisão G1 ciclo 3 — contratos revisados C0/C1/C4

> Este adendo complementa e prevalece sobre descrições anteriores de parser e de evidência C1/C4. Mantém status `draft`; não aprova G1 nem autoriza implementação.

### C4 — imagem e evidência reproduzível

Imagem candidata: `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`, o digest fixado em `docker-compose.bot.yml`. Nesta revisão, `docker inspect` do container local `bot-agents-postgres` confirmou digest e image ID iguais; consulta SQL ao banco retornou `server_version_num=180006`, `timescaledb=2.30.1` e `vector=0.8.6`. Essa evidência comprova o container local, não a inicialização pelo GitHub Actions.

Antes da aprovação G1, CI deve iniciar o digest exato como service container com `POSTGRES_USER=postgres`, `POSTGRES_PASSWORD=postgres`, `POSTGRES_DB=trading_bot`, porta 5432 e health `pg_isready -U postgres -d trading_bot` (intervalo 5s, timeout 5s, 10 tentativas). Após healthy, registrar saída/exit code sem segredo e link da execução para `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc 'SHOW server_version_num'` e `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc "SELECT extname || '=' || extversion FROM pg_extension WHERE extname IN ('timescaledb','vector') ORDER BY extname"`. Gate: versão ≥ 180000 e as duas extensões instaladas. Se disponíveis, mas não instaladas, executar `CREATE EXTENSION IF NOT EXISTS timescaledb; CREATE EXTENSION IF NOT EXISTS vector;` e consultar novamente; falha é bloqueante. CI real ainda não foi executado, então C4 continua pendente até evidência do runner GitHub.

### C0 — checker Rust AST

Criar binário checker independente em `backend/scripts/pg-manifest-checker/`, com versões/features de `syn` fixadas no lockfile, usando `syn::parse_file` e `syn::visit::Visit`. Percorrer arquivos `.rs` rastreados sob `backend/src`, módulos inline, `ItemFn`, blocos e `ExprCall`; extrair nome qualificado, caminho relativo e linha/coluna da chamada direta ao helper cujo último segmento do caminho seja `database_for_integration_test`. Comparar o conjunto exato extraído a `PG_TESTS`, reportando divergências e origens. Reconhecer somente `#[test]` e `#[tokio::test]` (formas observadas no repositório), também em `async fn`; novos atributos exigem inclusão explícita e fixture.

Fail closed para wrappers, aliases/imports, indireção por ponteiro/closure, `include!`, módulos externos via `#[path]`, macros contendo token do helper e código gerado; erros devem trazer arquivo/linha. Não expandir proc-macros nem inferir fluxo. Fixtures sem banco: async tokio em módulo inline passa; função sem atributo, wrapper, teste omitido, entrada fantasma e macro/include falham; menção em comentário/string é ignorada. Limite conhecido: AST não prova semântica/expansão de macros ou alcançabilidade runtime; essas formas permanecem proibidas, e C1 cobre execução alcançada.

### C1 — chegada ao helper e término das assertions

`PG_INTEGRATION_HELPER_OK` prova conexão/migração, não a conclusão do teste. Cada teste PG deve terminar com `pg_integration_assertions_complete!("<nome_qualificado>")`, colocado após suas operações e assertions PostgreSQL. A macro emite `PG_INTEGRATION_ASSERTIONS_OK:<nome_qualificado>`. O checker AST exige exatamente uma chamada direta ao helper e exatamente uma chamada de completion como última statement; retorno antecipado antes dela não emite a linha e causa falha do runner.

Para cada invocação isolada `--exact`, o runner exige exit zero, `1 passed`, zero falhas/ignorados, exatamente uma linha `PG_INTEGRATION_HELPER_OK` e uma linha `PG_INTEGRATION_ASSERTIONS_OK` para o mesmo teste. Fixture adversarial com `return` depois do helper e antes do marcador deve falhar por ausência de completion; fixture com assertions seguidas pelo marcador final deve passar. Revisão de código confirma que marcador vem após assertions; a linha comprova o protocolo, não a qualidade semântica das assertions.

### Status do adendo

C0/C1 ficam especificados para revisão técnica independente. C4 ainda precisa de CI real no digest selecionado. Sem re-review independente e evidência GitHub, G1 continua pendente e nenhum trabalho de implementação começa.