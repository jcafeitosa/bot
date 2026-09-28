

## Revisão G1 ciclo 3 — resolução proposta para C0, C1 e C4

> Este adendo prevalece sobre descrições anteriores de parser e evidência de imagem neste SDD. Mantém o documento em `draft`; não aprova G1 nem autoriza implementação. Foi preparado para revisão técnica independente.

### C4 — imagem fixada e validação reproduzível

A imagem candidata para CI é o digest já usado no serviço local do projeto: `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`. Evidência observada no ambiente local nesta revisão: `docker inspect` do container `bot-agents-postgres` retornou esse mesmo identificador de imagem e image ID; consulta SQL no banco `trading_bot` retornou `server_version_num=180006`, `timescaledb=2.30.1` e `vector=0.8.6`. Fonte local: `docker-compose.bot.yml` (serviço `agents-postgres`, digest/healthcheck e configuração `PGDATA`) e o container iniciado a partir dele. Isso confirma imagem e extensões no container local; não demonstra por si só inicialização sob a implementação de service container do GitHub Actions.

Antes da aprovação G1, reproduzir em runner GitHub com o digest exato, `POSTGRES_USER=postgres`, `POSTGRES_PASSWORD=postgres`, `POSTGRES_DB=trading_bot`, porta 5432 e health check `pg_isready -U postgres -d trading_bot` (intervalo 5s, timeout 5s, 10 tentativas). Depois de healthy, executar e guardar saída sem segredo destes comandos contra `DATABASE_URL`:

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc 'SHOW server_version_num'
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc "SELECT extname || '=' || extversion FROM pg_extension WHERE extname IN ('timescaledb','vector') ORDER BY extname"
```

O gate de evidência exige versão numérica ≥ `180000`, exatamente as duas extensões instaladas na resposta, exit code zero, health check healthy, e log identificando o digest executado. Se a extensão estiver disponível mas não instalada, o workflow deve instalá-la primeiro com `CREATE EXTENSION IF NOT EXISTS timescaledb; CREATE EXTENSION IF NOT EXISTS vector;` e repetir a consulta; falha de criação é bloqueante. Registrar o link da execução CI que mostra inicialização e consultas. O CI real ainda não foi executado nesta revisão; esse resultado continua requisito antes de fechar G1/aceite. Proveniência/limite: valores SQL são observação local do container atual; não inferir sucesso de GitHub Actions a partir deles.

### C0 — contrato do checker Rust AST

Substituir a busca textual por um binário dedicado em `backend/scripts/pg-manifest-checker/`, compilado com versões e features de `syn` fixadas no lockfile desse checker. O binário analisa `.rs` rastreados sob `backend/src` usando `syn::parse_file` e `syn::visit::Visit`, percorre módulos inline, itens `ItemFn`, blocos e `ExprCall`, e emite nome qualificado do teste + caminho relativo + linha/coluna para chamada direta a `database_for_integration_test`. Identifica como teste apenas `#[test]` e `#[tokio::test]`, incluindo funções async; qualquer novo atributo exige extensão explícita e fixture. O script compara igualdade dos nomes qualificados extraídos ao `PG_TESTS`, reportando diferenças com origem.

Suporte é intencionalmente restrito a chamadas diretas em funções de teste. Wrappers, aliases/imports para o helper, chamadas por ponteiro/closure, `include!`, módulos externos via `#[path]`, macro invocations com tokens do helper, geração de código e expansão de proc-macro não são suportados. Quando a forma aparece, o checker falha com arquivo/linha; não tenta inferir expansão ou semântica. Essa limitação torna o resultado fail-closed para as construções relevantes, em vez de alegar cobertura de Rust arbitrário. Fixtures sem banco cobrem: chamada direta em `#[tokio::test] async fn` aninhada em módulo inline; chamada em função sem atributo; wrapper; manifesto omitido; entrada fantasma; helper mencionado somente em comentário/string; uso em macro/include (rejeição). C0 mantém as provas de divergência de manifesto já listadas acima.

### C1 — provar chegada ao helper e término das assertions

A linha `PG_INTEGRATION_HELPER_OK` comprova somente que o helper obteve conexão/migração válida; ela não prova que o corpo do teste chegou ao fim. Cada teste do manifesto deve, portanto, encerrar explicitamente com `pg_integration_assertions_complete!("<nome_qualificado>")` depois de todas as operações e assertions PostgreSQL. A macro emite uma linha única e estável `PG_INTEGRATION_ASSERTIONS_OK:<nome_qualificado>`; o script exige exatamente uma linha correspondente no stdout daquela invocação individual, além de exit zero, `1 passed`, zero falhas/ignorados e exatamente uma linha `PG_INTEGRATION_HELPER_OK`. Toda saída é capturada por invocação separada com `--exact`.

O checker AST verifica também que cada função PG tenha exatamente uma chamada direta ao helper e exatamente uma chamada final ao marcador de conclusão como última statement do corpo; retorna antecipados antes dessa statement não emitem o marcador, portanto a execução falha no runner. Retorno antecipado depois de completar todo o trabalho de DB pode ser aceito somente se a chamada final ficar após as assertions que justificam sucesso. Fixture adversarial inclui retorno antecipado após helper e antes do marcador: o runner deve falhar por ausência do marcador; outro caso com assertion + marcador final passa. Review de código confirma que o marcador é colocado depois das assertions de cada teste; a linha, sozinha, é evidência de protocolo, não prova formal da qualidade semântica das assertions.

### Estado e gate

Este adendo resolve as lacunas de especificação descritas nos achados do Critic para C0/C1 e adiciona evidência local rastreável para C4. Permanecem pendentes o teste empírico do service container/digest no GitHub Actions e revisão independente deste adendo. Até ambas as condições serem registradas, status permanece `draft`, G1 não aprovado e não se inicia TDD/implementação.

## G1 ciclo 3 — contratos revisados C0/C1/C4

**Estado:** addendum de revisão, não autoriza implementação nem aprova G1. O adendo prevalece sobre descrições anteriores de parser/prova C1 e gate de imagem.

**C4 — imagem e evidência.** Candidata: `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`, igual ao digest do serviço local em `docker-compose.bot.yml`. Evidência observada nesta revisão no container `bot-agents-postgres`: `docker inspect` confirmou digest e image ID iguais; SQL no `trading_bot` retornou `server_version_num=180006`, `timescaledb=2.30.1`, `vector=0.8.6`. Isso é evidência local, não prova de execução em service container GitHub Actions. O CI deve iniciar exatamente o digest com `POSTGRES_USER=postgres`, `POSTGRES_PASSWORD=postgres`, `POSTGRES_DB=trading_bot`, health `pg_isready -U postgres -d trading_bot` (intervalo 5s, timeout 5s, 10 tentativas). Após healthy, executar e guardar exit code/output sem segredo: `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc 'SHOW server_version_num'` e `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc "SELECT extname || '=' || extversion FROM pg_extension WHERE extname IN ('timescaledb','vector') ORDER BY extname"`. Aceite requer `server_version_num >= 180000` e ambas as extensões em `pg_extension`; se disponíveis não instaladas, executar `CREATE EXTENSION IF NOT EXISTS timescaledb; CREATE EXTENSION IF NOT EXISTS vector;` e consultar de novo. Registrar URL/link da execução CI que comprova health, digest e consultas. CI real ainda não foi executado: gate permanece aberto.

**C0 — checker AST e escopo.** Criar binário checker em `backend/scripts/pg-manifest-checker/` com `syn::parse_file` + `syn::visit::Visit`, versões/features de syn fixas no lockfile. Percorrer `.rs` rastreados sob `backend/src`, módulos inline, `ItemFn`, blocos e `ExprCall`; registrar nome qualificado + arquivo relativo + linha/coluna da chamada direta cujo último segmento é `database_for_integration_test`; igualdade exata com `PG_TESTS`. Reconhecer somente `#[test]` e `#[tokio::test]` (formas hoje observadas), incluindo async; outros atributos exigem suporte/fixture explícitos. Rejeitar com origem macros contendo token do helper, `include!`, módulos externos `#[path]`, alias/import, chamada indireta, wrapper ou geração de código; não expandir proc macros nem inferir fluxo. Fixtures sem banco: async tokio em módulo inline passa; função sem atributo, wrapper, manifesto omitido/fantasma e macro/include falham; comentário/string inerte é ignorado. Limite: AST não prova semântica/expansão de macro nem alcançabilidade runtime; isso é C1.

**C1 — chegada e término.** `PG_INTEGRATION_HELPER_OK` prova só conexão/migração. Cada função PG deve encerrar com `pg_integration_assertions_complete!("<nome_qualificado>")`, após todas as operações/assertions PG. Emite exatamente `PG_INTEGRATION_ASSERTIONS_OK:<nome_qualificado>`. Checker AST exige uma chamada direta ao helper e exatamente um marcador como última statement da função; retorno antes do marcador deixa ausência de linha e runner falha. Runner captura saída por invocação `--exact` e exige sucesso, `1 passed`, zero failed/ignored, exatamente uma linha de helper e exatamente uma linha de completion para o mesmo nome. Fixture adversarial: return após helper antes do marcador falha; assertions + marcador final passa. Revisão de código verifica marcador depois das assertions; linha de protocolo não substitui julgamento semântico da assertion.

Pendente: revisão independente G1 deste adendo e execução CI real do digest exato. Sem ambas, status draft.

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

### Alternativas, risco e validação

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
