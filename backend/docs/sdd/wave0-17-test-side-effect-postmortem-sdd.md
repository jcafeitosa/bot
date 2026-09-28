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

Manter separados dois eventos na ordem registrada: (1) `bash scripts/verify-backend-gates.sh`, cujo teste do bin `bot` passou com 520 e a etapa posterior de completude falhou; e (2) `cargo test --locked --bin bot -- --test-threads=1`, com 453 testes aprovados e 80 falhos, saída truncada. Não há SHA/identidade limpa do checkout associado a esses eventos na evidência disponível; registrar como desconhecido, sem fundir contagens nem inferir a lista exata de testes executados. A matriz e SDD T-W0-06 contextualizam quais testes condicionais poderiam ter sido selecionados, mas não provam que cada um foi executado.

O registro da tarefa informa que a execução ampla carregou `backend/.env`, que não foi feito rollback, e que o snapshot `/tmp/bot-backend-safety-backup/trading_bot-2026-09-27.dump` foi criado depois do comando (SHA-256 `c9682fb5c58f566f4c1a217f5fce043648d9486e1e3b08e42d123ad68e941b8a`). Registrar esses itens como declarações do registro da tarefa, com horário não preservado, sem apresentá-los como observação independente. O snapshot é pós-evento, não prova o estado anterior e seu conteúdo/restaurabilidade não foi verificado. Não afirmar conexões, migrações, gravações finais, número/status de ordens nem impacto na integridade sem artefato observável. Não executar novamente a suíte nem consultar PostgreSQL, Neo4j ou Binance. Marcar eventos do registro sem timestamp recuperável como reconstruídos e timestamp não preservado.

## Critérios de aceite

- Revisão independente G1 deste escopo antes do postmortem.
- Timeline distingue horário indisponível, sequência observada e fontes.
- Impacto confirmado separado do potencial/incerto.
- Resposta limita-se a eventos sustentados pelo registro; quando não houver log independente, explicita que não há registro preservado. O snapshot é identificado por path/hash e descrito somente como pós-evento, com conteúdo e restaurabilidade não verificados.
- Inclui `What went well` mesmo que registre honestamente que não há evidência suficiente para identificar um fator positivo.
- Inclui `Related` após busca no corpus de postmortems existentes; linka correspondências existentes, ou declara que a busca não encontrou postmortems relacionados.
- Root cause e contribuintes descrevem o hazard confirmado de isolamento/exposição de credenciais por dotenv compartilhado e integrações elegíveis por ambiente, sem afirmar que isso causou os 80 failures ou qualquer resultado em PG/Neo4j/Binance. A causa dos failures e os resultados de efeitos externos continuam desconhecidos por causa da seleção não recuperável e saída truncada; sem culpa pessoal.
- Ações têm equipe/role responsável, data e condição observável de conclusão; não afirmar que foram implementadas.
- Revisão independente G3 e OpenKnowledge audit do documento, com achados registrados.

## Riscos e alternativas

O risco documental principal é transformar inferência em fato ou sugerir que o snapshot pós-evento restaura pré-estado. Não atribuir valores secretos nem identificadores sensíveis. Se evidência necessária não existir, explicitar a lacuna em vez de reconstruí-la como certeza. Alternativa de esperar por logs completos produziria maior precisão, mas não é pré-condição para registrar o que já se conhece e as incertezas.

## Plano de execução

Após G1: criar o postmortem pelo template de OpenKnowledge; checar links e lint com `audit`; enviar a versão exata à instância Critic independente; corrigir bloqueantes/importantes dentro de até três ciclos. Nenhum comando com efeitos externos está no plano.
