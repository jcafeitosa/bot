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

**Status: PROPOSED — G1 independente APROVADO COM FOLLOW-UP no ciclo final; implementação é autorizada sob os contratos descritos e o follow-up editorial está registrado abaixo.** O owner aprovou o seletor separado e todos os callers no resolver. O SDD continua proposed enquanto implementação/aceite G3 não forem concluídos. O objetivo autorizado é ter execução local dev e prod usando bancos locais distintos. Não há endpoint nem secret manager real de produção fornecido; este SDD não cria nem presume um.

## Contexto e objetivo

A configuração documentada hoje carrega .env, system.toml e bot.toml, e DATABASE_URL alimenta conexões PostgreSQL em múltiplos caminhos. A CLI já usa --environment dev|prod para o perfil de trading/Binance. A proposta de banco adiciona o seletor ortogonal --database-environment dev|prod; não muda a semântica do flag existente. A seleção ambígua pode direcionar comandos de produção para dados de desenvolvimento ou para o banco persistente existente.

A proposta dá a cada execução um ambiente explícito, uma URL correspondente e um alvo local dedicado. Ambos os ambientes usam PostgreSQL em Docker no host, com instâncias/volumes separados e nomes de database distintos. “prod” nesta proposta significa perfil local com comportamento de produção e dados isolados; não significa acesso a serviço de produção remoto.

## Contrato proposto

### Seleção de ambiente e URL

- Opção pública proposta: --database-environment dev|prod, independente do --environment existente usado para perfil trading/Binance. Não modifica market, strategy, run mode nem seleção de credenciais de exchange.
- A seleção de banco é obrigatória para todo caller runtime que conecta, lê, escreve ou migra PostgreSQL. Ela é propagada a monitor, serve, backtest --persist, graph-projection e stores transitivos do bootstrap.
- A resolução usa exclusivamente BOT_DATABASE_URL_DEV para dev e BOT_DATABASE_URL_PROD para prod. Não há fallback cruzado nem fallback DATABASE_URL.
- Se seletor/URL estiver ausente, duplicado ou inválido, comando falha antes de resolver conexão, criar schema ou migrar; mensagem redige URL e segredos.
- Todos os callers runtime que leem DATABASE_URL são migrados ao resolver comum nesta entrega. Se um caminho não receber DatabaseEnvironment, ele falha fechado e não pode manter leitura direta/legada.
- O ambiente aparece em logs/metadados sem credenciais. Nenhuma opção implícita escolhe prod.

Variáveis candidatas para acordo:

| Variável | Uso proposto |
|---|---|
| BOT_DATABASE_URL_DEV | URL somente para o PostgreSQL local de desenvolvimento |
| BOT_DATABASE_URL_PROD | URL somente para o PostgreSQL local do perfil prod |
| BOT_DATABASE_ALLOW_REMOTE | Não proposta; nenhum flag libera destino remoto nesta entrega |

As URLs reais permanecem em backend/.env, com permissões restritas e fora do Git. Exemplos documentam apenas nomes/forma redigida, sem credenciais ou valores copiáveis que possam ser confundidos com segredo real. Segredos locais não são copiados para snapshot de testes ou logs.

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

A regra proposta é abrangente: todo caller runtime que conecta, lê, escreve, migra ou apaga dados PostgreSQL recebe o mesmo DatabaseEnvironment e usa o resolver. Nenhum caller runtime aceita DATABASE_URL após a migração. Se um caminho não puder receber o seletor, falha fechado antes de resolver URL ou conectar.

| Caller / ferramenta | Classificação e operação PostgreSQL | Caminho de seleção proposto |
|---|---|---|
| serve / AppDatabases::bootstrap_runtime (bootstrap_http_api) | runtime transitivo: bootstrap/connect/migrations; stores de agents, bots, owner e provider credentials leem/escrevem via AppDatabases | propaga --database-environment ao resolver comum; stores não leem env por conta própria |
| monitor / bootstrap_monitor_postgres | runtime: conexão e migrations do monitor | recebe DatabaseEnvironment; ausente → zero connect/migrate |
| optional_postgres_for_monitor_supervisor_snapshot() | runtime transitivo: consulta snapshot/supervisor usando PostgreSQL opcional | recebe contexto do bootstrap selecionado; não chama postgres_url_from_env nem aceita DATABASE_URL; sem resolver, permanece sem DB/falha fechado |
| backtest --persist / postgres_for_cli_persist | runtime: leitura/escrita de persistência de backtest | herda --database-environment global; persist=false não abre conexão |
| orders CLI retention-purge | runtime CLI: abre/migra e apaga registros via postgres_url_from_env | substituir postgres_url_from_env por DatabaseEnvironment/resolver; --database-environment obrigatório; sem fallback DATABASE_URL |
| graph-projection drain | runtime CLI: lê/escreve outbox e valida/migra PG; Neo4j é conexão separada | recebe DatabaseEnvironment no PG; Neo4j mantém seu próprio config/gate |
| run-pg-integration-tests.sh | runner de testes de integração, não caller runtime; executa testes PG individuais | usa runner dedicado, URL BOT_PG_TEST_DATABASE_URL, gate/identity/marker; rejeita DATABASE_URL fallback antes de iniciar testes |
| pg-v18-monitor-persistence-audit.sh | ferramenta de auditoria PG, não runtime do produto; acesso a banco só no alvo explicitamente validado | exige target/manifest isolado e URL de auditoria explícita; não faz fallback para DATABASE_URL nem escolhe dev/prod silenciosamente |
| verify-backend-full.sh | wrapper de verificação; não conecta diretamente, pode delegar ao runner PG | encaminha apenas para run-pg-integration-tests.sh com target/gate explícitos; no modo default não resolve DATABASE_URL |

Esta matriz consolida [integração dos módulos](./database-module-integration-sdd.md), [core database](./core-database-sdd.md), [configuração centralizada](./centralized-config-sdd.md) e [referência de CLI](../reference/cli-and-config.md), mais os callers/orders e scripts identificados no review. Antes do aceite de implementação, busca de código e inspeção de call graph devem confirmar a lista completa de acessos diretos/transitivos. Critério: nenhum acesso runtime residual a DATABASE_URL ou postgres_url_from_env; helpers transitivos recebem DatabaseEnvironment; scripts de teste/auditoria ficam em contratos explícitos separados e não são apresentados como comandos runtime.

## Acordo owner e pendências de seam

Acordo literal recebido do owner: “Sim: aprovo --database-environment separado e todos os callers no resolver”. Isso cobre a separação do seletor em relação a --environment de trading/Binance e o requisito de migrar todos os callers runtime PostgreSQL ao resolver. A matriz documental acima está fechada para os callers/scripts identificados neste ciclo; auditoria do source/call graph ainda deve confirmar cobertura completa antes do aceite de implementação.

O owner também já aprovou nomes URLs/no-fallback, isolamento dos ambientes e contrato de persistência do backtest conforme registrados neste SDD. O G1 independente aprovou a proposta com follow-up. Restam como detalhes de execução a construir/verificar: manifest e labels, binding e revalidação de endpoint, lifecycle e cleanup exatos, e prova de backup consistente/restore ou alvo vazio. Nenhum destino remoto está autorizado ou configurado.

A implementação pode avançar sob os seams aprovados; operação mutável de Compose, integração PostgreSQL e smoke do banco ficam bloqueados até revisão G3 independente e execução explicitamente autorizada.

## Produção remota e segurança

Não existe endpoint ou secret manager de produção no escopo conhecido. Portanto, BOT_DATABASE_URL_PROD significa exclusivamente a instância local prod. Este SDD não autoriza e não especifica conexão remota, deployment ou migração de dados reais. Quando existir infraestrutura de produção, uma proposta separada deverá definir identidade de workload, secret manager, TLS/CA, allowlist de rede, backup/PITR, privilégio mínimo, auditoria, migração sem downtime, readiness, restore drill e aprovação de lançamento.

Em produção remota futura, não reutilizar .env local ou credenciais Docker locais. Um modo remoto deve ser opt-in separado, com configuração de secret manager e validação de destino; não adicionar BOT_DATABASE_ALLOW_REMOTE como bypass genérico.

## Alternativa simples considerada

Manter um único DATABASE_URL e trocar seu valor manualmente entre dev e prod exige menos código, mas não associa o alvo à seleção --environment e torna fácil conectar/migrar o banco errado. Foi rejeitada para este objetivo porque não oferece separação observável por execução. Outra alternativa, dois TOMLs com URLs, mistura segredos com configuração e amplia o risco de commit; URLs permanecem em env local.

## Matriz TDD e validação proposta após acordo

| Seam | RED/GREEN após acordo | Observabilidade/efeito esperado |
|---|---|---|
| --database-environment | ausente/inválido/duplicado falha; dev/prod resolvem destinos distintos | connector falso recebe zero chamadas nos erros; --environment trading não muda |
| Resolver e env vars | seleciona só URL do ambiente; nenhum DATABASE_URL runtime/fallback | zero connect/migrate e erro sem URL/segredo |
| serve + stores transitivos | bootstrap, owner, agents, bots, provider credentials recebem DatabaseEnvironment | sem seletor zero connect/read/write/migrate |
| monitor + optional_postgres_for_monitor_supervisor_snapshot() | contexto selecionado propaga ao caminho auxiliar; flag ausente falha fechado | zero resolução por postgres_url_from_env e zero query sem seletor |
| backtest e orders retention-purge | persist=true e purge recebem o seletor; persist=false não abre PG | sem seletor zero connect/migrate/delete; ambiente trading não escolhe DB |
| graph-projection drain | PG outbox usa destino selecionado; Neo4j é independente | zero PG access sem seletor |
| run-pg-integration-tests.sh | integrações só rodam com runner/target/marker explícitos | default não lê DATABASE_URL nem conecta ao runtime DB |
| pg-v18-monitor-persistence-audit.sh | ferramenta de auditoria requer target manifestado, não é CLI runtime | recusa target ausente/ambíguo antes de query; sem fallback DATABASE_URL |
| verify-backend-full.sh | verificação composta delega integração somente ao runner separado | modo default não resolve URL nem cria conexão |
| Docker context local | endpoint Unix local e daemon identity validados permitem operação; DOCKER_HOST/DOCKER_CONTEXT, SSH/TCP/HTTP remoto ou context alterado falham | nenhuma operação mutável antes/depois de falha de prova local |
| Compose resource identity | labels/project/environment-id/container ID/volume ID/mount source/image/port corretos permitem operação; divergência aborta | zero mutações Docker em caso negativo; dev não alcança prod |
| Lifecycle e URL estável | up cria uma vez; stop/start/migrate/backup preservam environment-id, porta e URL; cleanup preserva volume; destroy explícito remove apenas target validado | manifest mapeia deterministicamente URL→project/container/port/image/volume/mount; não fica URL stale nem cria volume substituto silencioso |
| Schema/migration | alvo vazio migra; alvo ocupado requer backup consistente/restaurável | versão/checksum por alvo; database irmão inalterado |

Testes unitários são offline, usam sentinelas e connector falso, sem carregar .env real. Integrações de runner/auditoria usam target e manifest explícitos descartáveis; scripts documentam que não são comandos runtime e não herdam DATABASE_URL. Esta seção define evidência futura, não execução já realizada.

## Rollout local e rollback

1. Após acordo dos novos seams e G1, inventariar e mapear todos os callers runtime de DATABASE_URL antes de código.
2. Implementar --database-environment e resolver puro; provar RED/GREEN offline com connector falso.
3. Validar o Docker context/socket local e daemon identity, sem overrides, antes de qualquer operação mutável.
4. Criar uma vez projeto/container/volume persistente por ambiente com environment-id estável e porta host fixa; manifest liga deterministicamente URL→project/container/port/image/volume/mount; manter stack atual intocado.
5. Criar targets vazios ou exigir backup consistente/restaurável; aplicar migrations e comparar versão/checksum.
6. Migrar todos os callers da matriz em fatias pequenas, cada um exigindo DatabaseEnvironment; URL sem seletor falha fechado.
7. Configurar BOT_DATABASE_URL_DEV/PROD localmente; usar --database-environment explícito. Stop/start preserva URL e environment-id; revisar logs redigidos.

Rollback interrompe somente os container IDs validados, restaura configuração e backup consistente do mesmo environment-id se migration exigir; URL e porta continuam apontando para aquele manifest. Não recriar silenciosamente volume de stack persistente nem mudar para environment-id novo. Destroy é operação distinta, explícita e identificada por environment-id, com backup restaurável ou prova de vazio e remoção apenas após revalidar IDs/mounts. Nunca apontar prod para dev. Nenhuma ação remota está prevista.

## Riscos e pendências

- Há vários entrypoints que podem abrir PostgreSQL; auditoria do código runtime precisa provar que nenhum caller lê DATABASE_URL diretamente após a migração.
- Novo flag e remoção do fallback DATABASE_URL são mudanças públicas e requerem acordo antes de testes/implementação.
- Nomes, portas, roles e mounts do Compose devem vincular ao environment-id estável; validação de labels, IDs e mount sources precisa ser implementada antes de qualquer operação mutável.
- “prod local” ajuda a reproduzir configuração e migrations, mas não simula IAM, TLS, rede, backup ou operação de produção real.
- A disponibilidade local de migrations/extensões da imagem deve ser validada nos containers novos; nenhum banco já existente é evidência automática para esse contrato.

## G1 independente — ciclo final

**Veredito registrado:** APROVADO COM FOLLOW-UP pelo Critic independente `/root/test_safety_sdd_critic`. O review aplicou-se ao SDD canônico OpenKnowledge revisão `2640bafc0cc822d64be8734f5a2b880c7b5191b9`. Follow-up editorial: padronizar identidade persistente como `environment-id`, registrar ciclo/hash aqui e manter esse ID estável ao longo do lifecycle. Este parágrafo registra o veredito externo; não substitui a execução do follow-up nem o aceite G3 da implementação.

## Decisão solicitada e estado

A proposta revisada seleciona banco por --database-environment dev|prod, separado do --environment de trading; todos os callers runtime PostgreSQL passam pelo resolver, com runner de teste distinto. O owner aprovou literalmente: “Sim: aprovo --database-environment separado e todos os callers no resolver”. Compose local comprova endpoint/socket/daemon antes de mutações; environment-id persistente vincula URLs estáveis a recursos identificados e governa up/stop/start/migrate/backup/cleanup. Destino remoto não é configurado. Status permanece proposed; implementação G3 e follow-up independente ainda precisam ser verificados antes de aceite.