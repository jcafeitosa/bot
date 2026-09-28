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

**Status: PROPOSED — T-W0-06 G1 foi reprovado; a implementação permanece congelada.** Esta revisão incorpora os findings do Critic e exige nova aprovação G1 independente antes de qualquer implementação. O owner autorizou execução local com .env e Docker, alvos de integração descartáveis, preservação do banco persistente do app, backtest persistente sob auth e somente a ordem Spot Testnet dedicada. Não autorizou live trading nem testes no banco persistente. W0-02 continua draft/G1 pendente e é uma dependência bloqueante do GREEN PG; este SDD não aprova sua revisão.

## Contexto e objetivo

Cargo test pode carregar configuração e credenciais locais. Credenciais, DATABASE_URL ou BOT_GRAPH_ENABLED nunca podem funcionar como opt-in. Integrações PG, Neo4j, providers e Binance precisam de gates explícitos antes de resolver configuração, abrir conexão ou construir client.

Objetivo: Cargo test direto mantém efeitos externos desativados por gates nos seams conhecidos; o wrapper dedicado acrescenta isolamento de rede e prova syscall-level de que processos e descendentes não fazem egress. Cargo direto não substitui essa auditoria dinâmica; chamadas diretas a socket são comprovadamente bloqueadas somente no wrapper.

## Cargo direto versus wrapper

- **Cargo direto:** gates PG/Neo4j/Binance desligados por padrão; testes de provider usam mocks locais. Presença de credenciais ou .env não ativa testes de integração. Critério é zero chamadas a connector, migration, Neo4j driver, provider HTTP ou Binance client/submit sem o respectivo gate.
- **Wrapper default:** executa a suite completa dentro de Docker --network=none, com seccomp default deny, strace -f e auditor fail-closed. Não monta checkout, .env real, config Cargo local, .git, logs, target ou socket Unix do host. Sem montagem de Docker/container-runtime socket, host network ou modo privilegiado.
- O snapshot é filtrado por allowlist e inclui somente arquivos regulares aprovados. O criador rejeita qualquer symlink antes de ler seu conteúdo, resolve cada caminho de origem/destino e falha fechado se o caminho canônico não permanecer dentro da raiz allowlisted. Isso vale para arquivos rastreados e novos. Fixture adversarial aponta symlink para um arquivo sensível fictício fora da raiz e prova que não é seguido/copied. Config de fixture usa apenas sentinelas não credenciais; não herda ambiente externo nem `.env` real.
- Cargo usa CARGO_NET_OFFLINE=true, --locked --offline, registry/cache previamente provisionado read-only em /cargo/registry e CARGO_HOME=/cargo; target/audit temporários por execução. Imagem é pinada por digest/ID e strace por versão. Sem imagem, cache, arquitetura ou profile compatível, aborta antes de Cargo; sem build/pull/fallback implícito.
- Docker usa --network=none, --cap-drop=ALL e somente SYS_PTRACE, no-new-privileges e usuário sem privilégio. Profile seccomp restritivo nega io_uring_setup/register/enter com EPERM; os três syscalls são explicitamente incluídos no filtro strace e auditor. Wrapper valida arquitetura/profile antes de iniciar imagem e falha fechado em host incompatível.
- Auditor permite somente mocks loopback dentro do namespace. DNS na porta 53 é proibido também em loopback; UDP/TCP externo, redirect, syscall de rede malformada ou endereço não interpretável falha mesmo se aplicação ignorou o erro. Socket Unix do host não é montado. O self-test adversarial deve provar mocks locais permitidos, DNS/UDP/TCP/redirect bloqueados e as três chamadas io_uring observadas com EPERM.
- Observer classifica pg_connect, pg_migrate, neo4j_connect/read/write, provider_http (System One/JEV/NIM e outros clients externos) e binance_submit. Ele complementa strace; não substitui a auditoria de todos os processos/descendentes. Default: zero tentativas de DB/provider/exchange e zero DNS/egress.

## PostgreSQL: runner local efêmero separado da CI (dependência W0-02)

W0-02 continua draft/G1 pendente. Sua revisão identifica questões no connector test-only compartilhado e no CI; T-W0-06 não edita W0-02 nem declara sua aprovação. A revisão G3 atual encontrou falso-green quando falha de conexão/migration pode ser convertida em skip/retorno normal. A aprovação independente de W0-02 é uma dependência bloqueante para implementar ou declarar GREEN o caminho PG de T-W0-06. O protocolo de conclusão do helper descrito em [W0-02](./wave0-02-ci-pg-fail-loud-sdd.md) integra o critério de GREEN nesta proposta.

**Alvo local:** cada execução elegível usa somente container PG vazio, efêmero, criado pelo runner dedicado, com UUID/run-id, container ID, digest de imagem, credencial aleatória, porta loopback exclusiva e volume descartável. O candidato local registrado em W0-02 é `timescale/timescaledb-ha@sha256:131bfdf82ec0dfe42eaa3f4a189f8e04b7b1dc2b27705cfd921e55ebef339840`; essa evidência descreve um alvo local e não prova o comportamento de GitHub Actions.

Antes de iniciar o binário, o runner verifica daemon local, imagem/digest, container ID completo, labels/run-id, porta, database e mounts contra um manifest privado da execução. Cria e valida `bot_test_database_marker` antes de liberar `BOT_PG_TEST_DATABASE_URL` ao teste; o marker vincula run-id, container ID, digest e nome do database. Marker ausente, ambíguo ou divergente invalida o alvo antes de conectar ou migrar. O runner rejeita IDs de container/volume pertencentes ao stack persistente do app, mesmo que a URL use o nome esperado. O nome `trading_bot`, se mantido por compatibilidade com W0-02, só pode existir dentro desse container efêmero e nunca identifica, seleciona ou autoriza o banco persistente da aplicação. Nome de database isolado não é prova de identidade.

**Gates e resultado:** testes PostgreSQL são marcados `#[ignore]` e o Cargo default os apresenta como ignored. Somente runner dedicado valida allowlist do nome completo e argumentos `--ignored --exact`, então invoca um teste por processo com `BOT_RUN_PG_INTEGRATION=1` e `BOT_PG_INTEGRATION_REQUIRED=1`. Credenciais, `.env`, `DATABASE_URL` e nome `trading_bot` não são opt-in nem seletor de destino. `BOT_PG_TEST_DATABASE_URL` é obrigatória; `DATABASE_URL` herdada presente é erro, sem fallback. O helper recebe prova do alvo/manifest/marker do runner e valida host, porta, database e identidade antes do connector/migration. Sem opt-in, não resolve URL e permanece ignored. Depois do opt-in, alvo ausente/inválido, identidade/marker incorreto, conexão, migration ou query com falha produz erro explícito e código de saída não zero, nunca `None`, skip ou sucesso silencioso.

**Conclusão exigida pelo runner:** cada teste do manifesto chama o helper uma vez e, após executar todas as operações e assertions PG, chama `pg_integration_assertions_complete!("<nome_qualificado>")` como última statement. O helper emite `PG_INTEGRATION_HELPER_OK`; o marcador final emite `PG_INTEGRATION_ASSERTIONS_OK:<nome_qualificado>`. O runner só considera GREEN quando há exit zero, exatamente um teste passado, zero falhas/ignorados, exatamente uma linha de sucesso do helper e exatamente uma linha de conclusão com o mesmo nome qualificado. Falha de helper, marcador ausente/repetido, retorno antes do marcador ou divergência de nome é fail. Helper success prova conexão/migração; completion prova que o teste alcançou seu fim após assertions, não a qualidade semântica delas. Este protocolo depende de W0-02 aprovado independentemente.

O connector de teste compartilhado continua bloqueado até aprovação G1 independente de W0-02; nenhuma implementação ou GREEN PG precede essa aprovação. Runtime não pode selecionar alvo de teste por `DATABASE_URL` nem operar container/volume efêmero do runner; teste nunca herda o banco persistente operacional. A tabela marker ausente em banco runtime só significa “sem marker” quando a consulta retorna especificamente relation/table absent (SQLSTATE definido pelo connector); erro de conexão, permissão ou consulta falha fechado. Marker presente e válido identifica alvo de teste, que runtime recusa. Guard de nome, URL, versão, extensões e protocolo helper/completion permanecem dependências de W0-02.

**CI PostgreSQL:** C4 bloqueia apenas alterar/aceitar service container/job PG do GitHub, não todo uso local. Antes de tocar no workflow, evidência real de execução GitHub precisa comprovar digest exato, inicialização do service, health, server_version_num >= 180000, extensions timescaledb e vector e jobs relevantes verdes, com link/log seguro. Se essa prova ainda não existe, C4 permanece residual CI bloqueante. Resultado local do digest não prova GitHub service. W0-02 permanece draft/G1 pending e nenhum status de aprovação é inferido.

## Neo4j: leitura e escrita em gates, alvos e roles separados

`BOT_RUN_NEO4J_READ_INTEGRATION=1` habilita somente testes classificados como leitura; `BOT_RUN_NEO4J_WRITE_INTEGRATION=1` habilita somente testes que escrevem. Ambos os grupos são `#[ignore]` no Cargo default; somente runner dedicado valida a lista de testes exatos e executa um teste por processo com `--ignored --exact` e um dos gates. Ausente, `0` ou outro valor mantém o teste ignored, sem ler configuração, resolver credenciais, construir driver ou conectar. Os dois gates são independentes: habilitar leitura não habilita escrita e vice-versa. `.env`, `BOT_GRAPH_ENABLED`, `BOT_AGENTS_ENABLED` e credenciais nunca provam opt-in nem identidade do alvo.

Cada execução elegível cria um container Neo4j efêmero e vazio, com run-id, container ID, digest, database isolado e marker `BotTestTarget` contendo run-id, container ID, digest, database e `mode=read|write`. O runner valida manifest/marker e rejeita IDs do stack persistente/runtime. `.env` não serve como origem alternativa de conexão nem como prova de identidade.

Para READ, o teste carrega uma única configuração immutable `Neo4jTestTarget` do manifest validado: endpoint, database, principal/role read e a referência exata da credencial efêmera. O pré-flight valida marker e read-only usando essa configuração e cria o driver que será entregue ao teste; as assertions usam o mesmo objeto/driver e mesma credencial, sem novo lookup, fallback ou reconexão com configuração de `.env`. Se for obrigatório reconectar, repetir a validação de identidade com os mesmos endpoint, database, principal e segredo antes da primeira query do teste. Divergência, configuração ambígua ou falha de role/marker aborta sem executar assertions.

Qualquer fixture/preparação com escrita é teste write. Write exige seu gate e marker `mode=write`, e só usa o container descartável daquela execução. Gate fechado deixa o teste ignored; depois de opt-in, erro de configuração, conexão, permissão ou query falha o teste e retorna não zero, nunca skip. Targets e credenciais runtime nunca são reutilizados.

## Binance Spot Testnet

`BOT_RUN_BINANCE_TESTNET_ORDER=1` é obrigatório e independente das credenciais. Somente runner dedicado com teste canônico selecionado por nome completo e `--exact` pode chegar à construção do client. Args extra, filtro parcial, nome diferente ou execução paralela de testes Binance falham antes do client. Um lock global do runner serializa todas as execuções Spot Testnet, inclusive processos separados. Gate é verificado antes de ler credenciais; URL é fixa em Spot Testnet.

O teste dedicado usa `clientOrderId` determinístico associado ao run-id de execução e reutiliza esse ID em toda repetição da mesma execução. Antes de uma repetição, consulta/reconcilia a ordem por esse ID; se já existir/estiver preenchida, não envia outra. Resultado ambíguo faz o runner falhar e interrompe reenvio até reconciliação, nunca gera ID novo automaticamente. GREEN inclui executar o mesmo filtro duas vezes para o mesmo run-id e provar no testnet que há no máximo uma ordem, além de comprovar serialização entre dois runners concorrentes. Sem gate, credenciais presentes produzem zero client/transport/submit. Autorização do owner cobre apenas ordem Spot Testnet dedicada; nenhuma ordem live.

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

1. **Default sem efeitos (RED/GREEN):** com sentinelas de credenciais, `.env`, `DATABASE_URL`, `BOT_GRAPH_ENABLED` e endpoints providers presentes, Cargo default mostra integrações como ignored; observadores confirmam zero resolução de config/URL, connector, migration, Neo4j driver, provider HTTP ou Binance client/submit. Mocks loopback são permitidos; DNS, UDP/TCP externo, redirect, trace malformado e io_uring sem EPERM reprovam.
2. **PG gate fechado (RED/GREEN):** nenhuma URL/connector/migration é consultada e teste está ignored. **Gate `=1` com URL ausente, `DATABASE_URL` isolada/herdada, alvo persistente ou identidade/marker inválido:** execução falha antes de conexão/migration. **Target efêmero válido:** runner exige sucesso do helper e marcador `PG_INTEGRATION_ASSERTIONS_OK` como último passo após assertions; marcador ausente, repetido, nome divergente ou retorno precoce reprova mesmo se libtest disser `1 passed`. Falhas de conexão/migration/query retornam não zero. Runner compara nome completo do teste e exige exactamente uma ocorrência de ambos os marcadores conforme [W0-02](./wave0-02-ci-pg-fail-loud-sdd.md). Testar saída ausente/repetida e retorno antes do completion.
3. **Snapshot (RED/GREEN):** criar em fixture um symlink para arquivo sensível fictício fora da raiz; RED deve provar que snapshot o seguiu/copiou, GREEN rejeita o symlink antes de ler e verifica que todos os caminhos canônicos permanecem dentro da raiz allowlisted.
4. **Neo4j gates fechados (RED/GREEN):** mesmo com `.env`/credenciais, read/write ficam ignored e deixam config/driver/conexão em zero. **Read válido:** marker/manifest, endpoint, database, principal, role e segredo do pré-flight são os mesmos usados pelo driver/teste. Mutar qualquer campo entre preflight e query, apontar `.env` a outro alvo ou usar role com escrita reprova antes de assertions. **Read inválido:** marker ausente/divergente, alvo runtime ou enforcement write falha. **Write:** exige gate separado e target/marker write; errors retornam não zero.
5. **Binance (RED/GREEN):** credenciais sem gate deixam zero lookup/client/transport/submit. Dois runners concorrentes não submetem em paralelo. Repetir filtro para o mesmo run-id consulta/reconcilia o `clientOrderId` existente sem segundo submit; resultado ambíguo bloqueia repetição. Args/nome/filtro inválidos falham antes do client. Submit real só em Spot Testnet dedicado autorizado.
5. **Backtest:** `persist=false`, 503 e 401 mantêm contador de store zero; bearer válido escreve somente com PG efêmero; `serve` persiste no store runtime após auth.
6. **Wrapper:** self-test prova auditoria de descendentes, egress bloqueado, io_uring observado como EPERM e parser fail-closed.
7. **C4 GitHub:** não alterar/aceitar service job até execução real provar init/health/version/extensions. Resultado local não satisfaz este gate CI.

## Rollout, rollback e riscos

Ordem: Critic aprova este desenho G1; W0-02 recebe aprovação G1 independente; só então implementar T-W0-06a e revisar G3. Antes do GREEN PG, implementar helper/completion protocol de W0-02 e runner exigir os dois marcadores depois das assertions. Provar snapshot sem symlink e caminhos confinados; validar Neo4j identidade única preflight→uso; serializar/idempotentizar Testnet em target dedicado. Executar integrações só em alvos efêmeros. CI só depois de evidência C4. Nenhum serviço operacional, token admin ou ordem é iniciado durante G1.

Rollback reverte wrapper/gates/runners dependentes juntos e mantém integrações ignored/desligadas. Container sem digest/UUID/marker, ID persistente do app ou cleanup incerto nunca é reutilizado. Gate inválido, alvo inválido, helper/completion ausente ou falhas pós-opt-in retornam erro; não voltam a ser skips silenciosos. Snapshot que encontra symlink ou caminho fora da raiz falha fechado. Pré-flight e teste Neo4j não podem trocar target/role/segredo. Execução Binance concorrente bloqueia; repetição idempotente nunca faz submit extra. Host/profile/cache sem prova falha fechado. C4 é residual bloqueante apenas para workflow PG CI.

## Resolução dos findings G1/G3 — revisão atual proposta

O G1 anterior foi reprovado; o G3 de implementação continua reprovado. Esta revisão não aceita nem libera código. Novo Critic G1 independente deve aprová-la. A dependência W0-02 também permanece draft/G1 pendente e bloqueia GREEN/implementação PG.

1. Cargo default apresenta integrações como ignored; gates explícitos são necessários para executá-las. Wrapper acrescenta garantia syscall-level contra egress arbitrário.
2. Snapshot exclui `.env` real, rejeita symlink e prova contenção do caminho resolvido na raiz allowlisted com fixture sensível fictícia.
3. PG local usa alvo efêmero `BOT_PG_TEST_DATABASE_URL`, opt-in, identidade e marker; `DATABASE_URL` não seleciona alvo. GREEN requer `PG_INTEGRATION_HELPER_OK` e `PG_INTEGRATION_ASSERTIONS_OK:<nome>` depois das assertions, exigidos exatamente pelo runner; qualquer falha/marcador ausente reprova. W0-02 só sai de pendente com aprovação independente.
4. Neo4j read/write têm gates separados. Pré-flight e teste compartilham a mesma identidade/credencial/driver; `.env` não prova nem troca o alvo. Role read-only é validada.
5. Binance Testnet globalmente serializado e repetição por run-id idempotente/reconciliada; nenhuma live order.
6. Store runtime do `serve` e store de teste persistente continuam separados; runtime somente após auth, testes somente com injeção ligada ao target PG efêmero.

## Estado

- T-W0-06: **PROPOSED — G1 reprovado no ciclo anterior; nova revisão independente pendente**. Código permanece congelado até G1.
- W0-02: **draft/G1 pendente e dependência bloqueante para GREEN/implementação PG**. C4 bloqueia alteração/aceite do job PG CI até prova de GitHub Actions. Review do connector compartilhado continua dependência.
- Escopo autorizado pelo owner: execução local, dados persistentes protegidos, runner PG/Neo4j efêmero, backtest runtime autenticado, ordem dedicada Spot Testnet apenas.
- Nenhuma evidência de CI GitHub foi alegada.
