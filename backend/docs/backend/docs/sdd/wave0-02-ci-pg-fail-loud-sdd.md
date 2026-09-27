

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