---
title: SDD T-W0-06 — Isolar efeitos externos da suíte de testes
description: Contrato para tornar cargo test inofensivo por padrão, isolar integrações em alvos efêmeros e auditar egress.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: proposed
---
# SDD T-W0-06 — Isolar efeitos externos da suíte de testes

**Status: PROPOSED — aguardando G1 independente.** O owner autorizou uso local de .env e Docker local, com proteção dos bancos persistentes, alvos PG/Neo4j descartáveis, persistência do backtest sob autenticação e somente uma ordem dedicada Spot Testnet. A autorização não cobre live trading nem testes no banco persistente do app. A revisão de W0-02 permanece draft/G1 pendente; este documento não a aprova.

## Contexto e objetivo

A suite Rust pode herdar credenciais ou carregar .env. Presença de URL, credenciais ou configuração nunca deve ativar integração. Hoje os helpers PG/Neo4j podem conectar, migrar ou gravar; o teste Binance pode submeter compra Spot Testnet quando encontra credenciais; e persist=true no backtest pode chegar ao store de runtime.

Objetivo: tanto cargo test direto quanto o wrapper padrão deixam PG, Neo4j, Binance e HTTP de providers desligados por default. O código exige gates específicos antes de resolver config, abrir conexão ou construir client. O wrapper adiciona uma barreira de rede e auditoria de syscalls para detectar chamadas diretas ou dependências que contornem os seams conhecidos. Cargo direto depende dos gates dos seams e não substitui prova syscall-level.

## Garantias e limites: Cargo direto e wrapper

1. **Cargo direto:** integrações PG, Neo4j e Binance só podem iniciar após o gate individual exato estar habilitado. Configuração e credenciais não são gates. Testes de provider usam mocks locais e não chamam endpoints reais. Sem gate, o helper retorna skip antes de carregar .env/resolver URL/conectar; o teste Binance não constrói client. O critério testável é contador zero por seam mesmo quando o processo contém variáveis sentinela de credenciais.
2. **Wrapper default:** todo processo de teste/descendente roda em Docker --network=none, sem montagem do checkout ou sockets do host, com seccomp fail-closed, strace -f e auditoria fail-closed. Só esta via demonstra que uma chamada arbitrária de rede não consegue egress. Mocks dentro do mesmo namespace podem usar loopback.
3. A imagem local é pinada por digest/ID, a versão strace é fixa, cache Cargo offline é read-only e target é temporário gravável. Não há pull/build implícito ou fallback para rede. Wrapper verifica a arquitetura suportada e aborta antes da imagem quando divergir.

## Snapshot, segredos e execução

O wrapper monta um snapshot temporário filtrado e allowlisted; não monta checkout nem arquivo .env real, .env.*, config Cargo local, .git, logs, targets ou credenciais. Testes que precisam exercitar configuração recebem apenas sentinelas não credenciais criadas para aquele processo, sem herdar o ambiente externo. O snapshot inclui fontes rastreadas necessárias e os scripts/testes de isolamento allowlisted, inclusive artefatos locais ainda não rastreados que façam parte do harness.

Cargo roda com CARGO_NET_OFFLINE=true e --locked --offline; cache previamente provisionado entra read-only em /cargo/registry e CARGO_HOME aponta para /cargo; target/audit são temporários por execução. Ausência de imagem pinada, strace esperado, perfil, arquitetura ou cache aborta antes de executar Cargo.

O container roda com --network=none, --cap-drop=ALL, somente SYS_PTRACE, no-new-privileges e usuário sem privilégio. Não recebe host network, modo privilegiado, sockets Unix, socket Docker/container runtime nem credenciais de metadata. Seccomp usa ação default deny e nega io_uring_setup, io_uring_register e io_uring_enter com EPERM; os três syscalls são listados explicitamente no filtro strace e parser. Plataforma sem suporte falha fechado.

O auditor reconhece chamadas de rede, endereços IPv4/IPv6/Unix, conexões estabelecidas e envios sem sockaddr, DNS em qualquer destino inclusive loopback e as três chamadas io_uring. Argumento malformado/truncado, família desconhecida, destino externo/DNS ou socket Unix do host é violação; não se infere sucesso por o processo ter tratado o erro. Loopback é permitido apenas para mocks do namespace. O self-test adversarial deve mostrar mock local permitido e tentativas de DNS, UDP, TCP, redirect e io_uring observadas/bloqueadas.

Observer de seams complementa a auditoria dinâmica para atribuir pg_connect, pg_migrate, neo4j_connect, neo4j_write, provider_http (inclusive System One/JEV/NIM) e binance_submit. A prova de egress é strace + network-none, não o inventário estático de clientes. Estado default: zero tentativas PG/Neo4j/provider/Binance e zero DNS/egress.

## PostgreSQL: execução local e CI são entregas distintas

W0-02 continua draft/G1 pendente. Ele é dependência para o contrato do connector de teste compartilhado e para o workflow PG da CI, mas sua revisão não bloqueia por si só o desenho do runner local descartável. T-W0-06 não altera W0-02.

### Runner local descartável

O digest local candidato registrado em W0-02 é timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840. W0-02 anota inspeção local do container e PostgreSQL 18.6, Timescale 2.30.1 e vector 0.8.6. Isso é evidência para o alvo Docker local, não prova de GitHub Actions.

Para cada execução, runner cria container novo com UUID aleatório, database trading_bot, volume efêmero, credencial aleatória e porta loopback exclusiva. Antes de disponibilizar URL ao binário de teste, verifica imagem digest/ID, ID do container, UUID/label, porta e database usando Docker local. Depois cria ou verifica bot_test_database_marker fora das migrations, vinculado ao UUID/container/digest/database, e publica manifest de execução protegido. Falha, divergência ou cleanup incerto invalida o target; não há reuso.

O helper requer BOT_RUN_PG_INTEGRATION=1 e BOT_PG_TEST_DATABASE_URL; rejeita qualquer DATABASE_URL presente como fallback, inclusive quando a URL dedicada existe. Comparação do host/porta/database com manifest e identidade do container, além da presença/valor do marker, ocorre antes de chamar connector de teste ou migration. Gate ausente significa skip antes de resolver URL; gate ligado sem target/marker comprovado falha antes da conexão de teste. O nome é trading_bot e o marker bot_test_database_marker, conforme W0-02; não introduzir outro nome.

O connector compartilhado continua bloqueado até revisão de W0-02 resolver seus findings de G1. Esse connector deve preservar name/version/extensions checks compartilhados, recusar no runtime qualquer target marcado e exigir marker no test path; não afrouxar PostgresDatabase::connect_from_url. Para runtime, tabela marker ausente significa banco não marcado e permite seguir após checks normais; SQLSTATE de tabela ausente é tratado como ausência, enquanto erros de permissão/conexão/query falham fechados. Uma tabela marker presente com linha válida identifica DB de teste e runtime recusa; runner local cria e exige a tabela antes de fornecer o target ao teste.

### Workflow PostgreSQL do GitHub Actions

C4 de W0-02 é apenas gate do job/service container CI, não proibição da execução local efêmera. Não alterar nem aceitar o workflow PG enquanto C4 não tiver evidência real de GitHub Actions para o digest exato: início do service, health check, server_version_num >= 180000, extensões timescaledb/vector e jobs relevantes verdes, com link/log seguro. Evidência do Docker local não fecha C4. Se essa execução ainda não existe, documentar C4 como residual bloqueante da CI e não afirmar aprovação de W0-02. Compatibilidade local do digest citado não substitui revisão do contrato compartilhado.

## Neo4j: leitura e escrita com gates independentes

Gates: BOT_RUN_NEO4J_READ_INTEGRATION=1 e BOT_RUN_NEO4J_WRITE_INTEGRATION=1. Ausente ou 0 deixa o caminho desativado antes de carregar configuração, resolver URI ou construir driver; .env, BOT_GRAPH_ENABLED, BOT_AGENTS_ENABLED ou credenciais não contam como opt-in.

Cada runner cria instância Neo4j vazia descartável com run UUID, ID do container, digest, database e marker de modo. Read exige prova de role/database realmente read-only antes do driver; se não verificável, falha fechado. Teste que grava dados durante preparação é write mesmo que faça query depois. Write requer gate próprio e target novo marcado mode=write. Um gate nunca habilita o outro, e target de runtime nunca é reutilizado.

## Binance Spot Testnet

BOT_RUN_BINANCE_TESTNET_ORDER=1 é obrigatório e independente das credenciais. Somente runner dedicado aceita a seleção exata do teste canônico integration_submits_minimal_market_buy_on_testnet, com --exact e uma thread; rejeita args, filtro ou teste diferente antes de iniciar o binário. Gate é verificado antes de ler credenciais ou construir client. Base é Spot Testnet fixa; não existe opção live/prod. Sem gate, credenciais presentes ainda produzem zero cliente/transporte/submit. A autorização existente vale apenas para essa execução dedicada.

## Backtest: store do serve e store de teste

persist=false é cálculo público. persist=true requer admin Bearer válido antes de resolver store ou URL.

- **Runtime serve:** autenticação válida precede store/config; então persistência usa o store normal de runtime configurado para o serviço. Sem token configurado retorna 503; Bearer ausente/incorreto retorna 401 sem resolver PG.
- **Teste default/unitário:** persist=false calcula com fixture válida e zero store; respostas 503/401 provam que store/gateway não foram chamados. Não herda nem usa store de runtime.
- **Teste autorizado de persistência:** runner PG injeta store que referencia exclusivamente container/manifest efêmero já validado. Nunca chama postgres_for_cli_persist nem usa DATABASE_URL. Sem target e marker válidos, nenhum store é criado.

| Caso | Resultado | Banco |
|---|---|---|
| persist=false | cálculo público | nenhum |
| persist=true sem token admin | 503 admin_auth_not_configured | nenhum |
| persist=true com Bearer inválido | 401 unauthorized | nenhum |
| persist=true com Bearer válido em teste | escrita demonstrável | somente PG efêmero |
| persist=true com Bearer válido no serve | persistência normal do runtime após auth | banco do serviço |

## Alternativas consideradas

- Mandar operador limpar .env antes de Cargo: rejeitado, pois é frágil e mistura ambiente runtime/teste.
- Usar somente wrapper: rejeitado, pois teste direto permaneceria inseguro.
- Usar somente gates: insuficiente para provar ausência de chamadas diretas a socket/dependência; wrapper também audita/isola.
- Reusar banco Neo4j/PG do runtime ou escolher outro nome de banco para contornar guard: rejeitado; targets vêm de containers recém-criados e marcador.
- Tratar W0-02 local como aprovação de CI: rejeitado; service container GitHub exige sua evidência própria.
- Reusar store de runtime no teste persist=true: rejeitado para não confundir autorização de operação do serve com alvo de teste.

## Validação observável

Após aprovação G1, aplicar TDD por seam; não declarar resultado antes da execução e guardar comandos/saídas sem segredo.

1. Cargo default direto com configuração/credenciais sentinela: zero contadores de PG connect/migrate, Neo4j connect/read/write, provider HTTP externo e Binance client/submit. Wrapper default soma a prova network-none/strace; mocks loopback permitidos; DNS/UDP/TCP/redirect e malformed trace reprovam.
2. PG gate ausente: skip antes de URL/connection; DATABASE_URL sozinho ou em conjunto com URL dedicada: erro antes do connector; gate ativo sem container identity/manifest/marker: falha antes do test driver/migration; marker correto permite somente connector test-only efêmero. Runtime DB sem tabela marker prossegue após checks normais; runtime DB marcado falha.
3. Neo4j sem cada gate: contador de conexão zero; read sem enforcement read-only: falha pré-driver; write exige target novo com marker de escrita.
4. Binance com credenciais e sem gate: zero client/submit; seleção parcial/arg extra/gate ausente: falha antes de client; ordem real só por runner exato Spot Testnet já autorizado.
5. Backtest: persist=false, 503 e 401 sem store; store runtime do serve só após auth; Bearer válido no teste grava exclusivamente no alvo PG efêmero.
6. Self-test sandbox: trace inclui as três syscalls io_uring com EPERM, todas as tentativas externas/DNS aparecem, loopback de mock é permitido e parser fail-closed rejeita sockaddr incompleto.
7. C4 CI: não mexer nem aceitar o job PG enquanto a execução GitHub real não provar service/digest/health/version/extensões. Essa dependência não impede revisão do runner local e não significa que W0-02 foi aprovado.

## Rollout, rollback, riscos

Sequência: aprovar este SDD G1; implementar código de gates; re-review isolado de T-W0-06a; aprovar runner local e target identity; resolver contrato do connector compartilhado com revisão W0-02; executar integrações somente nos containers efêmeros; tratar o job PG CI por C4 separado. Não iniciar serve, gerar token operacional ou executar ordem durante G1.

Rollback remove juntos wrapper/gates/runners dependentes e mantém integrações desativadas por default. Container sem digest/UUID/marker ou cleanup ambíguo nunca é reutilizado. Se arquitetura, seccomp, strace, audit ou cache offline não tiver prova, falha fechado. C4 continua residual bloqueante do workflow CI; não pode ser substituído por resultado local.

## Resolução dos findings G1

1. Cargo direto e wrapper têm garantias distintas e complementares: os gates do código protegem chamadas conhecidas no Cargo direto; o wrapper prova isolamento de egress de todo o processo. Ambos mantêm integrações desligadas por default.
2. Snapshot nunca inclui .env real ou config local. Qualquer teste de configuração recebe apenas sentinelas não credenciais.
3. T-W0-06 local e W0-02 CI ficam separados. Runner local usa digest/identity/marker e alvo efêmero comprovados localmente; connector compartilhado aguarda revisão do contrato W0-02. Workflow CI fica bloqueado por C4 até evidência real de GitHub Actions. W0-02 mantém draft/G1 pendente.
4. Store do serve e store de teste persist=true são explicitamente diferentes: runtime após auth usa config/store do serviço; testes só injetam store do target PG efêmero.

## Estado da revisão

- Autorização do owner: execução local com .env/Docker, sem escrita no DB persistente; backtest runtime sob auth; apenas Spot Testnet dedicado.
- T-W0-06: PROPOSED — aguardando G1 independente. Não aprovado para implementação por este documento.
- W0-02: status draft/G1 pendente. Findings sobre connector compartilhado continuam dependência; C4 continua gate da CI.
- Evidências locais registradas em W0-02 não afirmam inicialização/compatibilidade de GitHub Actions; nenhuma execução CI foi alegada aqui.
