---
title: SDD T-W0-07 — Postmortem de efeitos colaterais em testes
description: Escopo e critérios de evidência para documentar a execução de testes que carregou configuração local.
tags:
  - sdd
  - postmortem
  - operations
  - security
status: proposed
---

# SDD T-W0-07 — Postmortem de efeitos colaterais em testes

## Contexto e objetivo

Registrar, sem atribuição de culpa, o incidente da execução ampla do binário de testes em 2026-09-27. O relatório deve separar fatos confirmados de hipóteses, preservar incertezas sobre integrações selecionadas e documentar contenção e ações verificáveis.

## Artefato e seam

Um postmortem único em `postmortems/2026-09-27-test-suite-side-effects.md`, seguindo o template e as seções de postmortem instaladas neste projeto. A documentação ficará sob `backend/docs`, gerenciada por OpenKnowledge. Nenhuma API de produto ou teste será alterado.

## Evidência e limites

Usar apenas: saída registrada no contexto desta tarefa (453 testes passaram, 80 falharam, saída truncada), matriz de testes e SDD T-W0-06 existentes, além do snapshot descrito no contexto como criado após o comando. Não executar novamente a suíte nem consultar PostgreSQL, Neo4j ou Binance. Não afirmar contagens de testes externos individuais, transações, persistência final, número/status de ordens ou estado anterior do banco sem artefato que prove. Marcar eventos de transcript sem timestamp recuperável como reconstruídos e timestamp não preservado.

## Critérios de aceite

- Revisão independente G1 deste escopo antes do postmortem.
- Timeline distingue horário indisponível, sequência observada e fontes.
- Impacto confirmado separado do potencial/incerto.
- Resposta documenta parada de novas operações, ausência de rollback e limites do snapshot pós-evento.
- Root cause e contribuintes descrevem lacunas sistêmicas (dotenv compartilhado por testes, integrações habilitadas por env), sem culpa pessoal.
- Ações têm equipe/role responsável, data e condição observável de conclusão; não afirmar que foram implementadas.
- Revisão independente G3 e OpenKnowledge audit do documento, com achados registrados.

## Riscos e alternativas

O risco documental principal é transformar inferência em fato ou sugerir que o snapshot pós-evento restaura pré-estado. Não atribuir valores secretos nem identificadores sensíveis. Se evidência necessária não existir, explicitar a lacuna em vez de reconstruí-la como certeza. Alternativa de esperar por logs completos produziria maior precisão, mas não é pré-condição para registrar o que já se conhece e as incertezas.

## Plano de execução

Após G1: criar o postmortem pelo template de OpenKnowledge; checar links e lint com `audit`; enviar a versão exata à instância Critic independente; corrigir bloqueantes/importantes dentro de até três ciclos. Nenhum comando com efeitos externos está no plano.
