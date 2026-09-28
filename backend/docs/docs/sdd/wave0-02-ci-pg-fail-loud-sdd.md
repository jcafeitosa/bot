

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

O snapshot representa, no mínimo, `BOT_PG_INTEGRATION_REQUIRED`, `BOT_RUN_PG_INTEGRATION`, `BOT_PG_TEST_DATABASE_URL` e o caminho do manifest de alvo quando configurável. As flags de execução ficam tipadas como opt-in/required; URL e caminho mantêm parsing/validação centralizada e não aparecem em erros/logs. `DATABASE_URL` não é fallback e não participa da resolução. `DOCKER_HOST` e `DOCKER_CONTEXT`, caso ainda sejam usados pela prova de identidade local, também são lidos no boundary de configuração e transportados como estado tipado suficiente para rejeição; o resolver de persistência não consulta o processo. Nenhum valor default pode ativar PG integration.

Contrato do resolver: sem opt-in, o helper mantém o comportamento local de skip existente. Com required ligado, ausência de opt-in explícito, URL dedicada, manifest ou identidade válidos falha alto e antes de connector/migration. Com os dois opt-ins e configuração válida, prossegue apenas no runner/target já validado pelos critérios C0–C4. A composição de produção não deve permitir caller omitir silenciosamente required/opt-in; qualquer caminho de teste continua passando pelo mesmo helper público de integração e configuração central.

O gate estático existente `verify-backend-gates.sh` é critério executável do desenho: nenhuma ocorrência de `env::var(` ou `std::env::var(` em `src` fora da camada de configuração permitida. Não ampliar a allowlist do checker para `core/persistence`; remover o acesso direto e usar configuração central é a solução. Variáveis de opt-in de outros subsistemas não são ampliadas neste item, salvo as que este helper PG efetivamente consome.

### TDD e critérios observáveis

G1 aprova o seam público de configuração e teste descrito neste SDD; G3 implementa cada fatia com RED/GREEN antes de refatorar. Testes unitários com snapshot tipado usam resoluções puras e fábricas fake, sem ler ambiente, abrir socket ou iniciar serviço.

| Fatia pública | RED exigido antes da implementação | GREEN / evidência exigida |
|---|---|---|
| Snapshot de configuração PG | valores ausente, vazio, formatos de opt-in não aceitos e configuração parcial não produzem resolução válida | parser central produz snapshot tipado, sem segredos em Debug/Display/logs |
| Resolução do helper | sem opt-in mantém skip; required sem opt-in/url/manifest falha antes do connector | configuração válida chega ao lookup apenas após os gates já definidos |
| Lookup real via helper | fixtures SQL para ausência, vazio, válido, inválido, múltiplas linhas, schema incompatível e erro de query falham primeiro contra a classificação pretendida | os sete cenários retornam estado tipado correto; cada fixture é revertida na transação |
| Fail-closed do coordenador | marker presente ou erro SQL não deve chamar factory; teste verifica contador igual a zero | somente ausência inequívoca chama factory uma vez; erro de factory é propagado sem URL |
| Guard de ambiente | `verify-backend-gates.sh` falha quando um acesso `std::env::var` é introduzido em `core/persistence` | gate passa sem exceção específica para persistência; leituras permanecem na fronteira permitida |
| Compatibilidade | opt-in desligado preserva comportamento de skip para testes locais atuais | testes de unidade provam compatibilidade sem PostgreSQL e sem variável global compartilhada |

A prova do lookup SQL é integração e pertence exclusivamente a G4, no runner efêmero seguro aprovado, depois de G1 e da revisão G3 do runner. G3 pode executar unit tests/fakes e checker local; G3 não executa PostgreSQL real. A mera aprovação G1 não autoriza Docker, banco, CI nem exchange. G4 registra o comando e resultado dos sete cenários, identity/manifest do runner, e a saída do gate `verify-backend-gates.sh`. Não alegar execução nesta revisão documental.

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

