---
title: SDD T-W0-06 — Isolar efeitos externos da suíte de testes
description: Contrato para tornar cargo test inofensivo por padrão e exigir opt-in, alvo efêmero e identidade verificada para cada integração com efeitos.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: draft
---
# SDD T-W0-06 — Isolar efeitos externos da suíte de testes

**Status:** PROPOSED — G1 pendente. A semântica do backtest e o uso local de `.env` foram acordados pelo owner. Os nomes dos gates, runner, identidade dos alvos e esta revisão técnica ainda aguardam aprovação explícita do owner e do Critic independente. Este documento não autoriza implementação nem execução de integrações.

## Contexto e objetivo

A suíte padrão pode herdar credenciais do shell e carregar `.env`; isso permanece permitido para o runtime local. Credenciais presentes não autorizam conexões, migrações, gravações ou ordens durante testes. Hoje `database_for_integration_test()` conecta a partir de configuração de ambiente e migra; testes Neo4j podem conectar/mutar conforme configuração do stack; o teste Spot Testnet pode enviar compra Market de BTC/USDT; e `execute_backtest` persiste quando `persist=true`.

**Objetivo verificável:** `cargo test` não inicia conexão real PG/Neo4j, migration, escrita externa nem chamada Binance, mesmo com `.env` carregado e credenciais válidas. Testes de integração só passam por runners separados que verificam opt-in, identidade e marcador do alvo antes do primeiro efeito.

**Não objetivos:** remover `.env` do fluxo operacional; alterar comportamento funcional de runtime fora da auth condicional do backtest; executar ordem, conexão ou migração durante implementação deste desenho.

## Seams candidatos para aprovação

Os nomes abaixo são propostas, não contratos aprovados. Os gates têm default literal `0`; carregar dotenv não pode mudá-los para `1`. Configuração da aplicação (`DATABASE_URL`, configuração Neo4j e credenciais Binance) não é configuração de teste nem opt-in.

### Suite padrão e observador de tentativas

- Criar `backend/scripts/verify-test-isolation.sh` como wrapper do comando padrão de testes. Ele define explicitamente todos os gates de integração como `0`, preserva `.env` e executa a suite sem filtro amplo de integração externa habilitado.
- Cada ponto de saída de integração passa por um seam único de gate/observer. Antes de qualquer tentativa, registra no observer um evento `io_start(effect, target_id, test_name)`; eventos incluem `pg_connect`, `pg_migrate`, `neo4j_connect`, `neo4j_write` e `binance_submit`. Denegação do gate não emite `io_start`. O observer padrão é local, injetável e acumulador thread-safe; o wrapper falha se qualquer evento ocorrer.
- No gate padrão, PG e Neo4j usam endpoints-sentinela loopback controlados pelo runner em portas efêmeras; qualquer tentativa TCP é contada e causa falha. Binance usa adapter/client injetável de teste cujo submit conta tentativa e cujo modo não tem transporte real. Além da contagem dinâmica, um guard CI verifica que construtores de cliente e chamadas de persistência/submissão passam pelos seams; bypass conhecido falha o guard. HTTP mocks internos de testes unitários continuam permitidos e não são destinos externos.
- O observador prova tentativa, inclusive conexão que falha antes de gravar. Métricas esperadas do wrapper default: `pg_connect=0`, `pg_migrate=0`, `neo4j_connect=0`, `neo4j_write=0`, `binance_submit=0`; qualquer valor diferente de zero falha o comando.

### PostgreSQL: runner, identidade efêmera e marcador pré-migration

Variáveis propostas: `BOT_RUN_PG_INTEGRATION=1`, `BOT_PG_TEST_DATABASE_URL`, `BOT_PG_TEST_RUN_ID` e manifest restrito do runner `backend/.test-targets/pg.json`. Nunca selecionar alvo de teste usando `DATABASE_URL`.

1. O runner cria um container PostgreSQL novo por execução, com imagem/versionamento fixos pela configuração aprovada, nome/label que contenha UUID aleatório do run, senha aleatória, database `trading_bot_test`, volume efêmero e porta loopback aleatória. Não reutiliza container/volume/user DB. Em CI, o serviço é igualmente exclusivo do job/run e recebe identidade randômica.
2. O runner verifica por Docker API o container ID, image digest, label/UUID, estado, porta mapeada e nome do database. Só então cria a tabela de controle `test_run_marker` fora do código de teste, com UUID, container ID, digest, database, tipo `postgres` e timestamp; grava o manifest com permissões restritas e URL sem log.
3. Antes de conectar, o helper exige flag exatamente `1`, URL de teste dedicada e manifest válido. Confere formato/allowlist loopback e porta com manifest; o runner revalida que container ativo e ID/digest/label/porta correspondem. Endpoint ou identidade divergente falha antes da conexão.
4. Após conexão, o helper consulta somente o marker e confere integralmente run ID, container ID, digest, database e tipo. Ausente/incorreto implica falha e nenhuma migration. Só marker válido permite `migrate()`; observer registra conexão e migration.
5. Flag ausente significa skip sem conexão. Flag presente com URL/manifest ausente ou inconsistente falha fechado, sem exibir URL, senha ou token. A URL de teste não pode apontar a porta/endereço do DB de aplicação; rejeitar `DATABASE_URL` como fallback.

### Neo4j: gates separados para leitura e escrita

Variáveis propostas: `BOT_RUN_NEO4J_READ_INTEGRATION=1`, `BOT_RUN_NEO4J_WRITE_INTEGRATION=1`, `BOT_NEO4J_TEST_URI`, `BOT_NEO4J_TEST_DATABASE`, `BOT_NEO4J_TEST_RUN_ID` e `backend/.test-targets/neo4j.json`.

- Runner cria novo container Neo4j por run, instância/database vazios e descartáveis, credencial aleatória e porta loopback exclusiva; grava marker do run com UUID, container ID, image digest, database e modo permitido. Verifica esses atributos via container runtime e Neo4j antes de entregar o manifest.
- Toda leitura de integração requer o gate de leitura exatamente `1`, URI/database dedicados e marker cuja identidade coincida com o container ativo. Configuração normal `BOT_GRAPH_ENABLED`/credenciais não habilita testes. Um teste de leitura não recebe credencial/grant de escrita quando Neo4j suporta grant por database.
- Toda escrita/projeção requer adicionalmente gate de escrita exatamente `1`, manifest `mode=write`, marker válido e container criado como descartável neste run. O helper valida o gate e identidade antes do driver conectar; observer diferencia connect de write. Não inferir permissão de escrita do gate read.
- O inventário classificado no plano de implementação deverá mapear os testes em `core/database/graph_query.rs` e `core/database/neo4j.rs` como leitura; `modules/agents/adapters/graph_projection.rs`, `modules/bots/adapters/graph_projection.rs` e `modules/orders/adapters/graph_projection.rs` como escrita/projeção. Qualquer operação ambígua vai para gate write até revisão.
- Ausência de gate = skip sem connect. Gate habilitado com manifest ausente, incompatível, marker inválido ou modo insuficiente = falha fechada antes do driver.

### Binance Spot Testnet: runner com filtro efetivamente exato

Gates/runner propostos: `BOT_RUN_BINANCE_TESTNET_ORDER=1` e `backend/scripts/run-binance-testnet-order.sh`. Teste canônico: `modules::exchanges::adapters::binance_spot_testnet_submit::tests::integration_submits_minimal_market_buy_on_testnet`.

- O script aceita somente argumento vazio/nenhum do usuário; monta por si o comando cargo com package/bin de teste identificados, nome completo canônico e `-- --exact --test-threads=1`. Rejeita argumentos extras ou filtros alternativos. Não repassa argumentos livres ao harness.
- Antes de permitir o processo de teste, o runner resolve e compara nome de teste/bin esperados e verifica que o comando que será executado contém literalmente `--exact`, nome canônico e threads=1. O teste repete a verificação de `args_os()` do harness, exige gate igual a `1` antes de ler credenciais/construir cliente e exige apenas chaves Spot Testnet dedicadas.
- Com gate desligado ou seleção diferente do teste canônico, a execução termina antes de criar cliente; adapter observer conta `binance_submit=0`. Gate ligado exige ambas credenciais e base URL fixada em Spot Testnet; configuração não pode escolher ambiente de produção.
- Só runner com gate, filtro exato, credentials testnet e suite isolada pode chamar a testnet. A execução pode submeter a ordem autorizada pelo owner; por isso não faz parte do wrapper padrão.

## Backtest: matriz auth antes da persistência

Owner aprovou o seam funcional: cálculo com `persist=false` continua público; `persist=true` exige admin Bearer válido. O handler atual precisa passar estado de auth e porta de persistência explicitamente; a decisão de auth deve preceder execução/persistência. O caminho CLI pode manter sua semântica operacional, mas o handler HTTP não pode chamar persistência por variável de ambiente antes de autorizar.

| Request | Resultado público esperado | Acesso PG | Ambiente de teste |
|---|---|---|---|
| `persist=false`, sem bearer, mesmo sem PG | sucesso do cálculo conforme schema vigente | nenhum connect/persist; observer zero | default suite |
| `persist=true`, auth admin ausente/fraca | HTTP 503, código `admin_auth_not_configured` | nenhum | teste unitário do handler |
| `persist=true`, token configurado, Bearer ausente ou errado | HTTP 401, código `unauthorized` | nenhum | teste unitário do handler |
| `persist=true`, Bearer válido | sucesso de persistência | somente por porta/store injetada ligada ao PG ephemeral com marker válido | runner PG opt-in |
| rota diferente/método não permitido | resposta existente de 404/405 | nenhum | teste de roteamento |

A matriz testa payload válido e sucesso observável no caso de cálculo, não apenas classificação do middleware. Testes 503/401 afirmam contadores de store/connect/migrate em zero. Teste autorizado usa apenas o alvo PostgreSQL descartável validado pelo helper e afirma write observável. Se a persistência estiver indisponível, falha explícita; não recua para banco de aplicação nem ignora `persist=true`.

## Matriz de validação exigida após G1

1. **Default com dotenv:** rodar wrapper padrão com `.env` e variáveis de credencial/sentinela disponíveis, gates forçados em zero; suite passa e observer/contadores mostram todos zero. A evidência é tentativa observada, não ausência de gravação.
2. **PG:** flag ausente com `DATABASE_URL` presente resulta em skip e zero conexão; gate habilitado sem manifest/URL dedicada falha antes de connect; alvo que não está no allowlist/UUID diverge falha antes de connect; marker ausente/incompatível falha antes de migrate; alvo efêmero correto executa migration somente depois da validação marker.
3. **Neo4j:** gate ausente não conecta; read gate com marker read válido permite apenas testes de leitura; tentativa de escrita sem write gate/mode é negada antes de escrever; write gate só permite o container ephemeral do run. Contadores e marker demonstram identidade.
4. **Binance:** credenciais presentes sem gate não constroem cliente; gate com filtro ausente/extra/alternativo não abre processo com transporte e contagem de submit permanece zero; gate + filtro exato + credenciais testnet permite apenas o teste canônico.
5. **Backtest:** executar a matriz acima com observer no handler/store e resposta HTTP concreta. Confirmar persist=false nunca instancia PG; 503/401 ocorrem antes do store; bearer válido só grava no PG disposable identificado.
6. Executar casos RED/GREEN por seam público e análise estática do guard de construtores. Integração real PG/Neo4j só roda após runner criar/alistar target descartável; ordem Testnet é um comando separado e não faz parte de validação padrão.

## Rollout, rollback, riscos e alternativas

Rollout após G1: primeiro criar gates e helpers que falham fechados; migrar cada teste externo para o helper; introduzir runners de container/marker; então habilitar somente o job PG descartável necessário. Wrapper default mantém gates em zero. Neo4j read/write e Binance ficam desativados fora dos comandos dedicados. Rollback reverte helper/runner e os testes dependentes juntos; nunca remover gate sem restaurar proteção equivalente.

Riscos: Docker API e Neo4j marker podem não provar todos os modos de isolamento disponíveis em todos ambientes; o runner deve suportar apenas versões/ambientes declarados e falhar fechado. Teste com acesso direto a driver ou store pode contornar observer; guard de chamadas e revisão por arquivo precisam localizar bypasses. Estado/custo operacional do container é limitado por cleanup em trap e label de run; falha de cleanup não reaproveita container/volume. Secrets nunca aparecem no log, manifesto legível por grupo inadequado, mensagem de panic ou artefato CI.

Alternativas rejeitadas: limpar `.env` manualmente é frágil e contraria o uso local aprovado; usar só marcador sem identidade de container permite marcador falso em DB persistente; somente ignorar teste Binance não impede PG/Neo4j; usar `DATABASE_URL` para integração mistura alvo runtime e alvo descartável; checar só a resposta final não detecta conexão sem escrita.

## Perguntas e estado de aprovação

- **Acordado pelo owner:** `.env` pode ser usado localmente; credenciais não autorizam efeitos colaterais por si; backtest `persist=false` público/cálculo e `persist=true` protegido por Bearer admin.
- **Aguardando resposta explícita do owner:** aprovação dos nomes/seams candidatos `BOT_RUN_PG_INTEGRATION`, URL/manifest PG, gates separados Neo4j read/write, `BOT_RUN_BINANCE_TESTNET_ORDER`, script que impõe o filtro completo `--exact`, observer default e matriz/porta do backtest. Aprovação de usar containers efêmeros e markers verificados antes de migration/connect também precisa ser inequívoca.
- **Aguardando Critic independente:** revisão desta versão depois da resposta do owner. G1 permanece pendente; G3/implementação e execução de qualquer integração estão bloqueadas até essas aprovações.
- **Não executado:** nenhuma mudança de código/configuração, teste, processo de integração, chamada de rede, conexão/migration, nem ordem.

## Critério de aceite

G1: owner aprova os seams propostos ou substituições explícitas; Critic independente aprova design com os blockers resolvidos. G3: teste RED/GREEN de gates default e helper, rota/backtest e runners. G4: wrapper default observavelmente registra zero tentativas; testes PG/Neo4j demonstram identidade + marker anterior a migration/ação; runner Binance rejeita filtro não exato antes de qualquer cliente/rede; matriz backtest produz respostas e contadores esperados. Nenhum aceite se baseia somente em credenciais ausentes.
