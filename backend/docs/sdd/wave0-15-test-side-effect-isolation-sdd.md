---
title: SDD T-W0-06 — Isolar efeitos externos da suíte de testes
description: Contrato para isolar efeitos de testes e separar runners locais efêmeros do gate PG CI.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: proposed
---
# SDD T-W0-06 — Isolar efeitos externos da suíte de testes

**Status: PROPOSED — a implementação T-W0-06a foi reprovada em G3 e permanece pendente.** O desenho revisado exige nova aprovação G1 independente antes de implementação. O owner autorizou execução local com .env e Docker, alvos de integração descartáveis, preservação do banco persistente do app, backtest persistente sob auth e somente a ordem Spot Testnet dedicada. Não autorizou live trading nem testes no banco persistente. W0-02 segue draft/G1 pendente; este SDD não aprova sua revisão.

## Contexto e objetivo

Cargo test pode carregar configuração e credenciais locais. Credenciais, DATABASE_URL ou BOT_GRAPH_ENABLED nunca podem funcionar como opt-in. Integrações PG, Neo4j, providers e Binance precisam de gates explícitos antes de resolver configuração, abrir conexão ou construir client.

Objetivo: Cargo test direto mantém efeitos externos desativados por gates nos seams conhecidos; o wrapper dedicado acrescenta isolamento de rede e prova syscall-level de que processos e descendentes não fazem egress. Cargo direto não substitui essa auditoria dinâmica; chamadas diretas a socket são comprovadamente bloqueadas somente no wrapper.

## Cargo direto versus wrapper

- **Cargo direto:** gates PG/Neo4j/Binance desligados por padrão; testes de provider usam mocks locais. Presença de credenciais ou .env não ativa testes de integração. Critério é zero chamadas a connector, migration, Neo4j driver, provider HTTP ou Binance client/submit sem o respectivo gate.
- **Wrapper default:** executa a suite completa dentro de Docker --network=none, com seccomp default deny, strace -f e auditor fail-closed. Não monta checkout, .env real, config Cargo local, .git, logs, target ou socket Unix do host. Sem montagem de Docker/container-runtime socket, host network ou modo privilegiado.
- O snapshot é filtrado e allowlisted: arquivos fonte rastreados necessários mais os scripts/testes de isolamento locais, mesmo se novos e ainda não rastreados. Config usada por fixture vem de valores sentinela não credenciais criados apenas para aquele processo; não herda ambiente externo nem .env real.
- Cargo usa CARGO_NET_OFFLINE=true, --locked --offline, registry/cache previamente provisionado read-only em /cargo/registry e CARGO_HOME=/cargo; target/audit temporários por execução. Imagem é pinada por digest/ID e strace por versão. Sem imagem, cache, arquitetura ou profile compatível, aborta antes de Cargo; sem build/pull/fallback implícito.
- Docker usa --network=none, --cap-drop=ALL e somente SYS_PTRACE, no-new-privileges e usuário sem privilégio. Profile seccomp restritivo nega io_uring_setup/register/enter com EPERM; os três syscalls são explicitamente incluídos no filtro strace e auditor. Wrapper valida arquitetura/profile antes de iniciar imagem e falha fechado em host incompatível.
- Auditor permite somente mocks loopback dentro do namespace. DNS na porta 53 é proibido também em loopback; UDP/TCP externo, redirect, syscall de rede malformada ou endereço não interpretável falha mesmo se aplicação ignorou o erro. Socket Unix do host não é montado. O self-test adversarial deve provar mocks locais permitidos, DNS/UDP/TCP/redirect bloqueados e as três chamadas io_uring observadas com EPERM.
- Observer classifica pg_connect, pg_migrate, neo4j_connect/read/write, provider_http (System One/JEV/NIM e outros clients externos) e binance_submit. Ele complementa strace; não substitui a auditoria de todos os processos/descendentes. Default: zero tentativas de DB/provider/exchange e zero DNS/egress.

## PostgreSQL: runner local efêmero separado da CI (dependência W0-02)

W0-02 continua draft/G1 pendente. Sua revisão identifica questões no connector test-only compartilhado e no CI; T-W0-06 não edita W0-02 nem declara sua aprovação. A revisão G3 atual encontrou falso-green quando falha de conexão/migration pode ser convertida em skip/retorno normal. Esta revisão fecha esse comportamento antes de nova implementação.

**Alvo local:** cada execução elegível usa somente container PG vazio, efêmero, criado pelo runner dedicado, com UUID/run-id, container ID, digest de imagem, credencial aleatória, porta loopback exclusiva e volume descartável. O candidato local registrado em W0-02 é `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`; essa evidência descreve um alvo local e não prova o comportamento de GitHub Actions.

Antes de iniciar o binário, o runner verifica daemon local, imagem/digest, container ID completo, labels/run-id, porta, database e mounts contra um manifest privado da execução. Cria e valida `bot_test_database_marker` antes de liberar `BOT_PG_TEST_DATABASE_URL` ao teste; o marker vincula run-id, container ID, digest e nome do database. Marker ausente, ambíguo ou divergente invalida o alvo antes de conectar ou migrar. O nome `trading_bot`, se mantido por compatibilidade com W0-02, só pode existir dentro desse container efêmero e nunca identifica, seleciona ou autoriza o banco persistente da aplicação. Nome de database isolado não é prova de identidade.

**Gates e resultado:** os testes PostgreSQL são ignorados pelo Cargo default e só o runner dedicado pode invocá-los com nome exato e `BOT_RUN_PG_INTEGRATION=1`. Credenciais, `.env`, `DATABASE_URL` e nome `trading_bot` não são opt-in nem seletor de destino. `BOT_PG_TEST_DATABASE_URL` é obrigatória; `DATABASE_URL` herdada presente é erro, sem fallback. O helper recebe a prova de alvo/manifest/marker do runner e valida host, porta, database e identidade antes de connector/migration. Sem opt-in, teste fica explicitamente ignored e não resolve URL. Opt-in com alvo ausente/inválido, identity/marker incorreto, falha de conexão, migration ou query produz falha explícita do teste e código de saída não zero — nunca skip ou sucesso silencioso.

O connector de teste compartilhado continua bloqueado até W0-02 revisar seu contrato. Runtime não pode selecionar um alvo de teste por `DATABASE_URL` nem operar um container/volume efêmero do runner; teste não pode herdar o banco persistente operacional. A tabela marker ausente em banco runtime só significa “sem marker” se a consulta retornar especificamente relation/table absent (SQLSTATE definido pelo connector); erro de conexão, permissão ou consulta falha fechado. Marker presente e válido identifica alvo de teste, que o runtime recusa. O guard de nome, URL, versão e extensões compartilhados permanece dependência de revisão W0-02.

**CI PostgreSQL:** C4 bloqueia apenas alterar/aceitar service container/job PG do GitHub, não todo uso local. Antes de tocar no workflow, evidência real de execução GitHub precisa comprovar digest exato, inicialização do service, health, server_version_num >= 180000, extensions timescaledb e vector e jobs relevantes verdes, com link/log seguro. Se essa prova ainda não existe, C4 permanece residual CI bloqueante. Resultado local do digest não prova GitHub service. W0-02 permanece draft/G1 pending e nenhum status de aprovação é inferido.

## Neo4j: leitura e escrita em gates, alvos e roles separados

`BOT_RUN_NEO4J_READ_INTEGRATION=1` habilita somente testes classificados como leitura; `BOT_RUN_NEO4J_WRITE_INTEGRATION=1` habilita somente testes que escrevem. Ausente, `0` ou outro valor significa teste explicitamente ignored, sem ler configuração, resolver credenciais, construir driver ou conectar. Os dois gates são independentes: habilitar leitura não habilita escrita e vice-versa. `.env`, `BOT_GRAPH_ENABLED`, `BOT_AGENTS_ENABLED` e credenciais nunca provam opt-in nem identidade do alvo.

Cada execução elegível cria um container Neo4j efêmero e vazio, com run-id, container ID, digest, database isolado e marker `BotTestTarget` que contém run-id, container ID, digest, database e `mode=read|write`. O runner valida o manifest e o marker antes de liberar configuração ao teste; configuração do `.env` não serve como alvo alternativo nem como prova.

A configuração de conexão do teste é injetada pelo runner após validar a identidade, sem carregar endpoints/credenciais do `.env`. Read exige uma role efetivamente read-only e uma prova executável de que escrita é negada; se o enforcement ou marker não puder ser validado, falha antes de construir o driver. Qualquer fixture/preparação com escrita é teste write. Write exige seu gate e marker `mode=write`, e só usa o container descartável daquela execução. Falha de conexão, permissão ou query após opt-in falha o teste em vez de virar skip. Targets e credenciais runtime nunca são reutilizados.

## Binance Spot Testnet

BOT_RUN_BINANCE_TESTNET_ORDER=1 é obrigatório e independente das credenciais. Somente runner dedicado com teste canônico selecionado por nome completo e --exact pode chegar à construção do client. Args extra, filtro parcial, modo paralelo ou nome diferente falham antes de iniciar o binário. Gate é verificado antes de ler credenciais; URL Spot Testnet é fixa. Sem gate, credenciais presentes produzem zero client/transport/submit. Autorização do owner cobre apenas essa ordem Spot Testnet dedicada.

## Backtest persistente: runtime e teste separados

persist=false continua cálculo público. persist=true exige admin Bearer válido antes de resolver store/config.

- **serve/runtime:** auth válida precede store e URL; então usa store normal configurado para o serviço. Token ausente/fraco: 503; bearer ausente/incorreto: 401, sem resolver PG.
- **Cargo default/unitário:** persist=false calcula com fixture válida e zero store; 503/401 provam gateway/store não chamados. Não usa o store do serve.
- **Teste persist=true com bearer válido:** somente runner PG injeta store ligado ao container/manifest efêmero já verificado. Nunca chama postgres_for_cli_persist nem herda DATABASE_URL. Sem target/marker não cria store.

| Caso | Resultado | Destino |
|---|---|---|
| persist=false | cálculo público | nenhum banco |
| persist=true sem token | 503 admin_auth_not_configured | nenhum banco |
| persist=true bearer inválido | 401 unauthorized | nenhum banco |
| persist=true bearer válido no teste | escrita provada | PG efêmero validado |
| persist=true bearer válido no serve | persistência após auth | store runtime do serviço |

## Alternativas consideradas

- Pedir ao operador limpar .env antes de Cargo: rejeitado, frágil e não protege chamadas diretas.
- Somente wrapper: rejeitado, pois Cargo direto continuaria inseguro.
- Somente gates: não prova que chamadas diretas/dependências foram bloqueadas; wrapper complementa com network-none/strace.
- Reusar DB/Neo4j runtime ou contornar o guard com outro nome: rejeitado; targets novos têm digest, identidade e marker.
- Inferir aprovação do CI a partir de teste local: rejeitado; C4 exige evidência GitHub.
- Usar store runtime em testes persistentes: rejeitado, pois ameaça dados do app e mistura seams.

## Validação TDD observável

A implementação anterior não tem aceite G3. Após G1 independente, cada seam revisado exige uma prova RED seguida de GREEN; não declarar skip como sucesso nem zero tentativas sem contadores/observadores explícitos.

1. **Default sem efeitos:** com sentinelas de credenciais, `.env`, `DATABASE_URL`, `BOT_GRAPH_ENABLED` e endpoints providers presentes, Cargo default mostra testes de integração como ignored e observadores confirmam zero resolução de config/URL, connector, migration, Neo4j driver, provider HTTP ou Binance client/submit. O wrapper adicionalmente audita descendentes/egress. Mocks loopback são permitidos; DNS, UDP/TCP externo, redirect, trace malformado e io_uring sem EPERM reprovam.
2. **PG sem gate:** RED prova que nenhuma URL/connector/migration é consultado e o teste permanece ignored. **Gate `=1`, URL/target faltante ou `DATABASE_URL` isolada:** RED deve falhar antes do connector; não há fallback. **Manifest/marker incompatível, ou alvo persistente da aplicação:** falha antes da conexão/migration. **Target efêmero válido:** runner chama teste exato; connection, migration ou query falha deve fazer teste e runner retornarem não zero, nunca skip. Nome `trading_bot` no target efêmero não basta sem ID/manifest/marker.
3. **Neo4j default:** com `.env` e credenciais presentes, ambos os gates fechados deixam resolução de config, driver e conexão em zero; integração aparece ignored. **Read gate habilitado:** target+marker read válido e role read-only permite somente consulta; ausência/incompatibilidade de marker, alvo runtime ou escrita não bloqueada falha antes do driver. **Write gate habilitado:** exige target descartável e marker write; falha de conexão/query resulta em não zero. Read e write não habilitam um ao outro.
4. **Binance:** credenciais sem gate deixam zero lookup de credenciais/client/transport/submit. Gate inválido, nome de teste ou args diferentes de seleção exata falham antes do client. Submit real somente por Spot Testnet dedicado já autorizado.
5. **Backtest:** `persist=false`, 503 e 401 mantêm contador de store zero; bearer válido escreve somente com PG efêmero; `serve` persiste no store runtime após auth.
6. **Wrapper:** self-test prova auditoria de descendentes, egress bloqueado, io_uring observado como EPERM e parser fail-closed.
7. **C4 GitHub:** não alterar/aceitar service job até execução real provar init/health/version/extensions. Resultado local não satisfaz este gate CI.

## Rollout, rollback e riscos

Ordem: aceitar G1 T-W0-06; implementar gates; Critic revisar T-W0-06a; validar runner local/identidade; resolver em W0-02 o connector compartilhado; executar integrações só em containers efêmeros; depois avaliar CI sob C4. Nenhum serviço operacional, token admin ou ordem é iniciado durante G1.

Rollback reverte wrapper/gates/runners dependentes juntos e mantém integrações desligadas. Container sem digest/UUID/marker ou cleanup incerto nunca é reutilizado. Host/profile/cache sem prova falha fechado. C4 é residual bloqueante apenas para workflow PG CI.

## Resolução dos findings G1

1. Cargo direto usa gates nos seams conhecidos; wrapper acrescenta garantia syscall-level contra egress arbitrário. Ambos deixam integrações off por default.
2. Snapshot exclui .env real e config local. Fixtures recebem apenas valores sentinela não credenciais.
3. Runner PG local efêmero é uma trilha separada, sustentada pela evidência local do digest/version/extensions. Connector compartilhado aguarda revisão do contrato W0-02. CI fica bloqueada por C4 até evidência real GitHub. Não se afirma W0-02 aprovado.
4. Store runtime do serve e store do teste persistente são separados. Runtime somente após auth; testes somente com injeção ligada ao target PG efêmero.

## Estado

- T-W0-06: **PROPOSED — aguardando G1 independente**; não aprovado para implementação por este documento.
- W0-02: **draft/G1 pendente**. C4 bloqueia alteração/aceite do job PG CI até prova de GitHub Actions. Review do connector compartilhado continua dependência.
- Escopo autorizado pelo owner: execução local, dados persistentes protegidos, runner PG/Neo4j efêmero, backtest runtime autenticado, ordem dedicada Spot Testnet apenas.
- Nenhuma evidência de CI GitHub foi alegada.
