---
title: SDD T-DB-ENV — Ambientes dev/prod com PostgreSQL local isolado
description: Proposta para selecionar conexões PostgreSQL distintas por ambiente em execução local, com isolamento de dados e limites claros para produção remota.
tags:
  - sdd
  - backend
  - database
  - configuration
status: proposed
---
# SDD T-DB-ENV — Ambientes dev/prod com PostgreSQL local isolado

**Status: PROPOSED — o contrato atual ainda aguarda aprovação G1 independente.** O owner aprovou um único `--environment` para selecionar exchange e banco; o último Critic G1 apontou três bloqueios antes de qualquer implementação: par completo de credenciais live e allowlist explícita de comandos `prod`; prova de localidade/identidade antes de qualquer SQLx connector; e testes RED/GREEN que provem esses gates e a compatibilidade dev. Este SDD registra as propostas para sanar esses bloqueios. Implementação, testes com conexões reais, Docker/DB e chamadas a exchange permanecem proibidos até um novo G1 aprovado; esta revisão não roda live. O objetivo autorizado é execução local dev e prod com bancos isolados, sem submissão de ordem live.

## Contexto e objetivo

A configuração documentada hoje carrega .env, system.toml e bot.toml, e DATABASE_URL alimenta conexões PostgreSQL em múltiplos caminhos. O owner simplificou a proposta: o --environment existente seleciona coerentemente os dois domínios — dev usa exchange Spot Testnet e BOT_DATABASE_URL_DEV; prod usa exchange live e BOT_DATABASE_URL_PROD. Não adicionar --database-environment. Os callers PostgreSQL runtime devem resolver URL a partir do mesmo Environment efetivo usado pela configuração de trading.

A proposta dá a cada execução um ambiente explícito, uma URL correspondente e um alvo local dedicado. Ambos os ambientes usam PostgreSQL em Docker no host, com instâncias/volumes separados e nomes de database distintos. “prod” nesta proposta significa perfil local com comportamento de produção e dados isolados; não significa acesso a serviço de produção remoto.

## Escopo desta proposta

- **Inclui:** mapear `--environment dev|prod` para Spot Testnet/live e para `BOT_DATABASE_URL_DEV/PROD`; exigir o par live antes de qualquer efeito/conexão `prod`; permitir somente os comandos locais declarados na allowlist; provar identidade/localidade do target antes de SQLx para todos os callers runtime; isolar o runner PG em `BOT_PG_TEST_DATABASE_URL`; especificar testes comportamentais RED/GREEN.
- **Não inclui:** PostgreSQL remoto, deploy, secret manager, validação de credencial chamando Binance, monitor/live exchange access em prod nesta fatia, envio de ordem live, transferir/copiar dados entre ambientes, ou migrar/reutilizar silenciosamente bancos/volumes existentes. Ordens live seguem Disabled/rejeitadas; o teste Spot Testnet autorizado anteriormente é independente e não será executado por esta revisão.
- **Usuário beneficiado e sucesso observável:** operador local escolhe um ambiente único, e o serviço falha sem connector attempt quando credenciais, comando, URL, manifest, daemon ou identidade não provam o destino local correto; comandos permitidos não conseguem cruzar dados nem habilitar ordens.

## Contrato proposto

### Seleção de ambiente e URL

- Opção pública: --environment dev|prod existente. Não adicionar --database-environment.
- Uma única escolha de Environment é compartilhada entre exchange e DB: dev → credenciais/endpoints Spot Testnet + BOT_DATABASE_URL_DEV; prod → configuração de exchange live + BOT_DATABASE_URL_PROD.
- Todos os callers runtime que conectam, leem, escrevem, migram ou apagam PostgreSQL recebem esse mesmo `Environment` e usam o resolver comum; nenhum lê `DATABASE_URL` diretamente nem faz fallback. O resolver é o único seam até SQLx e deve devolver uma prova verificada do target local antes de criar/invocar qualquer connector.
- URL selecionada ausente/vazia/inválida falha fechado antes de conexão/migration/delete. Erros e logs nunca exibem URL ou segredo.
- O Environment resolvido aparece em logs/metadados sem credenciais. O default permanece dev; prod só é selecionado por `--environment prod`, nunca pela presença de chaves. Em qualquer comando prod que vá abrir uma conexão, migrar ou executar outra ação externa, validar antes que `BINANCE_PROD_API_KEY` e `BINANCE_PROD_API_SECRET` estejam ambos presentes, não vazios e não placeholders; ausência ou presença parcial falha antes de qualquer connector/migration/ação. A validação não chama Binance nem prova validade remota; não registra nem ecoa valores.

### Prova de target local antes do connector

A presença de URL em `.env`, `localhost` ou uma porta com resposta **não** prova que o alvo é o PostgreSQL local autorizado: processo env pode substituir `.env`, porta pode ser ocupada/redirecionada e Docker context pode apontar a daemon remota. Portanto, o resolver seleciona `BOT_DATABASE_URL_DEV` ou `BOT_DATABASE_URL_PROD`, valida localidade e identidade sem SQLx, e só então entrega a URL ao connector. Não se permite tentativa de conexão como método de descoberta.

A prova deve, antes de qualquer `PgPoolOptions`/SQLx connect:

1. Rejeitar `DATABASE_URL` como runtime fallback, host não-loopback, hostname remoto, endereço/link-local/private não permitido, URL malformada, porta que não consta no manifest, database ou role diferentes dos declarados, e qualquer override de Docker para daemon desconhecida/remota (`DOCKER_HOST`, `DOCKER_CONTEXT`, SSH/TCP/HTTP).
2. Confirmar que o manifest local de targets existe, parseia sem ambiguidade e contém exatamente uma entrada para o `Environment` selecionado, com project/service, environment-id estável, container ID, image digest/ID, volume ID/mount source, host-port→container-port e nomes esperados de database/role.
3. Confirmar daemon Docker disponível pelo endpoint/socket local esperado e identidade local, inspecionar serviço/container e volume vivos, e comparar rótulos/IDs/image/mount/porta do estado real com o manifest e a URL selecionada. `BOT_DATABASE_URL_DEV` só pode corresponder ao manifest dev; `BOT_DATABASE_URL_PROD` somente ao manifest prod.
4. Produzir um `VerifiedLocalPostgresTarget` imutável para aquele Environment e URL. Os callers runtime aceitam o connector somente com esse valor; URL ou prova ausente/inválida nunca chega a SQLx.

Manifest ausente, corrompido, duplicado, identity mismatch, daemon/serviço/volume ausente, endereço ou porta remotos/divergentes, process env override ambíguo, falha de inspeção e qualquer erro de prova resultam em erro estável, sem URL/segredo, e **zero tentativas de connector**. Prova local apenas valida destino/identidade; não garante correção dos dados nem autoriza ordens.

Variáveis candidatas para acordo:

| Variável | Uso proposto |
|---|---|
| BOT_DATABASE_URL_DEV | URL somente para o PostgreSQL local de desenvolvimento |
| BOT_DATABASE_URL_PROD | URL somente para o PostgreSQL local do perfil prod |
| BINANCE_PROD_API_KEY | Uma metade do par obrigatório do perfil prod antes de qualquer ação/conexão externa |
| BINANCE_PROD_API_SECRET | Outra metade do par obrigatório do perfil prod; não é usada isoladamente nem por dev |
| BOT_PG_TEST_DATABASE_URL | URL exclusiva do runner de integração para banco PostgreSQL descartável; não é fallback runtime |
| BOT_DATABASE_ALLOW_REMOTE | Não proposta; nenhum flag libera destino remoto nesta entrega |

As URLs reais permanecem em `backend/.env`, com permissões restritas e fora do Git. Exemplos documentam apenas nomes/forma redigida, sem credenciais ou valores copiáveis. `BINANCE_PROD_API_KEY` e `BINANCE_PROD_API_SECRET` são um par indivisível; runtime não usa uma metade nem cai para chaves dev/testnet. Segredos locais não são copiados para snapshot de testes ou logs.

### Comandos locais suportados em `prod`

`Config::validate` hoje rejeita `Environment::Prod`. G3 não deve remover essa barreira globalmente. Em vez disso, uma validação por comando deve permitir somente os casos locais abaixo e continuar rejeitando qualquer comando `prod` fora da lista:

| Comando/perfil local | Decisão proposta para G3 | Pré-condições / limites |
|---|---|---|
| `backtest --environment prod` sem `--persist` | Permitido; fixture/simulação offline | Não abre DB, não consulta exchange, não habilita execução de ordem; par live pode estar ausente porque não há ação externa/conexão |
| `backtest --environment prod --persist` | Permitido somente com par completo live e destino PG local provado | Credenciais verificadas antes de SQLx/migration; persiste apenas no DB associado ao manifest prod |
| `serve --environment prod` | Permitido somente como API local em loopback, sem `--with-monitor` e com execução de ordens `Disabled` | Par live completo validado antes de qualquer DB connector/migration e antes do listener; PG local prod provado; serve não inicializa exchange |
| `serve --environment prod --with-monitor`, monitor/TUI live, `orders retention-purge`, `graph-projection drain` e todo outro comando ausente da lista permitida | Rejeitado por padrão nesta entrega | Rejeição antes de connector, migration, listener ou chamada externa; exigir design/decisão separada para ampliar |
| qualquer submissão de ordem live | Rejeitada e fail-closed | Chaves live presentes não habilitam ordem. Sem teste live nesta entrega; somente autorização explícita e SDD/revisão próprios poderiam alterar esse gate |

A allowlist de comandos é uma decisão de contrato a ser aprovada no próximo G1. Ela permite smoke local do serviço e backtest determinístico, sem ligar o monitor à Binance live nem confundir `prod` local com autorização de trading. Um comando permitido deve rejeitar bind não-loopback, persistência sem prova de target e qualquer configuração de execução diferente de `Disabled` antes de efeitos externos.

### PostgreSQL e isolamento

Compose provisiona dois stacks persistentes, um por ambiente, com container, volume, database e role próprios. Cada ambiente recebe uma identidade estável (environment-id criado uma vez e guardado no manifest), nomes únicos e porta host fixa distinta; reiniciar não cria novo UUID nem deixa a URL local apontando para volume/container antigo. BOT_DATABASE_URL_DEV/PROD ficam estáveis no .env local e o manifest associa deterministicamente cada URL a project, environment-id, container ID, porta, image digest/ID, volume ID e mount source. URL ausente ou divergente do manifest falha fechado.

O banco prod local começa vazio e isolado. Não se restaura dump de produção nem se copia conteúdo de dev por padrão. Dados de mercado públicos e fixtures sintéticas podem ser carregados por ferramenta explicitamente identificada; dados de usuário, tokens, credenciais de exchange e provider não entram no seed. O banco dev pode ser recriado sem afetar o volume prod.

Migration executa apenas após validar ambiente, database, role e identidade Docker. A mesma cadeia versionada é aplicada em dev e prod local, sem schema divergente. Antes da primeira migration, provar alvo vazio. Para alvo não vazio, exigir backup consistente prévio associado ao database/container/volume e evidência de restore verificável; se isso não puder ser demonstrado, recusar migration e criar alvo vazio novo. Migration falha interrompe o comando, preserva log redigido e não altera o outro ambiente.

Os documentos [core database](./core-database-sdd.md), [integração de módulos](./database-module-integration-sdd.md) e [configuração centralizada](./centralized-config-sdd.md) descrevem o seam PostgreSQL e as camadas de configuração existentes. Este SDD propõe seleção por ambiente sem declarar que esses contratos foram alterados ou aprovados.

### Compose local

Compose cria um projeto persistente exclusivo por ambiente, com nome derivado de bot-db-env-<env>-<environment-id>; o ID é estável entre start/stop/restart/migrate/backup e muda somente quando um novo ambiente vazio é criado após teardown explícito. Services e volumes também carregam environment-id. Não usar nome default por diretório nem reutilizar recursos do docker-compose.bot.yml.

Antes de qualquer operação mutável, runner comprova o Docker daemon/context: rejeita DOCKER_HOST, DOCKER_CONTEXT ou outras variáveis/flags que redirecionem endpoint; exige o context local explicitamente configurado; inspeciona endpoint e aceita apenas socket Unix local (ou transporte local equivalente comprovado pelo host). SSH, TCP/HTTP, endpoint remoto e context desconhecido/redefinido falham antes de start/stop/backup/migrate/cleanup. Executar sempre com ambiente Docker sanitizado e context explícito, conferir endpoint/daemon identity novamente antes da mutação. Se a prova local não estiver disponível, não operar Docker.

Em seguida validar labels de projeto/ambiente/environment-id, container ID, image digest/ID, volume ID, mount source e porta contra manifest e URL selecionados. A URL estável BOT_DATABASE_URL_DEV/PROD deve resolver para exatamente o container/porta do mesmo manifest; divergência, porta ocupada, IDs trocados ou container ausente abortam sem mutação e nunca criam substituto silenciosamente que deixe .env stale.

Cada ação nomeia explicitamente context local + project + service/container + volume validados. Lifecycle: up cria stack/volume só se manifest e destino estiverem ausentes; start/stop/restart/migrate/backup operam somente IDs do manifest e mantêm URL/porta estáveis; backup grava artifact identificado por environment-id/database/container/volume e valida conclusão; cleanup normal só para o serviço, preservando volume. Destruir volume exige comando explicitamente destrutivo com environment-id confirmado, backup consistente/restaurável ou prova de vazio, revalidação de IDs/mount e confirmação; remove apenas recurso exato. Proibidos down sem identidade validada, down -v, docker system prune, volume prune, rm amplo ou nomes parciais. Dev nunca opera recursos prod. Inventariar stack atual antes de criar recursos; não reatribuir volumes nem migrar o stack existente. Bind somente loopback/rede privada e credenciais não ficam no Compose.

## Matriz de callers runtime e ferramentas

A regra proposta é abrangente: todo caller runtime que conecta, lê, escreve, migra ou apaga dados PostgreSQL recebe o mesmo --environment efetivo e usa o resolver. Nenhum caller runtime aceita DATABASE_URL após a migração. Se um caminho não puder receber o seletor, falha fechado antes de resolver URL ou conectar.

| Caller / ferramenta | Classificação e operação PostgreSQL | Caminho de seleção proposto |
|---|---|---|
| serve / AppDatabases::bootstrap_runtime (bootstrap_http_api) | runtime transitivo: bootstrap/connect/migrations; stores de agents, bots, owner e provider credentials leem/escrevem via AppDatabases | propaga o --environment compartilhado ao resolver comum; stores não leem env por conta própria |
| monitor / bootstrap_monitor_postgres | runtime: conexão e migrations do monitor | recebe o Environment compartilhado; URL selecionada ausente → zero connect/migrate |
| optional_postgres_for_monitor_supervisor_snapshot() | runtime transitivo: consulta snapshot/supervisor usando PostgreSQL opcional | recebe contexto do bootstrap selecionado; não chama postgres_url_from_env nem aceita DATABASE_URL; sem resolver, permanece sem DB/falha fechado |
| backtest --persist / postgres_for_cli_persist | runtime: leitura/escrita de persistência de backtest | herda o --environment compartilhado; persist=false não abre conexão |
| orders CLI retention-purge | runtime CLI: abre/migra e apaga registros via postgres_url_from_env | substituir postgres_url_from_env pelo resolver do Environment compartilhado; sem fallback DATABASE_URL |
| graph-projection drain | runtime CLI: lê/escreve outbox e valida/migra PG; Neo4j é conexão separada | recebe o Environment compartilhado no PG; Neo4j mantém seu próprio config/gate |
| run-pg-integration-tests.sh | runner de testes de integração, não caller runtime; executa testes PG individuais | usa runner dedicado, URL BOT_PG_TEST_DATABASE_URL, gate/identity/marker; rejeita DATABASE_URL fallback antes de iniciar testes |
| pg-v18-monitor-persistence-audit.sh | ferramenta de auditoria PG, não runtime do produto; acesso a banco só no alvo explicitamente validado | exige target/manifest isolado e URL de auditoria explícita; não faz fallback para DATABASE_URL nem escolhe dev/prod silenciosamente |
| verify-backend-full.sh | wrapper de verificação; não conecta diretamente, pode delegar ao runner PG | encaminha apenas para run-pg-integration-tests.sh com target/gate explícitos; no modo default não resolve DATABASE_URL |

Esta matriz consolida [integração dos módulos](./database-module-integration-sdd.md), [core database](./core-database-sdd.md), [configuração centralizada](./centralized-config-sdd.md) e [referência de CLI](../reference/cli-and-config.md), mais os callers/orders e scripts identificados no review. Antes do aceite de implementação, busca de código e inspeção de call graph devem confirmar a lista completa de acessos diretos/transitivos. Critério: nenhum acesso runtime residual a DATABASE_URL ou postgres_url_from_env; helpers transitivos recebem o Environment compartilhado; scripts de teste/auditoria ficam em contratos explícitos separados e não são apresentados como comandos runtime.

## Acordo owner e pendências de seam

## Novo direcionamento do owner e estado dos gates

Direcionamento aprovado pelo owner: um único `--environment` seleciona exchange e banco; `dev` usa Spot Testnet + `BOT_DATABASE_URL_DEV`, `prod` seleciona conta live + `BOT_DATABASE_URL_PROD`; integração PG usa somente `BOT_PG_TEST_DATABASE_URL` isolada; não existe `--database-environment`. Esta decisão substitui o seletor separado anterior e continua sendo o seam público.

O G1 APROVADO COM FOLLOW-UP registrado abaixo cobre somente o desenho anterior e não aprova esta revisão. O último Critic G1 manteve implementação bloqueada por três pontos: (1) par completo `BINANCE_PROD_API_KEY`/`BINANCE_PROD_API_SECRET` antes de qualquer conexão/migration/ação externa e definição inequívoca da allowlist de comandos prod apesar de `Config::validate` rejeitar `Environment::Prod`; (2) prova local de target e identidade antes do connector para todos os callers runtime, com zero tentativas se URL/host/porta/manifest/identity/daemon falharem; (3) TDD RED/GREEN dessas barreiras, da allowlist e da compatibilidade dev. Esta revisão incorpora propostas para os três; a aprovação de escopo do owner não substitui o novo G1 técnico.

Os contratos de no-fallback para URL, isolamento local, backtest persist e integração descartável continuam aplicáveis. Testes PG usam exclusivamente `BOT_PG_TEST_DATABASE_URL`, nunca URL runtime, sem submission de ordens. Até novo G1 aprovado: nenhuma implementação, chamada a DB/Docker/exchange ou smoke operacional. Nenhuma ordem live é permitida nesta entrega.

## Produção remota e segurança

Não existe endpoint ou secret manager de produção no escopo conhecido. Portanto, BOT_DATABASE_URL_PROD significa exclusivamente a instância local prod. Este SDD não autoriza e não especifica conexão remota, deployment ou migração de dados reais. Quando existir infraestrutura de produção, uma proposta separada deverá definir identidade de workload, secret manager, TLS/CA, allowlist de rede, backup/PITR, privilégio mínimo, auditoria, migração sem downtime, readiness, restore drill e aprovação de lançamento.

Em produção remota futura, não reutilizar .env local ou credenciais Docker locais. Um modo remoto deve ser opt-in separado, com configuração de secret manager e validação de destino; não adicionar BOT_DATABASE_ALLOW_REMOTE como bypass genérico.

## Alternativas consideradas

- **`--environment` compartilhado (escolhido pelo owner):** mantém exchange e database coerentes; `dev` seleciona Testnet + URL_DEV e `prod` live + URL_PROD. Um seletor de database separado foi rejeitado pelo owner porque permite perfis cruzados.
- **`DATABASE_URL` único com troca manual (rejeitado):** operador/processo pode migrar o banco errado e `DATABASE_URL` herdado pelo process env pode sobrepor `.env`; nenhum fallback será mantido.
- **Confiar somente em `.env`, `localhost` ou resposta da porta (rejeitado):** isso não prova qual container/volume/daemon atende ao socket. A prova de manifest + daemon/container/volume/port identity ocorre antes do connector.
- **Conectar primeiro e inspecionar depois (rejeitado):** a tentativa já pode alcançar host remoto/serviço errado antes da validação; viola o requisito de zero connector attempts nos casos negativos.
- **Remover globalmente a rejeição de `Environment::Prod` (rejeitado):** habilitaria monitor/exchange live e comandos não avaliados. A alternativa proposta é allowlist por comando, restrita a backtest offline e `serve` local sem monitor, e rejeitar o restante.
- **Flag genérica para host remoto (rejeitada):** um `BOT_DATABASE_ALLOW_REMOTE` converteria erro de prova em bypass; configuração remota requer proposta/autorização separadas.

URLs permanecem em env local; TOMLs não recebem segredos.

## Matriz TDD e validação proposta para novo G1 (nenhuma execução nesta revisão)

| Seam | Critério RED/GREEN para G3 após aprovação G1 | Observabilidade/efeito esperado |
|---|---|---|
| --environment compartilhado | dev e prod mapeiam em pares coerentes exchange+DB; nenhuma opção de DB separada | teste puro prova mapping dev→testnet+URL_DEV e prod→live+URL_PROD; trading e DB nunca divergem |
| Resolver e env vars | Environment seleciona só `BOT_DATABASE_URL_DEV/PROD`; nenhum `DATABASE_URL` runtime/fallback | URL ausente/vazia falha antes de connect/migrate; nenhum segredo em erro/log |
| Credencial prod ausente/parcial | RED: connector fake conta tentativa em `serve prod`/`backtest --persist` com ambos ausentes ou só uma chave | GREEN: falha antes do verifier/connector/migration/listener/ação externa; contador de connector = 0; mensagem não inclui valores |
| Prova local antes do connector | RED para hostname/IP remoto, port mismatch, DB/role divergente, manifest ausente/corrompido/duplicado, IDs/volume/container/image divergentes, daemon ausente/remota/override | GREEN: cada caso negativo produz 0 tentativas SQLx, nenhuma migration/query/ação; somente URL+manifest+daemon+container identity verificados produzem `VerifiedLocalPostgresTarget` |
| Comandos prod permitidos/rejeitados | RED: `Config::validate` hoje rejeita `Environment::Prod`; testes de dispatch exercitam allowlist por comando | GREEN: backtest offline permitido sem efeitos; backtest persist e serve loopback sem monitor só com par live+target provado; `serve --with-monitor`, monitor e comandos prod não listados rejeitados antes de efeitos; ordens live sempre rejeitadas |
| Dev compatibility | RED/GREEN com apenas configuração/URL dev e sem chaves live | dev escolhe Spot Testnet + `BOT_DATABASE_URL_DEV`, os mesmos gates locais se aplicam antes de SQLx; ausência de chaves prod não altera dev; nenhuma URL/test credential prod é consultada |
| Proteção live orders | RED com `BOT_ORDERS_EXECUTION=live_exchange` mesmo com par de chave presente | prod continua `Disabled`; qualquer ordem live é rejeitada sem submit; somente autorização explícita e SDD/teste live separados poderiam alterar esse gate |
| serve + stores transitivos | bootstrap, owner, agents, bots, provider credentials recebem o Environment compartilhado e prova local verificada | sem seleção/URL/manifest/daemon/identity válidos, zero connector attempts, read/write/migrate |
| monitor + optional_postgres_for_monitor_supervisor_snapshot() | contexto selecionado propaga ao caminho auxiliar; flag ausente falha fechado | zero resolução por `postgres_url_from_env`; qualquer falha de prova local causa zero connector attempts/query |
| backtest e orders retention-purge | persist=true e purge recebem seletor e target provado; persist=false não abre PG | sem seletor/prova zero connector/migrate/delete; purge prod rejeitado pela allowlist desta entrega |
| graph-projection drain | PG outbox usa destino selecionado; Neo4j é independente | zero PG access sem seletor |
| run-pg-integration-tests.sh | integrações só rodam com runner/target/marker explícitos | default não lê DATABASE_URL nem conecta ao runtime DB |
| pg-v18-monitor-persistence-audit.sh | ferramenta de auditoria requer target manifestado, não é CLI runtime | recusa target ausente/ambíguo antes de query; sem fallback DATABASE_URL |
| verify-backend-full.sh | verificação composta delega integração somente ao runner separado | modo default não resolve URL nem cria conexão |
| Docker context local | endpoint Unix local e daemon identity validados permitem operação; `DOCKER_HOST`/`DOCKER_CONTEXT`, SSH/TCP/HTTP remoto ou context alterado falham antes do connector | zero connector attempts e nenhuma operação mutável quando a prova não passa |
| Compose resource identity | labels/project/environment-id/container ID/volume ID/mount source/image/port/database/role corretos permitem conexão somente se iguais ao manifest selecionado | mismatch aborta antes do SQLx; zero connector attempts; dev não alcança prod |
| Lifecycle e URL estável | up cria uma vez; stop/start/migrate/backup preservam environment-id, porta e URL; cleanup preserva volume; destroy explícito remove apenas target validado | manifest mapeia deterministicamente URL→project/container/port/image/volume/mount; não fica URL stale nem cria volume substituto silencioso |
| Schema/migration | alvo vazio migra; alvo ocupado requer backup consistente/restaurável | versão/checksum por alvo; database irmão inalterado |

Testes unitários serão offline, usarão sentinelas, verifier/manifest fixtures e connector falso com contador, sem carregar `.env` real. Cada linha abaixo exige RED antes da implementação e GREEN após a mudança mínima. Integrações de runner/auditoria usam apenas target/manifest descartável por `BOT_PG_TEST_DATABASE_URL`; nunca herdam `DATABASE_URL` ou URL runtime. Esta matriz define evidência futura; não há execução de teste neste trabalho.

## Rollout local e rollback

1. Fechar os três blockers e obter G1 independente aprovado; até então não há implementação, testes de integração ou operação local de DB/Docker/exchange.
2. Após G1, mapear todos os callers transitivos e provar que cada caminho SQLx recebe `Environment` e `VerifiedLocalPostgresTarget`; escrever testes RED/GREEN offline com connector e verifier fakes.
3. Implementar a allowlist de comandos sem tornar `Config::validate(Environment::Prod)` permissivo globalmente. Confirmar que comandos não listados são rejeitados antes de resolver URL, connector, migration, listener ou exchange.
4. Para validação comportamental posterior autorizada, começar com `dev` e instância PG descartável identificada pelo manifest; não usar `trading_bot` local existente nem volumes preexistentes como banco descartável. Nenhuma chamada à exchange é necessária para provar estes seams.
5. Somente com autorização separada para operações Docker/DB, criar containers/volumes dedicados vazios por ambiente, verificar digest/extension/identity, depois executar migrations com backup/restauração e comparar versão/checksum. Nunca migrar banco populado sem backup verificável.
6. Antes de qualquer ação de perfil `prod` que abra connector, migre ou aja externamente, validar as duas chaves live juntas e provar o target prod. `serve prod` somente em loopback, sem monitor e com execução `Disabled`; nenhuma ordem live é executada nem incluída nesta validação.
7. Testes PG usam exclusivamente `BOT_PG_TEST_DATABASE_URL` para instância descartável. O runner não herda URL runtime nem chama exchange. Inspecionar logs para confirmar ausência de secrets e URLs.

**Rollback:** credencial parcial, target não provado, migration inesperada ou comando fora da allowlist deve abortar antes do connector/efeito externo. Se G3 introduzir regressão, reverter em conjunto o resolver e todos os adapters/comandos; não reabilitar fallback `DATABASE_URL` nem remover o preflight. Se migration já ocorreu, atuar apenas no environment-id/container/volume daquele ambiente e restaurar backup verificado correspondente; nunca apontar prod para dev. Nenhuma ação remota/live está prevista.

## Riscos e pendências

- A presença das chaves live não comprova que estejam válidas ou restritas a leitura; esta proposta nunca testa a validade com Binance e não dá ordem de trading. Uma chave com privilégios amplos continua segredo de alto impacto.
- Uma manifest/daemon proof incompleta pode aprovar container errado. Qualquer campo ausente/divergente deve falhar sem tentativa SQLx; toda rota/caller runtime precisa usar o mesmo seam, sem helper paralelo.
- Reuso de porta, process-env override, manifest stale/corrompido, daemon/context remoto ou disponibilidade de Docker ausente podem impedir operações locais legítimas; o custo aceito é fail-closed sem fallback.
- `Config::validate` atualmente rejeita `Environment::Prod`; comando allowlist precisa ser explícita e testável para que liberar `serve`/`backtest` não libere monitor, troca live ou ordem por efeito colateral.
- “prod local” não simula IAM, TLS, rede, backups/PITR nem operação remota real. Docker/image/migrations/extensões exigem validação futura somente em stack descartável identificado; banco existente não é evidência.
- Resolução local do target não valida conteúdo/durabilidade nem substitui backup/restauração; migration parcialmente aplicada exige estratégia específica por ambiente.

## G1 anterior — ciclo final do contrato supersedido

**Veredito histórico:** APROVADO COM FOLLOW-UP pelo Critic independente `/root/test_safety_sdd_critic` para a revisão canônica OpenKnowledge `2640bafc0cc822d64be8734f5a2b880c7b5191b9`. Esse ciclo analisou --database-environment separado. O owner posteriormente simplificou o seam para o --environment único; portanto o veredito anterior não cobre nem aprova a mudança. O follow-up editorial sobre environment-id foi incorporado. Novo G1 independente solicitado para o mapping --environment → exchange + DB.

## Decisão solicitada e estado

A proposta atual usa somente --environment dev|prod para selecionar exchange e DB coerentemente: dev → Testnet + BOT_DATABASE_URL_DEV; prod → live + BOT_DATABASE_URL_PROD. Todos os callers runtime PostgreSQL passam pelo resolver que recebe o Environment compartilhado; runner de integração usa somente BOT_PG_TEST_DATABASE_URL descartável e não envia ordens. O veredito G1 anterior foi supersedido pela mudança de seam; novo G1 independente está pendente. Compose local continua restrito a endpoint/socket/daemon verificado e environment-id persistente. Destino remoto não é configurado. Status permanece proposed; não implementar até novo G1.