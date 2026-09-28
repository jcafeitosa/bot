---
title: SDD T-W0-06 — Isolar efeitos externos da suíte de testes
description: Contrato de segurança para impedir que cargo test herde efeitos de dotenv, bancos ou Binance sem opt-in isolado.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: draft
---
# SDD T-W0-06 — Isolar efeitos externos da suíte de testes

**Status:** PROPOSED — G1 pendente. Este documento propõe seams públicos para aprovação do owner; não autoriza implementação.

## Problema e objetivo

Uma execução ampla de `cargo test` não deve descobrir credenciais locais e, por isso, conectar ou migrar bancos, gravar no Neo4j ou enviar ordens à exchange. O comportamento deve ser previsível mesmo quando `backend/.env` existe. Operações de integração precisam de opt-in específico, alvo descartável e seleção explícita do teste.

**Objetivo observável:** o caminho padrão da suíte não lê `.env` e não tenta rede nem persistência, mesmo se o arquivo contém credenciais; cada integração com efeitos exige autorização própria e prova do destino isolado.

**Fora de escopo:** alterar o fluxo de configuração do runtime `serve`; mudar lógica de negócio de PG/Neo4j/exchange; executar ordem testnet ou iniciar suite PG/Neo4j durante a implementação deste SDD.

## Sistema atual

- `backend/src/main.rs` chama `ensure_dotenv_loaded()` no entrypoint operacional. `backend/src/core/config/env_loader.rs` lê o dotenv padrão e, como fallback, `backend/.env`; seu teste `ensure_dotenv_loaded_is_idempotent` chama essa função dentro do bin de testes. Isso pode inserir valores de `.env` no ambiente compartilhado pelo processo de testes.
- `backend/src/core/persistence/pg_integration.rs::database_for_integration_test` usa `Database::connect_from_env()` e executa migrações. Os testes PG chamam esse helper; sem opt-in específico, um teste normal pode consumir `DATABASE_URL`.
- Os testes de projeção/leitura de grafo podem ser habilitados por configuração de Neo4j via `neo4j_stack_enabled()`; requerem alvo de integração isolado para não tocar o grafo de desenvolvimento.
- `backend/src/modules/exchanges/adapters/binance_spot_testnet_submit.rs::integration_submits_minimal_market_buy_on_testnet` está registrado na suíte comum. Com credenciais visíveis, envia uma ordem Market de teste para BTC/USDT. Credenciais por si só não são autorização suficiente para o efeito.
- A execução deliberada de integração PG é especificada em [W0-02](./wave0-02-ci-pg-fail-loud-sdd.md); este SDD adiciona um gate de segurança anterior a ela.

## Contratos públicos propostos — aguardam concordância

1. **Suíte padrão:** `cargo test` não carrega dotenv em nenhum teste. `ensure_dotenv_loaded()` permanece no runtime, fora do caminho de inicialização do test harness. A presença de credenciais herdadas do shell nunca habilita por si só chamadas de integração com efeitos.
2. **PostgreSQL:** apenas `BOT_RUN_PG_INTEGRATION=1` autoriza testes PG. O helper exige ainda marcador de banco de teste e conexão para banco/instância descartável. Ausência de opt-in significa skip explícito; opt-in sem pré-requisitos falha alto e sem revelar URL/segredo.
3. **Neo4j:** testes que escrevem ou consultam integração externa exigem opt-in próprio e instância de teste descartável identificável. A configuração de runtime/credenciais não deve habilitar esses testes automaticamente. O nome e formato desse opt-in e do marcador precisam ser definidos na revisão G1.
4. **Binance Spot Testnet:** somente a combinação de `BOT_RUN_BINANCE_TESTNET_ORDER=1`, as duas credenciais testnet presentes e filtro exato de `integration_submits_minimal_market_buy_on_testnet` pode submeter a ordem. Fora desse comando, a suíte não faz chamada à Binance; credenciais sozinhas não habilitam a ordem. Credencial ausente ou gate incompleto não envia ordem e produz skip/mensagem diagnóstica segura.
5. **Diagnóstico:** logs/status de gate identificam o motivo do skip ou bloqueio sem incluir endpoint com credenciais, key, secret ou URL de banco.

A lista de alvos isolados deve distinguir o banco de aplicação persistente dos bancos descartáveis. O marcador não substitui a separação da instância/credencial nem permite executar migrations no banco de aplicação.

## Alternativas consideradas

- **Remover dotenv do processo de testes e adicionar opt-ins por efeito (proposta):** mantém testes unitários sem configuração local e torna explícita cada integração.
- **Continuar carregando `.env` e exigir que o operador lembre de limpar variáveis:** simples, mas transforma credenciais locais em habilitação incidental de efeitos e depende de ambiente externo.
- **Marcar somente a ordem como `#[ignore]`:** reduz o risco da exchange, mas deixa PG/Neo4j acoplados a `.env` e impede prova unificada de que o default não tenta persistência.

## Riscos, validação e rollout

O principal risco é um teste futuro introduzir acesso externo sem passar pelo gate. O verificador precisa observar tentativa, não apenas resultado: com `.env` contendo valores sentinela, a suite padrão deve concluir sem conexões PG/Neo4j, migração, chamada de rede ou ordem. O teste explícito da ordem deve ser validado por seleção exata e pelo gate de opt-in antes de qualquer acesso à rede; executar essa ordem é um efeito separado e requer a autorização já registrada pelo owner. Integrações PG/Neo4j só rodam contra alvos descartáveis com marcador.

Rollout após aprovação G1: (a) tornar a suite padrão sem dotenv e adicionar guards; (b) configurar o job local/CI de PG para habilitar somente o opt-in PG contra DB isolado; (c) deixar ordem Binance e Neo4j opt-in desligados no gate geral; (d) fornecer comando dedicado para cada integração externa. Nenhuma publicação/deploy faz parte do trabalho. Rollback reverte guards e runner juntos; até então, não se executa a suite com ambiente contendo credenciais.

## Plano de validação (após G1 e seam acordado)

- **RED/GREEN em seam público:** executar o test harness padrão com dotenv sentinela presente e nenhum opt-in; provar que não há tentativa de abrir conexão PG/Neo4j nem tráfego HTTP e que nenhum dado é gravado. Primeiro, o teste deve falhar na implementação atual onde o carregamento incidental produzir efeito; então implementar a menor guarda e repetir.
- Com PG opt-in ausente, os testes PG são pulados com motivo estável e não conectam. Com PG opt-in presente, banco sem marcador/isolamento é recusado antes de conexão/migração; alvo descartável marcado executa o teste selecionado e passa.
- Com Binance flag ausente (mesmo que as credenciais existam), teste exato não envia ordem. Com flag ligada sem filtro exato, o runner bloqueia antes de iniciar a ordem. Só o comando exato com flag e credenciais pode chegar à exchange testnet.
- Verificar que `verify-backend-gates.sh` e a suíte padrão não sourceiam `.env`; executar smoke por interceptador local / conexão proibida que observe zero tentativas. Rodar testes de integração somente com destino isolado e opt-in respectivo.
- Registrar quais testes de escrita Neo4j existem e seu gate observável antes de declarar o objetivo cumprido.

Não executado neste SDD: testes, comandos de integração, conexão aos bancos, solicitação à Binance ou alteração de código/configuração.

## Questões abertas de G1

1. **O owner aprova estes seams públicos e os opt-ins `BOT_RUN_PG_INTEGRATION=1` e `BOT_RUN_BINANCE_TESTNET_ORDER=1`, incluindo exigir filtro exato para a ordem?** A evidência de fechamento é resposta explícita do owner; bloqueia qualquer teste ou código.
2. **Qual contrato exato de opt-in e marcador isolado para Neo4j?** A resposta deve nomear variável, instância/database de teste e prova que a instância é descartável; decider: owner, com revisão independente de segurança/banco.
3. **Para o default, o bloqueio deve observar zero tentativa de conexão/tráfego por interceptor ou também garantir que o processo de testes não receba credenciais do ambiente pai?** Proposta: ambos para CI/local gate reproduzível; teste em ambiente controlado decide.

## Aceite

G1 exige aprovação independente deste desenho e respostas às questões que definem o seam. G4 exige evidência de zero tentativas de rede/persistência na suite padrão com arquivo `.env` presente, gates explícitos para cada integração, rejeição de alvo PG não marcado/não isolado, e seleção exata obrigatória no teste de ordem.