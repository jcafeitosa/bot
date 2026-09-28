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

**Status:** PROPOSED — G1 pendente. O uso local de `.env` e o comportamento do backtest foram acordados pelo owner. Os nomes dos gates, runners e provas de identidade abaixo continuam candidatos, aguardando aprovação explícita do owner e revisão G1 independente. Não implementar nem executar integrações antes desses gates.

## Contexto e objetivo

A suíte pode herdar credenciais do shell ou carregar `.env`; isso permanece permitido no runtime local. Credenciais presentes nunca são opt-in. Hoje o helper PG conecta pela configuração ambiente e migra; testes Neo4j conectam/escrevem conforme configuração do stack; o teste Spot Testnet pode enviar compra Market BTC/USDT; e `execute_backtest` persiste quando `persist=true`.

**Objetivo mensurável:** `cargo test` padrão não inicia tentativa de conexão PG/Neo4j, migration, escrita externa nem submissão Binance, mesmo com `.env` carregado. Cada integração só prossegue por runner dedicado após verificar gate, alvo descartável e identidade.

## Seams públicos propostos

### Suíte padrão e observação de tentativas

`backend/scripts/verify-test-isolation.sh` define explicitamente os gates de integração como `0`, preserva `.env` e executa a suite usual. Gate ausente equivale a desabilitado, mesmo que dotenv defina configuração/credenciais. Gate só é aceito quando valor é exatamente `1`.

Todos os pontos de I/O passam por seams únicos, com observer injetável e acumulador thread-safe. Imediatamente antes de cada tentativa, registrar `io_start(effect, target_id, test_name)`, onde effect é `pg_connect`, `pg_migrate`, `neo4j_connect`, `neo4j_write` ou `binance_submit`. Denegação do gate não registra tentativa. Wrapper default falha se qualquer contador for diferente de zero.

O wrapper inicia listeners-sentinela loopback em portas efêmeras para os gateways PG e Neo4j e falha se receber SYN/conexão, contando tentativa mesmo que o servidor de teste rejeite. Binance usa client/transport adapter injetável; o default exige transporte não-real e falha se houver submit. Um guard CI localiza construtores de drivers/clients e chamadas de persistência/submissão que não usam o seam; bypass reprova. HTTP mocks internos de unit tests são permitidos e separados de destinos externos.

### PostgreSQL

Variáveis candidatas: `BOT_RUN_PG_INTEGRATION=1`, `BOT_PG_TEST_DATABASE_URL`, `BOT_PG_TEST_RUN_ID`; manifest candidato `backend/.test-targets/pg.json`. Nunca escolher DB de teste via `DATABASE_URL`.

1. Runner cria container PostgreSQL novo por execução: UUID aleatório no nome/label, image digest fixo, senha aleatória, database `trading_bot_test`, volume efêmero e porta loopback aleatória. Sem reaproveitar container, volume ou banco de aplicação.
2. Runner verifica pela Docker API ID do container, digest, label/run UUID, porta mapeada e nome do DB. Só após essa verificação cria marker `test_run_marker` fora do helper/teste, contendo run UUID, container ID, digest, database e tipo `postgres`. Grava manifest restrito, nunca imprime URL/senha.
3. O helper só considera conexão se flag é exatamente `1`, URL dedicada e manifest válido; confirma loopback/porta contra manifest e runner revalida container ativo/ID/digest/label/porta. Divergência falha antes da conexão. `DATABASE_URL` não é fallback.
4. Depois de conectar, mas antes de migration, helper lê marker e confere UUID/container/digest/database/type. Ausente ou incompatível falha sem migration. Só marker correto autoriza migrate. O observer contabiliza connect e migrate.
5. Flag desligada = skip explícito e zero conexão. Flag ligada sem URL/manifest ou identidade inconsistente = falha fechada, sem revelar segredo. Os DBs de teste não podem coincidir com endereço/DB runtime.

### Neo4j

Gates candidatos separados: `BOT_RUN_NEO4J_READ_INTEGRATION=1` e `BOT_RUN_NEO4J_WRITE_INTEGRATION=1`; destino `BOT_NEO4J_TEST_URI`, `BOT_NEO4J_TEST_DATABASE`, `BOT_NEO4J_TEST_RUN_ID`; manifest `backend/.test-targets/neo4j.json`.

Runner cria instância/container novo e descartável por run, credencial randômica, URI loopback/porta exclusiva e marker com UUID, container ID, image digest, database e mode (`read` ou `write`). Verifica identidade no runtime de containers e no Neo4j antes de liberar manifest.

Gate read permite apenas operações de leitura e requer marker/identidade correspondente; sempre usar grant read-only quando suportado. Gate write é independente, requer marker `mode=write` e container criado descartável para esse run. Configuração runtime como `BOT_GRAPH_ENABLED` e credenciais normais nunca ativa integração. Helpers validam gate e marker antes do driver conectar; nenhuma operação ambígua recebe read-only implicitamente.

Inventário inicial: leitura em `core/database/graph_query.rs` e `core/database/neo4j.rs`; projeções que escrevem em `modules/agents/adapters/graph_projection.rs`, `modules/bots/adapters/graph_projection.rs` e `modules/orders/adapters/graph_projection.rs`. Confirmar cada teste durante implementação; default é gate write para classificação ambígua. Gate ausente = skip sem conexão. Gate ativo sem marker/identidade/mode compatíveis = falha antes do driver.

### Binance Spot Testnet

Candidatos: flag `BOT_RUN_BINANCE_TESTNET_ORDER=1`, script `backend/scripts/run-binance-testnet-order.sh`, teste canônico `modules::exchanges::adapters::binance_spot_testnet_submit::tests::integration_submits_minimal_market_buy_on_testnet`.

Script não aceita argumentos livres. Ele próprio monta o comando Cargo com nome completo e `-- --exact --test-threads=1`. Rejeita filtros alternativos, argumentos extras ou modo paralelo antes de invocar o binário. Antes da construção do cliente, o teste verifica `args_os()` do harness para confirmar `--exact`, nome canônico e uma thread; então exige flag exatamente `1` e somente credenciais dedicadas Spot Testnet. Credenciais não são lidas até passar gate/filtro. Base URL é fixada em Spot Testnet; modo produção é impossível nesse runner.

Sem flag ou com filtro/argumentos incompatíveis, sai antes de construir client/transport e observer permanece `binance_submit=0`. Apenas o comando exato e credenciais testnet podem alcançar o submit. Esse runner nunca integra o wrapper padrão; a ordem autorizada pelo owner é uma execução explícita separada.

## Backtest HTTP: auth antes do store

Owner acordou: `POST /api/v1/backtest/sma-crossover` com `persist=false` é cálculo público; com `persist=true` requer admin Bearer válido. O handler deve aplicar auth antes de execução/persistência e receber explicitamente porta/store de persistência. Não pode resolver DB de produção via ambiente antes de autenticar. CLI pode preservar seu fluxo operacional.

| Request | Resposta esperada | Tentativas DB | Teste |
|---|---|---:|---|
| Payload válido, `persist=false`, sem Bearer/PG | sucesso HTTP e corpo de cálculo conforme contrato da rota | 0 | suite default; observer/store contador zero |
| `persist=true`, admin token ausente/fraco | 503, `admin_auth_not_configured` | 0 | unit handler |
| `persist=true`, token configurado, bearer ausente/errado | 401, `unauthorized` | 0 | unit handler |
| `persist=true`, bearer válido | sucesso persistente | conexão/write somente no store PG efêmero com marker válido | runner PG opt-in |
| path/método desconhecido | 404/405 conforme roteamento | 0 | teste roteador |

Testes de 503/401 afirmam que nem store nem gateway PG foram chamados. Caso persist=false usa payload válido e comprova resultado de cálculo, sem instanciar PG. Caso autenticado é validado apenas em alvo PG descartável previamente verificado e afirma write observável. Falha no store não reverte para DB runtime nem ignora persist=true.

## Matriz de validação depois do G1

1. Default com `.env` e variáveis credenciais/sentinelas: gates forçados em zero, suite concluída, todos os contadores de tentativa zero, listeners sem conexão recebida.
2. PG: DATABASE_URL presente sem gate -> skip/0; gate sem manifest/URL dedicada -> erro antes de connect; URL/ID/digest/porta divergentes -> erro antes de connect; marker ausente/divergente -> connect observável, migrate=0; identidade e marker corretos -> migrate habilitado.
3. Neo4j: sem gate -> connect=0; read gate/marker read -> leitura permitida, write negada; write gate/mode incompatível -> erro antes de escrita; identidade+marker write corretos -> operação de projeção permitida.
4. Binance: credenciais sem flag -> cliente/submit=0; flag sem filtro exato ou args extras -> runner rejeita antes do processo/cliente; seleção exata, gate e credenciais testnet -> somente teste canônico pode submeter.
5. Backtest: exercer cada linha da matriz com resposta HTTP e contador do handler/store; nenhum DB para cálculo público/503/401; Bearer válido persiste somente no PG efêmero autenticado.
6. Testes de integração são executados apenas após runner criar alvo dedicado. Testnet é chamada separada e nunca parte do default. Verificação inclui guard estático de bypass e testes RED/GREEN nos seams públicos.

## Rollout, rollback, riscos e alternativas

Após G1, implementar gates fechados primeiro, migrar testes para helpers e depois adicionar runners/markers. Default mantém gates zero. Só job PG descartável fica habilitado onde requerido; Neo4j e Binance ficam desativados até comando dedicado. Rollback reverte helper e runner juntos. Cleanup usa trap e UUID/labels; falha de cleanup nunca causa reuso.

Docker e Neo4j precisam suportar os checks declarados; versões não suportadas falham fechadas. Acesso direto a driver/store pode contornar observer; guard e revisão de chamadas são parte do aceite. Nunca logar URL, password, API key, secret ou dados do manifest. Alternativas rejeitadas: depender de operador limpar `.env`; confiar somente em marker sem provar container; ignorar só Binance; usar `DATABASE_URL` para teste; avaliar somente ausência de writes em vez de tentativas.

## Decisões e pendências

- Owner aprovou uso local de `.env`, sem inferir opt-in a partir de credenciais, e a semântica do backtest acima.
- **Pendente owner:** aprovar/substituir explicitamente nomes e comportamento candidato dos gates PG, Neo4j read/write, Binance; runners e validação `--exact`; container UUID/identidade+marker; observer default e portas/testes da matriz do backtest.
- **Pendente Critic G1:** revisão independente depois da resposta do owner. Sem ambas aprovações, não iniciar G3 nem integrações.
- Não executado neste SDD: código, testes, processos/conexões/migrations, chamadas de rede ou ordens.

## Aceite

G1 exige owner aceitar seams ou declarar substituições e Critic independente aprovar design. G4 exige default observer com zero tentativas, PG/Neo4j provando identidade/marker antes de migrate/ação, Binance recusando qualquer seleção não exata antes do cliente e matriz backtest com respostas/contadores previstos. Credenciais ausentes não contam como evidência de isolamento.
