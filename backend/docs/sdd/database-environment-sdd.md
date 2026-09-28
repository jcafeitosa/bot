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

- O CLI recebe --environment dev ou --environment prod uma única vez no escopo global e propaga a mesma seleção ao monitor, serve, backtest e comandos de banco que aceitam conexão.
- A resolução usa exclusivamente BOT_DATABASE_URL_DEV para dev e BOT_DATABASE_URL_PROD para prod. Não há fallback cruzado entre variáveis.
- Se a variável selecionada estiver ausente, vazia, inválida ou apontar para ambiente não permitido localmente, o comando falha antes de conectar, criar schema ou migrar. O diagnóstico informa a variável ausente/inválida sem imprimir a URL.
- DATABASE_URL permanece fora do novo caminho após migração. A implementação não o consulta como fallback para dev/prod. A retirada de usos legados de DATABASE_URL ocorre em entrega separada, com inventário de callers e janela de compatibilidade; até lá, caminhos legados não aceitam --environment como se fossem isolados.
- Nenhuma opção implícita escolhe prod. Para comandos que podem persistir, ausência de --environment falha antes de resolver URL. O valor selecionado aparece em logs/metadados sem credenciais.

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

Migration executa apenas após validar ambiente, container, database, role e identidade do volume. A mesma cadeia versionada de migrations e schema é aplicada a dev e prod localmente; não se mantém schema divergente por ambiente. Antes de migrar, o runner registra a versão atual e tira backup local do volume selecionado, ou exige banco vazio quando backup confiável não é possível. Migrations são transacionais quando suportado; falha interrompe o comando e preserva log redigido.

Os documentos [core database](./core-database-sdd.md), [integração de módulos](./database-module-integration-sdd.md) e [configuração centralizada](./centralized-config-sdd.md) descrevem o seam PostgreSQL e as camadas de configuração existentes. Este SDD propõe seleção por ambiente sem declarar que esses contratos foram alterados ou aprovados.

### Compose local

Compose deve oferecer comandos separados para subir, health-check, migrate, backup e parar cada ambiente. Um comando dev não pode iniciar, parar, resetar ou migrar postgres-prod. O serviço prod local não publica porta em todas as interfaces; se a aplicação precisar conectar, usa rede Docker privada ou bind loopback. Senhas de role são distintas e geradas localmente; o compose não contém valores de segredo em texto fixo.

Antes de implementar, inventariar docker-compose.bot.yml e containers/volumes existentes. Não reatribuir volume, trocar imagem, apagar container nem executar migrations em alvos atuais como parte da entrega sem plano de migração revisado. Criar stack nova com nomes e volumes inequívocos; rollback mantém o stack antigo intacto.

## Seams que exigem acordo explícito antes de testes

1. Se --environment é obrigatório para todo caminho mutável/que persiste ou somente para comandos ligados a banco; proposta: obrigatório para qualquer comando que possa escrever no PostgreSQL.
2. Nomes BOT_DATABASE_URL_DEV e BOT_DATABASE_URL_PROD, precedência e rejeição total de fallback DATABASE_URL; proposta: nomes exatos acima, sem fallback.
3. Se prod local usa container/volume independente; proposta: instância, role, database e volume independentes de dev.
4. Política de host aceito e proof of identity do container Compose antes de connect/migrate; proposta: somente container criado/identificado pelo stack local e nenhuma URL remota.
5. Se backtest persist=true herda a seleção de ambiente ou exige um parâmetro explícito próprio; proposta: herda --environment e só grava no alvo local selecionado.

Até esses contratos receberem acordo do owner e G1, não criar testes de contrato desses seams nem alterar parser, configuração, connector, migrations ou Compose.

## Produção remota e segurança

Não existe endpoint ou secret manager de produção no escopo conhecido. Portanto, BOT_DATABASE_URL_PROD significa exclusivamente a instância local prod. Este SDD não autoriza e não especifica conexão remota, deployment ou migração de dados reais. Quando existir infraestrutura de produção, uma proposta separada deverá definir identidade de workload, secret manager, TLS/CA, allowlist de rede, backup/PITR, privilégio mínimo, auditoria, migração sem downtime, readiness, restore drill e aprovação de lançamento.

Em produção remota futura, não reutilizar .env local ou credenciais Docker locais. Um modo remoto deve ser opt-in separado, com configuração de secret manager e validação de destino; não adicionar BOT_DATABASE_ALLOW_REMOTE como bypass genérico.

## Alternativa simples considerada

Manter um único DATABASE_URL e trocar seu valor manualmente entre dev e prod exige menos código, mas não associa o alvo à seleção --environment e torna fácil conectar/migrar o banco errado. Foi rejeitada para este objetivo porque não oferece separação observável por execução. Outra alternativa, dois TOMLs com URLs, mistura segredos com configuração e amplia o risco de commit; URLs permanecem em env local.

## Validação proposta após acordo

- Testes unitários de resolução de ambiente verificam URL exata, variável ausente, variável vazia, variável do outro ambiente, URL inválida e redaction, sem abrir conexão.
- Testes de integração usam containers locais novos por execução, com identidade de container/volume validada antes de conexão/migration; exercitam schema e migrations nas duas variantes sem ler .env real.
- Testes demonstram que comando dev não consegue parar/resetar/migrar prod e que URL remota é recusada antes de socket/connect.
- Prova de migração compara versão de schema e conjunto de migrations aplicadas nos dois bancos; falha de migration não altera o outro ambiente.
- Execução manual local verifica health, conexão de serve e persistência backtest com dados descartáveis. Relatório não inclui URL, senha, token ou conteúdo sensível.

Esta seção descreve critérios futuros, não evidência de testes já executados.

## Rollout local e rollback

1. Após aprovação G1 e acordo de seams, implementar primeiro resolução/config pura e testes sem conexão.
2. Adicionar novos serviços Compose e volumes separados sem substituir containers/volumes existentes; validar health e identidade.
3. Criar databases vazios, aplicar migrations e executar smoke checks limitados a dados descartáveis.
4. Configurar BOT_DATABASE_URL_DEV e BOT_DATABASE_URL_PROD localmente; iniciar cada comando com --environment explícito e revisar logs redigidos.
5. Migrar callers legados de DATABASE_URL em fatias separadas; manter bloqueio fail-closed se caller não indicar ambiente.

Rollback interrompe processos, restaura configuração local anterior e seleciona os containers/volumes antigos sem apagá-los. Se uma migration tiver alterado o novo volume, recriar somente o volume descartável correspondente ou restaurar seu backup; nunca apontar rollback para o outro ambiente. Nenhuma ação remota está prevista.

## Riscos e pendências

- Há vários entrypoints que podem abrir PostgreSQL; inventário completo dos callers e de quais aceitam --environment é pré-requisito de implementação.
- DATABASE_URL é usado por código e documentação existentes; compatibilidade precisa ser delimitada sem permitir que ela contorne a nova seleção.
- Nomes de serviços, portas, roles, volume, checks de identidade e política de dados seed precisam de confirmação no inventário do Compose local.
- “prod local” ajuda a reproduzir configuração e migrations, mas não simula IAM, TLS, rede, backup ou operação de produção real.
- A disponibilidade local de migrations/extensões da imagem deve ser validada nos containers novos; nenhum banco já existente é evidência automática para esse contrato.

## Decisão solicitada e estado

A decisão proposta é selecionar exclusivamente por --environment e usar BOT_DATABASE_URL_DEV/BOT_DATABASE_URL_PROD com dois alvos PostgreSQL Docker independentes; bloquear fallback DATABASE_URL e qualquer destino remoto. Aguardam-se: revisão técnica independente G1 e acordo explícito do owner sobre os cinco seams acima. Até então, status permanece proposed; não implementar, alterar bancos ou executar testes.