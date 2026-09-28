---
title: Postmortem — Efeitos colaterais possíveis na suíte de testes
description: Registro blameless da execução de testes que carregou configuração local e deixou incertos os resultados de integrações.
tags:
  - postmortem
  - testing
  - security
  - database
  - backend
status: review
incident_date: 2026-09-27
---

# Postmortem — Efeitos colaterais possíveis na suíte de testes

## Summary

Em 2026-09-27, uma execução ampla do harness de testes do binário `bot` terminou com 453 testes aprovados e 80 falhos; a saída preservada no registro da tarefa está truncada. O registro também informa que um teste de carregamento de ambiente leu `backend/.env`, tornando credenciais disponíveis a testes de integração. A causa dos 80 failures e se houve efeito em PostgreSQL, Neo4j ou Binance Spot Testnet não podem ser determinadas pelo registro disponível.

Não há impacto em usuários ou produção confirmado nas evidências preservadas. Também não há prova suficiente para concluir que não houve efeito externo. O alcance confirmado é uma falha da suíte local; o status de dados do banco e de uma possível ordem testnet permanece desconhecido. A identidade do build/árvore e o SHA dos comandos não foram preservados.

## Timeline

Os horários exatos não estão preservados. Esta sequência é reconstruída do registro desta tarefa; as contagens não devem ser combinadas porque vieram de comandos distintos.

| Ordem | Evento | Evidência e limite |
|---|---|---|
| Antes da execução ampla | `bash scripts/verify-backend-gates.sh` concluiu a etapa de teste do binário com 520 aprovados; o gate parou depois na verificação de completude documental. As cinco suites workspace de integração não foram executadas naquele ciclo. | Matriz de testes e registro desta tarefa ([matriz](../reference/test-matrix.md)). Este é um evento separado do comando abaixo. |
| Depois | Executou-se `cargo test --locked --bin bot -- --test-threads=1`; o resumo foi 453 aprovados e 80 falhos. A saída detalhada está truncada e o SHA/estado da árvore não foi registrado. | Registro desta tarefa; horário e identidade do checkout não preservados. A lista exata de testes aprovados/falhos não pode ser reconstruída. |
| Durante a execução ampla | O registro relata que `ensure_dotenv_loaded_is_idempotent` carregou `backend/.env`. A matriz/SDD documentam testes binários condicionais a PG, Neo4j e credenciais da Binance; isso torna os efeitos possíveis, mas não prova seleção, conexão, gravação nem submissão de ordem. | [SDD de isolamento](../sdd/wave0-15-test-side-effect-isolation-sdd.md) e [matriz de testes](../reference/test-matrix.md), mais registro desta tarefa. |
| Após a execução | O registro informa que não houve rollback e que foi criado o dump `/tmp/bot-backend-safety-backup/trading_bot-2026-09-27.dump`, SHA-256 `c9682fb5c58f566f4c1a217f5fce043648d9486e1e3b08e42d123ad68e941b8a`. | Registro desta tarefa; horário não preservado. É um snapshot pós-evento; conteúdo e restaurabilidade não foram verificados. Não representa o estado pré-execução. |
| Revisão posterior | Foi relatada uma consulta de leitura ao PostgreSQL com contadores cumulativos/estimados de atividade. Esses contadores não permitem atribuir alterações ao comando. Nenhuma consulta linha a linha nem rollback foi feito. | Resumo da tarefa; os resultados completos da consulta não estão preservados neste artefato. A atribuição causal permanece desconhecida. |

## Scope: tests that ran and tests not established

- **Executado:** o harness de testes do binário `bot`, conforme o comando acima, com 453 aprovados e 80 falhos no agregado. A saída disponível não preserva a lista individual nem o resultado de cada teste.
- **Não executado por esse comando:** os cinco alvos de integração workspace invocados com `cargo test --test …` e o runner dedicado `scripts/run-pg-integration-tests.sh`. Eles não fazem parte do comando `--bin bot`.
- **Possivelmente selecionados, resultado desconhecido:** testes PG, Neo4j e `integration_submits_minimal_market_buy_on_testnet` que residem no harness do binário. A presença de `.env` e a saída truncada não provam que cada um executou ou que teve sucesso/falha.
- **Contexto separado:** em execução anterior, um filtro exato incorreto selecionou zero testes; depois o teste individual autorizado de ordem Spot Testnet foi executado e passou. Isso não prova se a ordem foi selecionada novamente no comando amplo nem se alguma submissão posterior foi aceita.

## Impact

**Confirmado:** a suíte local do binário terminou com 80 falhas e a evidência detalhada é insuficiente para diagnóstico por teste.

**Não confirmado:** impacto em usuários/serviço, integridade final do banco, alterações no Neo4j e qualquer segunda ordem testnet. O dump existente foi criado após a execução e não permite recuperar ou comparar o estado prévio. Não há evidência preservada de quais operações externas, se alguma, foram aceitas.

## Trigger, root cause e sintoma

- **Sintoma:** o comando amplo terminou com 453 testes aprovados e 80 falhos.
- **Trigger:** a execução do harness `--bin bot` em um ambiente onde um teste carregou configuração de `.env`, conforme o registro da tarefa.
- **Root cause do risco de efeitos colaterais:** o desenho da suíte permitia que configuração/credenciais presentes habilitassem testes com efeitos externos sem opt-in dedicado e destino isolado. O carregamento de dotenv em um teste podia tornar essa configuração disponível no mesmo processo. Esta é uma condição de risco de isolamento confirmada pela documentação de implementação ([SDD de isolamento](../sdd/wave0-15-test-side-effect-isolation-sdd.md)); não é uma causa demonstrada dos 80 failures nem prova de qualquer efeito em PG, Neo4j ou Binance.
- **Causa dos 80 failures e resultado de operações externas:** desconhecidos. A saída truncada e a falta de identidade do checkout impedem inferência confiável.

## Contributing factors

- O teste que carrega dotenv e os testes que consomem credenciais compartilham o processo/harness do binário.
- Testes de integração PG, Neo4j e Binance vivem no harness do binário e dependem de variáveis de ambiente; credenciais locais podiam ser lidas sem uma barreira de autorização própria por integração. A [matriz de testes](../reference/test-matrix.md) enumera esses alvos.
- O comando e as credenciais não foram isolados de forma que uma execução genérica provasse zero tentativas externas.
- A saída detalhada e a identidade do checkout não foram preservadas, atrasando a confirmação de quais casos foram selecionados.

## Detection and response

O problema foi percebido ao observar o resultado amplo de 453/80. O registro da tarefa indica que chamadas adicionais a banco/exchange e rollback foram suspensos enquanto se reconciliava o estado. Não há log independente com timestamps nem inventário de efeitos aceitos preservado; por isso, o documento não afirma que banco ou exchange ficaram intactos.

Foi criado um dump depois do comando para preservar um snapshot corrente. Ele não é um backup anterior ao incidente, e a sua capacidade de restauração não foi testada.

## What went well

Depois da execução, o risco de execução incidental foi identificado e, conforme o registro da tarefa, novas chamadas a banco/exchange foram suspensas. O registro informa que não houve rollback e que foi criado um snapshot posterior identificável por path e hash; não registra a motivação para não fazer rollback. O snapshot representa apenas o pós-evento e não estabelece o estado anterior.

## Action items

Todos os itens abaixo estão **abertos**; não há evidência de implementação. Prazos são datas-alvo para o time responsável e devem ser atualizados no acompanhamento do projeto.

| Tipo | Owner | Prazo | Ação e condição verificável de conclusão | Status |
|---|---|---|---|---|
| Prevention | Backend / test infrastructure | 2026-10-05 | Fechar primeiro a aprovação do owner e do Critic G1 para o desenho/prova de egress no [SDD de isolamento](../sdd/wave0-15-test-side-effect-isolation-sdd.md), então implementar o contrato aprovado. O desenho mantém dotenv para runtime local sem inferir autorização de teste a partir das credenciais. Só após aprovação e implementação, comprovar com `.env` sentinela e sem opt-ins que o runner faz zero tentativas de rede, PG ou Neo4j. | Aberto |
| Mitigation | Backend / database owners | 2026-10-05 | Após fechar os seams com owner e Critic independente, implementar opt-in dedicado e prova automatizada de destino descartável para testes PG/Neo4j; registrar teste negativo que recusa DB persistente de desenvolvimento antes de conectar/migrar. Evidência de conclusão: teste negativo passa e o teste explicitamente opt-in usa apenas destino descartável. | Aberto |
| Prevention | Backend / exchange owners | 2026-10-05 | Após fechar os seams com owner e Critic independente, bloquear a ordem testnet fora de runner dedicado, opt-in explícito e seleção exata do teste; provar com credenciais sentinela e interceptador que a suíte padrão não envia requisição de ordem. | Aberto |
| Detection | Backend / test infrastructure | 2026-10-05 | Fazer o relatório de CI/local registrar comando, SHA/estado dirty, resumo por alvo e lista de testes; simular falha para verificar que a saída arquivada permite identificar o teste sem imprimir segredos. | Aberto |

## Related

A busca em `backend/docs/postmortems/` não encontrou postmortems anteriores. Documentos relacionados:

- [SDD T-W0-06 — Isolar efeitos externos da suíte de testes](../sdd/wave0-15-test-side-effect-isolation-sdd.md)
- [SDD W0-02 — Testes PostgreSQL e DB isolado](../sdd/wave0-02-ci-pg-fail-loud-sdd.md)
- [Matriz de testes do backend](../reference/test-matrix.md)
- [SDD T-W0-07 — Escopo deste postmortem](../sdd/wave0-17-test-side-effect-postmortem-sdd.md)
