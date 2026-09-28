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

**Status:** PROPOSED — G1 pendente. O owner aprovou o uso local de `.env` e o comportamento funcional do backtest. Nomes de gates, runners e demais seams abaixo são candidatos pendentes de aprovação do owner e do Critic independente. Sem ambos, não implementar nem executar integrações.

## Contexto e objetivo

A suíte pode herdar credenciais ou carregar `.env`; isso permanece permitido para runtime local. Presença de credenciais nunca equivale a opt-in. Hoje o helper PG conecta/migra via ambiente, Neo4j pode conectar/escrever, o teste Binance pode enviar compra Market BTC/USDT e `execute_backtest` persiste quando `persist=true`.

**Objetivo mensurável:** `cargo test` padrão não tenta conexões PG/Neo4j, migrations, escritas externas nem HTTP de provedores; não submete ordem, mesmo com `.env` e credenciais disponíveis. Cada integração só prossegue por runner explícito após validar gate, alvo descartável e identidade.

## Gates e observer padrão

Wrapper candidato: `backend/scripts/verify-test-isolation.sh`. Define todos os gates de integração como `0` (dotenv não pode habilitá-los), preserva `.env` e executa a suite padrão em sandbox de rede dedicado. Essa sandbox é apenas para testes default; runners PG/Neo4j/Binance opt-in ficam separados.

**Isolamento e prova de egress:** executar o processo completo de testes (incluindo filhos) dentro de container Docker com `--network=none`, sem interfaces/rota externa, em imagem de verificação com `strace`. Rodar `strace -f -e trace=network` envolvendo Cargo e todos os test binaries, gravando auditoria de syscalls. Wrapper analisa cada tentativa `connect`, `sendto`, `sendmsg` e demais syscalls de rede: só permite loopback IPv4/IPv6 e socket Unix local; qualquer DNS/UDP para resolver nomes, sockaddr público, ou tentativa externa resulta em falha de suite mesmo se a aplicação tratou erro. Sandbox bloqueia e auditoria registra tentativa direta, inclusive chamada feita por teste/dependência sem passar pelos adapters. Container não recebe credenciais de infraestrutura/metadata nem usa host network. Se Docker/strace ou auditoria de filhos não estiver disponível, o wrapper falha fechado; não troca para execução sem sandbox.

O sandbox preserva mocks locais: testes podem iniciar servidor mock no próprio namespace em `127.0.0.1` ou `::1`, e conectar a ele. O wrapper valida o endereço de bind/peer observado; mock em host externo, bridge, hostname/DNS ou redirect para fora é rejeitado. Não compartilha listener mock do host, porque `--network=none` isola o namespace.

Observer por seam segue útil para classificação funcional: registra `io_start(effect, target_id, test_name)` para `pg_connect`, `pg_migrate`, `neo4j_connect`, `neo4j_write`, `provider_http` (incluindo todos os clients externos do inventário, como System One/JEV/NIM) e `binance_submit`. A auditoria syscall é autoridade para tentativa de egress; observer e sentinelas PG/Neo4j explicam qual componente tentou e validam que gates impediram efeitos antes do transporte. Nenhum client externo pode usar loopback como exceção genérica: loopback é permitido apenas quando destino foi registrado como mock de teste. Default esperado: zero tentativas não-loopback/DNS e zero efeitos PG/Neo4j/provider/Binance.

## PostgreSQL alinhado ao W0-02

W0-02 define database de teste `trading_bot`, marcador `bot_test_database_marker`, configuração/guard por nome, e um connector de teste `#[cfg(test)]` próprio que exige marker. Este SDD herda esses nomes e semântica. Não propõe `trading_bot_test` nem `test_run_marker`.

Gates candidatos adicionais: `BOT_RUN_PG_INTEGRATION=1`, `BOT_PG_TEST_DATABASE_URL`, `BOT_PG_TEST_RUN_ID`, manifest `backend/.test-targets/pg.json`. A seleção não usa `DATABASE_URL`; DB/name guard continua exigindo `trading_bot`.

Runner cria para cada execução container PostgreSQL novo, com UUID aleatório em label/nome, image digest aprovado pelo W0-02, credencial randômica, database `trading_bot`, volume efêmero e porta loopback aleatória. Nunca reusa DB/volume. Runner verifica container ID, digest, UUID, porta, nome DB e estado; cria marker `bot_test_database_marker` fora do helper e das migrations, depois da validação do alvo, contendo marker type, run UUID, container ID, digest e database. Manifest restrito referencia a identidade e URL de teste sem expor senha.

O helper exige flag exatamente `1`, URL dedicada e manifest válido. Rejeita `DATABASE_URL` como fallback e comprova que host/porta/DB correspondem ao container ID/digest/UUID ativos antes da conexão. O connector `#[cfg(test)]` deve reconciliar explicitamente com `PostgresDatabase::connect_from_url` e guard W0-02: preserva validações compartilhadas de URL, nome `trading_bot`, versão/extensões e segurança, sem permitir que o caminho de teste passe pelo bloqueio runtime de marker. Ele requer `bot_test_database_marker` antes de migration e emite `PG_INTEGRATION_HELPER_OK` conforme W0-02. Runtime mantém a proteção inversa: `connect_from_url` recusa banco marcado. Não duplicar nem afrouxar o name guard.

Marker ausente/divergente falha antes de migration; migração ocorre só após identidade efêmera e marker validados. Gate ausente = skip com zero tentativa. Gate ligado sem manifest/URL/identity = falha fechada sem segredo em logs. W0-02 continua dono do manifesto PG, modo `BOT_PG_INTEGRATION_REQUIRED=1`, versão/extensões, marker em runtime, execução exata e evidência individual de helper.

## Neo4j com gates de leitura/escrita

Gates candidatos: `BOT_RUN_NEO4J_READ_INTEGRATION=1`, `BOT_RUN_NEO4J_WRITE_INTEGRATION=1`; destino `BOT_NEO4J_TEST_URI`, `BOT_NEO4J_TEST_DATABASE`, `BOT_NEO4J_TEST_RUN_ID`; manifest `backend/.test-targets/neo4j.json`.

Runner cria instância/container vazio e descartável por run, credencial randômica, endpoint loopback exclusivo e marker contendo UUID, container ID, image digest, database e mode (`read`/`write`). Valida marker e identidade no container e no Neo4j antes de liberar manifest.

Read gate autoriza apenas leitura. Para prosseguir, o runner/helper precisa provar enforcement efetivo read-only da credencial/role/database ou usar adapter/transação cujo contrato garante ausência de writes. Se mecanismo read-only comprovável não estiver disponível, falha fechado antes do driver/connect; “read gate” sozinho não é proteção. Write gate é independente, requer marker `mode=write` e container descartável recém-criado. `BOT_GRAPH_ENABLED`/credenciais runtime não habilitam integração. Helper valida gate e target identity antes de conexão. Operação ambígua é classificada como write.

Inventário inicial: leitura em `core/database/graph_query.rs`, `core/database/neo4j.rs`; projeção/escrita em `modules/agents/adapters/graph_projection.rs`, `modules/bots/adapters/graph_projection.rs`, `modules/orders/adapters/graph_projection.rs`. Confirmar por função/teste durante implementação.

## Binance Spot Testnet

Candidatos: `BOT_RUN_BINANCE_TESTNET_ORDER=1`, runner `backend/scripts/run-binance-testnet-order.sh`, teste canônico `modules::exchanges::adapters::binance_spot_testnet_submit::tests::integration_submits_minimal_market_buy_on_testnet`.

Script não aceita argumento livre e monta o comando Cargo com nome completo canônico e `-- --exact --test-threads=1`. Rejeita qualquer filtro/arg extra ou modo paralelo antes de invocar o binário. Teste verifica `args_os()` do harness (`--exact`, nome canônico, 1 thread) e gate exatamente `1` antes de ler credenciais ou construir cliente. Exige credenciais exclusivamente Spot Testnet; base URL é fixa para Testnet. Ausência de gate/seleção exata encerra antes de client/transport e registra zero submit. Apenas runner dedicado pode enviar a ordem autorizada; nunca integra o default.

## Backtest HTTP: auth anterior à persistência

Owner aprovou: `POST /api/v1/backtest/sma-crossover` com `persist=false` é cálculo público; `persist=true` exige token admin configurado e Bearer válido. Handler avalia auth antes de executar persistência e recebe explicitamente porta/store; não resolve DB de produção de ambiente antes da auth. CLI pode manter caminho operacional.

| Request | Resultado | Acesso/contador DB | Ambiente |
|---|---|---:|---|
| Payload válido, `persist=false`, sem token/PG | sucesso HTTP e resposta de cálculo conforme contrato | 0 | default |
| `persist=true`, token ausente/fraco | 503 `admin_auth_not_configured` | 0 | unit handler |
| `persist=true`, token configurado, bearer ausente/errado | 401 `unauthorized` | 0 | unit handler |
| `persist=true`, bearer válido | sucesso persistente | somente PG ephemeral com marker válido | runner PG |
| método/path inválido | 404/405 conforme roteamento | 0 | roteador |

Teste de persist=false prova cálculo com payload válido sem instanciar PG. Casos 503/401 afirmam store e gateway não chamados. Bearer válido grava somente no PG descartável verificado e prova write. Falha do store não recua para DB runtime nem ignora persist.

## Plano de validação após G1

1. Rodar wrapper default com `.env` e credenciais sentinela: gates zerados; observer, sentinelas e contadores mostram zero tentativa PG, migration, Neo4j, provider HTTP e Binance.
2. PG: `DATABASE_URL` sozinho -> skip/0; gate sem URL/manifest -> falha antes de conectar; container identity divergente -> falha antes de conectar; marker ausente/divergente -> migration=0; identity+marker `trading_bot` corretos -> helper emite prova e migration permitida. Runtime contra marker continua recusado.
3. Neo4j: sem gate -> connect=0; read gate com enforcement não verificável -> falha antes de connect; read-only válido -> só leitura; read path tentando write -> negado; write gate/mode incompatível -> falha; identity+marker write válidos -> projeção.
4. HTTP: credenciais de provedores disponíveis com gate default -> DNS/socket/HTTP externo bloqueado e contador provider_http=0; mock loopback explicitamente permitido; redirect para host externo bloqueado.
5. Binance: credencial sem gate -> client/submit=0; filtro não exato/args extras -> reject antes do cliente; gate+exact+credenciais Testnet só pode rodar o teste canônico.
6. Backtest: executar cada linha com resposta HTTP e counters; persist=false/503/401 sem DB, Bearer válido somente com PG efêmero.
7. Guard estático de bypass e TDD RED/GREEN em seams públicos. Integrações somente após target efêmero, nenhum comando externo no default.

## Rollout, rollback e riscos

Após aprovação owner+G1, implementar helpers fail-closed, migrar integrações e adicionar runners. Default mantém gates 0. Job PG usa o contrato W0-02 e target descartável; Neo4j/Binance ficam desligados fora de seus runners. Cleanup por UUID/trap; falha de cleanup não reusa target. Versões/container sem prova de identidade, marker ou read-only falham fechados.

Chamadas diretas a driver/provider podem contornar observer; guard estático e revisão do inventário localizam bypass. Nunca logar URLs, secrets, keys, manifests sensíveis ou dados de conexão. Alternativas rejeitadas: operador limpar `.env`; marker sem identidade do container; ignorar só Binance; database de teste com nome diferente que afrouxe guard W0-02; checar só gravações e não tentativas.

## Aprovação e critérios

- Owner acordou uso de `.env` local sem inferir autorização e comportamento do backtest. **Pendente owner:** aprovar ou substituir os gates/runner/filtro exato/observer e os métodos de isolamento descritos.
- **Pendente Critic G1:** revisão independente após decisão do owner. G1 bloqueia G3.
- Não executado: código, testes, banco/driver, rede externa ou ordem.
- G1: owner aprova seams e Critic independente aprova desenho. G4: default comprova zero tentativa de toda rede externa; PG alinha W0-02 com marker e identity anterior à migration; Neo4j read-only é comprovado ou falha fechado; Binance rejeita filtro diverso antes de client; matriz backtest produz resultados/counters esperados.
