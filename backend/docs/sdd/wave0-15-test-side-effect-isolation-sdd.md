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

**Status: PROPOSED — aguardando G1 técnico independente.** O owner autorizou execução local com .env e Docker, alvos de integração descartáveis, preservação do banco persistente do app, backtest persistente sob auth e somente a ordem Spot Testnet dedicada. Não autorizou live trading nem teste no DB persistente. W0-02 segue draft/G1 pendente; este SDD não aprova sua revisão.

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

## PostgreSQL: runner local separado da CI (dependência W0-02)

W0-02 continua status draft/G1 pending. Seu review identifica questões no contrato do connector test-only compartilhado e no CI; T-W0-06 não edita W0-02 nem declara que foi aprovado.

**Runner local:** pode ser desenhado e validado independentemente do C4 de GitHub. Para cada execução cria container PG vazio e efêmero, database trading_bot, UUID aleatório, credencial randômica, volume efêmero e porta loopback exclusiva. O digest local candidato registrado em W0-02 é timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840; W0-02 registra inspeção local e PostgreSQL 18.6, Timescale 2.30.1, vector 0.8.6. Isso prova o alvo local documentado, não GitHub Actions.

Antes de liberar qualquer URL ao test process, runner verifica imagem digest/ID, container ID completo, label/run UUID, porta, nome DB e estado Docker; depois provisiona/verifica bot_test_database_marker fora das migrations, com UUID, container ID, digest e DB, e emite manifest privado da execução. Marker ausente/divergente, identidade não comprovada ou cleanup ambíguo invalida o alvo. Nunca reutiliza container.

Gate local exato BOT_RUN_PG_INTEGRATION=1; URL dedicada BOT_PG_TEST_DATABASE_URL obrigatória. DATABASE_URL presente é erro, sem fallback, mesmo que a URL dedicada exista. O helper compara URL/host/porta/DB/manifest com container ID/digest/UUID e marker antes de chamar o test connector ou migration. Sem gate retorna skip antes de resolver URL/conectar. O nome é trading_bot e o marker bot_test_database_marker; não inventar nomes alternativos.

O connector compartilhado fica bloqueado até W0-02 revisar seu contrato test-only. Ele deve preservar guard de nome, versão/extensões e URL compartilhados; connector de teste exige marker, runtime rejeita DB marcado. Antes de migration, alvo e marker devem estar comprovados pelo runner e helper. DB de runtime cuja tabela marker está ausente é considerado não marcado e pode prosseguir após verificações normais; só SQLSTATE de relation/table absent tem esse tratamento. Erros de permissão, conexão ou consulta falham fechados. Tabela presente com marker válido identifica DB de teste e runtime recusa.

**CI PostgreSQL:** C4 bloqueia apenas alterar/aceitar service container/job PG do GitHub, não todo uso local. Antes de tocar no workflow, evidência real de execução GitHub precisa comprovar digest exato, inicialização do service, health, server_version_num >= 180000, extensions timescaledb e vector e jobs relevantes verdes, com link/log seguro. Se essa prova ainda não existe, C4 permanece residual CI bloqueante. Resultado local do digest não prova GitHub service. W0-02 permanece draft/G1 pending e nenhum status de aprovação é inferido.

## Neo4j: leitura e escrita em opt-ins distintos

BOT_RUN_NEO4J_READ_INTEGRATION=1 habilita somente leitura; BOT_RUN_NEO4J_WRITE_INTEGRATION=1 habilita somente testes classificados como write. Ausente/0 significa zero config resolution, driver construction e connection. .env, BOT_GRAPH_ENABLED, BOT_AGENTS_ENABLED e credenciais não habilitam integração.

Cada runner cria container Neo4j vazio/descartável com UUID, ID, digest, database e marker de modo. Read só conecta após provar role/database efetivamente read-only; se não provar, falha antes do driver. Teste que escreve na preparação é classificado write ainda que depois consulte. Write exige gate próprio e marker mode=write. Targets runtime nunca são usados.

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

## Validação observável

Depois de G1, TDD por seam; não declarar RED/GREEN ou zero attempts sem executar e guardar resultado seguro.

1. Cargo direto com ambiente sentinela: zero chamadas PG/migration, Neo4j, HTTP externo e Binance client/submit. Wrapper soma prova de egress completa; loopback de mock é permitido; DNS/UDP/TCP/redirect e trace malformado reprovam.
2. PG sem gate: skip pré-URL; DATABASE_URL sozinho ou junto da URL dedicada: erro antes do connector; gate sem target/identity/marker: falha pré-test connection/migration; somente container trading_bot validado chega ao connector. Runtime permite marker table ausente como DB não marcado, falha em erros de query e recusa DB marcado.
3. Neo4j read/write gates desligados: contador zero antes do driver. Read exige enforcement read-only; write exige target novo com marker write.
4. Binance com credenciais e sem gate: zero client; filtro/args inválidos falham pré-client; submit real somente pela seleção exata Spot Testnet já autorizada.
5. Backtest: persist=false, 503 e 401 sem store; teste bearer válido escreve apenas no PG ephemeral; serve persiste no store runtime após auth.
6. Self-test do wrapper: auditoria de descendentes, egress bloqueado, io_uring registrado como EPERM e parser fail-closed.
7. C4 GitHub: não mudar/aceitar service job até execução real provar init/health/version/extensions. Não confundir esse gate de CI com runner local.

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
