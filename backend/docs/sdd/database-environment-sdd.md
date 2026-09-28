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

**Status: PROPOSED — requer revisão G1 independente e acordo explícito dos seams antes de criar testes ou implementar.** O objetivo autorizado é ter execução local dev e prod usando bancos locais distintos. Não há endpoint nem secret manager real de produção fornecido; este SDD não cria nem presume um.

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

Compose provisiona dois serviços locais separados, cada um com container, volume, database e role próprios. Os serviços têm nomes explícitos, por exemplo postgres-dev e postgres-prod; volume nunca é compartilhado. Portas host distintas ou redes internas isoladas impedem colisão. O runner verifica que o destino resolve para o container/porta local esperado antes de qualquer migration e recusa host externo, alias de rede desconhecido ou container preexistente não identificado pelo próprio stack.

O banco prod local começa vazio e isolado. Não se restaura dump de produção nem se copia conteúdo de dev por padrão. Dados de mercado públicos e fixtures sintéticas podem ser carregados por ferramenta explicitamente identificada; dados de usuário, tokens, credenciais de exchange e provider não entram no seed. O banco dev pode ser recriado sem afetar o volume prod.

Migration executa apenas após validar ambiente, database, role e identidade Docker. A mesma cadeia versionada é aplicada em dev e prod local, sem schema divergente. Antes da primeira migration, provar alvo vazio. Para alvo não vazio, exigir backup consistente prévio associado ao database/container/volume e evidência de restore verificável; se isso não puder ser demonstrado, recusar migration e criar alvo vazio novo. Migration falha interrompe o comando, preserva log redigido e não altera o outro ambiente.

Os documentos [core database](./core-database-sdd.md), [integração de módulos](./database-module-integration-sdd.md) e [configuração centralizada](./centralized-config-sdd.md) descrevem o seam PostgreSQL e as camadas de configuração existentes. Este SDD propõe seleção por ambiente sem declarar que esses contratos foram alterados ou aprovados.

### Compose local

Compose deve criar projetos exclusivos por ambiente e execução, como bot-db-env-dev-<run-id> e bot-db-env-prod-<run-id>, com nomes únicos para containers e volumes. Não usar nome default por diretório nem reutilizar recursos do docker-compose.bot.yml. Antes de cada start/stop/backup/migrate/cleanup, validar project/environment/run labels, container ID, image ID/digest, volume ID e mount source do manifest. Qualquer divergência aborta sem mutação.

Cada operação nomeia explicitamente project + service/container + volume validados. Proibidos: down sem identidade validada, down -v, docker system prune, volume prune, rm amplo ou nomes parciais. Cleanup só remove IDs criados pelo runner após revalidar labels/IDs/mount sources. Dev nunca opera recursos prod. Inventariar stack atual antes de criar recursos; não reatribuir volumes nem migrar o stack existente. Bind somente loopback ou rede privada Docker; credenciais não ficam em texto fixo no Compose.

## Matriz de callers runtime e contrato

| Caller | Uso PostgreSQL documentado | Mudança proposta |
|---|---|---|
| serve / AppDatabases::bootstrap_runtime (bootstrap_http_api) | bootstrap, hidratação de registry/owner e writes de stores habilitados | recebe DatabaseEnvironment; URL, connector e migrations usam resolver comum |
| monitor / bootstrap_monitor_postgres e persistência opcional de market data | conexão e migration quando persistência está ativa | propaga --database-environment; sem flag não resolve URL nem conecta |
| backtest --persist / postgres_for_cli_persist | escrita de resultados | herda database-environment global; persist=false não abre PG |
| graph-projection drain | leitura/escrita de outbox e migration/validação PG | recebe o seletor; Neo4j segue configuração própria |
| stores providers credentials, bots, agents e product-owner | leitura/escrita via bootstrap/store | sem leitura própria de URL; recebem AppDatabases já selecionado |
| runner PostgreSQL de testes | conexão e migration de integração | seam separado: exige BOT_PG_TEST_DATABASE_URL + gate/identity/marker do runner; DATABASE_URL não é fallback |

Este inventário deriva de [integração dos módulos](./database-module-integration-sdd.md), [core database](./core-database-sdd.md), [configuração centralizada](./centralized-config-sdd.md) e [referência de CLI](../reference/cli-and-config.md). Implementação deve auditar toda leitura direta/transitiva de DATABASE_URL e mapear símbolo runtime a DatabaseEnvironment; qualquer caller sem resolução comum reprova aceite.

## Seams que exigem novo acordo explícito antes de testes

A autorização anterior cobriu os cinco seams do rascunho original. Esta revisão altera flag pública e migra todos os callers, por isso exige novo acordo para: (1) --database-environment dev|prod ortogonal a --environment de trading/Binance; (2) nomes BOT_DATABASE_URL_DEV/PROD e ausência de fallback DATABASE_URL; (3) migração de todos os callers da matriz e fail-closed nos sem seletor; (4) Compose com nomes por run e validação de project labels, IDs e mount sources, sem down/prune amplo; (5) backup consistente/restaurável em banco não vazio ou exigência de target vazio; (6) backtest persist=true herdar seletor e persist=false não abrir PG.

Até acordo e G1, não criar testes dependentes desses seams nem alterar parser, config, connector, migrations ou Compose.

## Produção remota e segurança

Não existe endpoint ou secret manager de produção no escopo conhecido. Portanto, BOT_DATABASE_URL_PROD significa exclusivamente a instância local prod. Este SDD não autoriza e não especifica conexão remota, deployment ou migração de dados reais. Quando existir infraestrutura de produção, uma proposta separada deverá definir identidade de workload, secret manager, TLS/CA, allowlist de rede, backup/PITR, privilégio mínimo, auditoria, migração sem downtime, readiness, restore drill e aprovação de lançamento.

Em produção remota futura, não reutilizar .env local ou credenciais Docker locais. Um modo remoto deve ser opt-in separado, com configuração de secret manager e validação de destino; não adicionar BOT_DATABASE_ALLOW_REMOTE como bypass genérico.

## Alternativa simples considerada

Manter um único DATABASE_URL e trocar seu valor manualmente entre dev e prod exige menos código, mas não associa o alvo à seleção --environment e torna fácil conectar/migrar o banco errado. Foi rejeitada para este objetivo porque não oferece separação observável por execução. Outra alternativa, dois TOMLs com URLs, mistura segredos com configuração e amplia o risco de commit; URLs permanecem em env local.

## Matriz TDD e validação proposta após acordo

| Seam | RED/GREEN após acordo | Observabilidade/efeito esperado |
|---|---|---|
| --database-environment | ausente/inválido/duplicado falha; dev e prod resolvem destinos diferentes | connector falso recebe zero chamadas nos erros; --environment trading não muda |
| Resolver e env vars | escolhe só BOT_DATABASE_URL_DEV/PROD; DATABASE_URL isolada ou presente como fallback é rejeitada | zero connect/migrate e erro sem URL/segredo |
| Callers runtime | serve, monitor, backtest, graph-projection e stores recebem mesmo DatabaseEnvironment | sem seletor todos registram zero conexão/read/write/migrate |
| Compose resource identity | labels/container ID/volume ID/mount source corretos permitem operação; divergência aborta | nenhum comando Docker mutável no caso negativo; tentativa dev não alcança prod |
| Schema/migration | alvo vazio migra; alvo ocupado requer backup consistente comprovável | versão/checksum por alvo; database irmão inalterado |
| Test runner separado | BOT_PG_TEST_DATABASE_URL só via runner validado; DATABASE_URL nunca fallback | sem gate/runner, zero connect/migrate; runtime recusa marker de teste |

Testes unitários são offline, usam valores-sentinela e connector falso, sem carregar .env real. Integração usa apenas runner descartável autorizado; esta seção define evidência futura e não afirma execução já realizada.

## Rollout local e rollback

1. Após acordo dos novos seams e G1, inventariar e mapear todos os callers runtime de DATABASE_URL antes de código.
2. Implementar --database-environment e resolver puro; provar RED/GREEN offline com connector falso.
3. Criar projetos, containers e volumes Compose novos com run-id; validar labels, IDs e mount sources; manter o stack atual intocado.
4. Criar targets vazios ou exigir backup consistente/restaurável; aplicar migrations e comparar versão/checksum.
5. Migrar todos os callers da matriz em fatias pequenas, cada um exigindo DatabaseEnvironment; URL sem seletor falha fechado.
6. Configurar BOT_DATABASE_URL_DEV/PROD localmente; usar --database-environment explícito em cada execução; revisar logs redigidos.

Rollback interrompe processos, restaura configuração local anterior e seleciona os containers/volumes antigos sem apagá-los. Se uma migration tiver alterado o novo volume, recriar somente o volume descartável correspondente ou restaurar seu backup; nunca apontar rollback para o outro ambiente. Nenhuma ação remota está prevista.

## Riscos e pendências

- Há vários entrypoints que podem abrir PostgreSQL; auditoria do código runtime precisa provar que nenhum caller lê DATABASE_URL diretamente após a migração.
- Novo flag e remoção do fallback DATABASE_URL são mudanças públicas e requerem acordo antes de testes/implementação.
- Nomes, portas, roles e mounts do Compose devem ser por run-id; validação de labels, IDs e mount sources precisa ser implementada antes de qualquer operação mutável.
- “prod local” ajuda a reproduzir configuração e migrations, mas não simula IAM, TLS, rede, backup ou operação de produção real.
- A disponibilidade local de migrations/extensões da imagem deve ser validada nos containers novos; nenhum banco já existente é evidência automática para esse contrato.

## Decisão solicitada e estado

A proposta revisada seleciona banco exclusivamente por --database-environment dev|prod, ortogonal ao --environment de trading; usa BOT_DATABASE_URL_DEV/PROD sem fallback e migra todos os callers runtime PostgreSQL ao resolver comum; separa runner de testes; e opera somente projetos/containers/volumes novos após validar labels, IDs e mount sources. Destino remoto não é configurado. Aguardam-se novo acordo explícito dos seams materiais e revisão G1 independente. Até então, status permanece proposed; não implementar, alterar bancos ou executar testes.