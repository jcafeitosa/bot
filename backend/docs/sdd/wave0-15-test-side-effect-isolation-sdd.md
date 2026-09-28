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

Uma execução ampla de `cargo test` não deve inferir autorização de efeito externo pela presença de credenciais e, por isso, conectar ou migrar bancos, gravar no Neo4j ou enviar ordens à exchange. O comportamento deve ser previsível quando `backend/.env` existe ou quando o shell exporta credenciais. Integrações precisam de opt-in específico, alvo descartável e seleção explícita do teste.

**Objetivo observável:** o caminho padrão da suíte não tenta rede nem persistência, mesmo quando credenciais estão no ambiente por causa de `.env` ou do shell; cada integração com efeitos exige opt-in próprio e prova do destino isolado.

**Fora de escopo:** alterar o fluxo de configuração do runtime `serve`; mudar lógica de negócio de PG/Neo4j/exchange; executar ordem testnet ou iniciar suite PG/Neo4j durante a implementação deste SDD.

## Sistema atual

- `backend/src/main.rs` chama `ensure_dotenv_loaded()` no entrypoint operacional. `backend/src/core/config/env_loader.rs` lê o dotenv padrão e, como fallback, `backend/.env`; seu teste `ensure_dotenv_loaded_is_idempotent` chama essa função dentro do bin de testes. Isso pode inserir valores de `.env` no ambiente compartilhado pelo processo de testes.
- `backend/src/core/persistence/pg_integration.rs::database_for_integration_test` usa `Database::connect_from_env()` e executa migrações. Os testes PG chamam esse helper; sem opt-in específico, um teste normal pode consumir `DATABASE_URL`.
- Os testes de projeção/leitura de grafo podem ser habilitados por configuração de Neo4j via `neo4j_stack_enabled()`; requerem alvo de integração isolado para não tocar o grafo de desenvolvimento.
- `backend/src/modules/exchanges/adapters/binance_spot_testnet_submit.rs::integration_submits_minimal_market_buy_on_testnet` está registrado na suíte comum. Com credenciais visíveis, envia uma ordem Market de teste para BTC/USDT. Credenciais por si só não são autorização suficiente para o efeito.
- A execução deliberada de integração PG é especificada em [W0-02](./wave0-02-ci-pg-fail-loud-sdd.md); este SDD adiciona um gate de segurança anterior a ela.

## Contratos públicos propostos — aguardam concordância

1. **Suíte padrão:** `cargo test` pode herdar variáveis do shell ou carregar `.env` por um teste/configuração local, mas credenciais por si só nunca habilitam integrações com efeitos. Os guards PG, Neo4j e Binance exigem opt-in explícito e alvo permitido. `ensure_dotenv_loaded()` continua válido para o runtime e não deve ser tratado como autorização de teste.
2. **PostgreSQL:** apenas `BOT_RUN_PG_INTEGRATION=1` autoriza testes PG. O helper exige ainda marcador de banco de teste e conexão para banco/instância descartável. Ausência de opt-in significa skip explícito; opt-in sem pré-requisitos falha alto e sem revelar URL/segredo.
3. **Neo4j:** testes que escrevem ou consultam integração externa exigem opt-in próprio e instância de teste descartável identificável. A configuração de runtime/credenciais não deve habilitar esses testes automaticamente. O nome e formato desse opt-in e do marcador precisam ser definidos na revisão G1.
4. **Binance Spot Testnet:** somente a combinação de `BOT_RUN_BINANCE_TESTNET_ORDER=1`, as duas credenciais testnet presentes e filtro exato de `integration_submits_minimal_market_buy_on_testnet` pode submeter a ordem. Fora desse comando, a suíte não faz chamada à Binance; credenciais sozinhas não habilitam a ordem. Credencial ausente ou gate incompleto não envia ordem e produz skip/mensagem diagnóstica segura.
5. **Diagnóstico:** logs/status de gate identificam o motivo do skip ou bloqueio sem incluir endpoint com credenciais, key, secret ou URL de banco.

A lista de alvos isolados deve distinguir o banco de aplicação persistente dos bancos descartáveis. O marcador não substitui a separação da instância/credencial nem permite executar migrations no banco de aplicação.

## Alternativas consideradas

- **Adicionar opt-ins por efeito, permitindo o dotenv local:** mantém a conveniência de configuração operacional sem transformar credenciais em autorização implícita para testes.
- **Continuar carregando `.env` e exigir que o operador lembre de limpar variáveis:** simples, mas transforma credenciais locais em habilitação incidental de efeitos e depende de ambiente externo.
- **Marcar somente a ordem como `#[ignore]`:** reduz o risco da exchange, mas deixa PG/Neo4j acoplados a `.env` e impede prova unificada de que o default não tenta persistência.

## Riscos, validação e rollout

O principal risco é um teste futuro introduzir acesso externo sem passar pelo gate. O verificador precisa observar tentativa, não apenas resultado: com `.env` contendo valores sentinela, a suite padrão deve concluir sem conexões PG/Neo4j, migração, chamada de rede ou ordem. O teste explícito da ordem deve ser validado por seleção exata e pelo gate de opt-in antes de qualquer acesso à rede; executar essa ordem é um efeito separado e requer a autorização já registrada pelo owner. Integrações PG/Neo4j só rodam contra alvos descartáveis com marcador.

Rollout após aprovação G1: (a) adicionar guards de opt-in na suite, preservando `.env` para runtime local; (b) configurar o job local/CI de PG para habilitar somente o opt-in PG contra DB isolado; (c) deixar ordem Binance e Neo4j opt-in desligados no gate geral; (d) fornecer comando dedicado para cada integração externa. Nenhuma publicação/deploy faz parte do trabalho. Rollback reverte guards e runner juntos; até então, não se executa a suite com ambiente contendo credenciais.

## Plano de validação (após G1 e seam acordado)

- **RED/GREEN em seam público:** executar o test harness padrão com dotenv sentinela presente e nenhum opt-in; provar que não há tentativa de abrir conexão PG/Neo4j nem tráfego HTTP e que nenhum dado é gravado. Primeiro, o teste deve falhar na implementação atual onde o carregamento incidental produzir efeito; então implementar a menor guarda e repetir.
- Com PG opt-in ausente, os testes PG são pulados com motivo estável e não conectam. Com PG opt-in presente, banco sem marcador/isolamento é recusado antes de conexão/migração; alvo descartável marcado executa o teste selecionado e passa.
- Com Binance flag ausente (mesmo que as credenciais existam), teste exato não envia ordem. Com flag ligada sem filtro exato, o runner bloqueia antes de iniciar a ordem. Só o comando exato com flag e credenciais pode chegar à exchange testnet.
- Verificar que `verify-backend-gates.sh` e a suíte padrão não habilitam efeitos só porque `.env` ou variáveis herdadas contêm credenciais; executar smoke por interceptador local / conexão proibida que observe zero tentativas. Rodar testes de integração somente com destino isolado e opt-in respectivo.
- Registrar quais testes de escrita Neo4j existem e seu gate observável antes de declarar o objetivo cumprido.

Não executado neste SDD: testes, comandos de integração, conexão aos bancos, solicitação à Binance ou alteração de código/configuração.

## Questões abertas de G1

1. **O owner aprova estes seams públicos e os opt-ins `BOT_RUN_PG_INTEGRATION=1` e `BOT_RUN_BINANCE_TESTNET_ORDER=1`, incluindo exigir filtro exato para a ordem?** A evidência de fechamento é resposta explícita do owner; bloqueia qualquer teste ou código.
2. **Qual contrato exato de opt-in e marcador isolado para Neo4j?** A resposta deve nomear variável, instância/database de teste e prova que a instância é descartável; decider: owner, com revisão independente de segurança/banco.
3. **Credenciais herdadas de `.env` ou do shell podem estar presentes no processo padrão de testes?** Resolvida pelo owner: `.env` é permitido para uso local; presença de credenciais não habilita efeitos. O comportamento verificável é zero tentativa de rede/persistência sem opt-in, provado por interceptador/runner em ambiente controlado.

## Aceite

G1 exige aprovação independente deste desenho e respostas às questões que definem o seam. G4 exige evidência de zero tentativas de rede/persistência na suite padrão com arquivo `.env` presente, gates explícitos para cada integração, rejeição de alvo PG não marcado/não isolado, e seleção exata obrigatória no teste de ordem.

## Adendo — decisões do owner e achado de backtest

O owner esclareceu que `.env` pode ser usado no trabalho local e que o backtest deve funcionar como em produção. Isso autoriza carregar configuração para execução operacional, mas não transforma credenciais disponíveis no processo em opt-in para efeitos colaterais de um teste comum. O gate padrão continua sem efeitos; integrações seguem exigindo opt-in explícito e alvo descartável.

**Contrato aprovado para o backtest:** `POST /api/v1/backtest/sma-crossover` permanece cálculo público quando `persist=false`; com `persist=true`, exige token admin configurado e Bearer válido. O teste público usa `persist=false`; o caso persistente é validado separadamente contra PostgreSQL descartável e com auth válida. Este comportamento depende de autorização condicional no handler porque a classificação middleware atual cobre método+path; revisão independente deve aprovar o desenho técnico e os testes antes do G3. A rota não é provada segura pela matriz de classificação sozinha.

**Evidência de código:** `backend/src/modules/backtest/cli.rs::execute_backtest` chama `persist_dataset_if_configured` quando `BacktestCli.persist` é verdadeiro. Portanto, a descrição antiga de “seis POSTs de cálculo sem efeito colateral” não é suficiente para este endpoint: a flag torna o caminho persistente.

### Seams atualizados

- O owner permite `.env` para runtime e execução local; qualquer credencial herdada pelo processo de teste continua sem habilitar integração por si só.
- O conjunto padrão não tenta conectar ou persistir em PG/Neo4j e não chama Binance, independentemente da presença de credenciais.
- Integração PG exige opt-in, conexão dedicada para instância descartável e marcador criado pelo setup explícito. O helper não deve aceitar apenas um marcador: a configuração/runner também deve provar identidade do alvo isolado antes de migrar. A forma exata dessa prova (variável dedicada, container id/host e validação de banco) fica para aprovação independente G1.
- Teste Neo4j exige opt-in separado e instância dedicada descartável; configuração local ou credencial, isoladamente, não habilita operação.
- A ordem Spot Testnet exige opt-in dedicado e filtro exato, além de credenciais testnet válidas. O opt-in autoriza apenas esse teste; não libera a suíte para outras chamadas externas.
- Backtest sem persistência é público e sem PG; backtest persistente exige Bearer e só testa com PG descartável.

### Status de aceitação atualizado

O owner fechou o princípio de não inferir autorização de efeitos a partir de `.env` (que segue permitido para runtime local), a necessidade de opt-ins/targets descartáveis e o comportamento do backtest conforme a flag. O G1 deste SDD segue **PENDENTE** até revisão técnica independente do adendo, fechamento da prova do destino descartável para PG, e confirmação dos seams específicos (nomes exatos dos opt-ins e mecanismo verificável de filtro exato). Nenhuma integração foi executada.
