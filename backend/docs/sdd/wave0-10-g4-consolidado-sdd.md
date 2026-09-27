---
title: SDD W0-10 — G4 consolidado de T-05/T-07/T-10/T-15 e docs de status
description: Fatia Onda 0 de registro: Critic consolida G4 de C9/C10/C12–C17 com evidência de CI e os docs de planning deixam de copiar status e contagens à mão
tags:
  - sdd
  - backend
  - gates
  - docs
  - wave0
status: draft
---

# SDD W0-10 — G4 consolidado e docs de status

- **Estado:** draft. Registro de gate, sem código de produto. Nenhum gate aprovado por este arquivo.
- **Plano:** W0-10 em [master-plan](../planning/master-plan.md) §4.1.

## Contexto

- C16 e C17 têm **G3 aprovado com follow-up** ([monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) linhas 87 e 93-95); G4 depende de V18 (W0-09) e de evidência de CI (W0-02).
- Docs de planning ainda dizem "C17 pendente": [backend-work-plan](../planning/backend-work-plan.md) linhas 31 e 46, [current-state-and-roadmap](../planning/current-state-and-roadmap.md) linha 51.
- Contagens de teste são copiadas à mão em vários docs e divergem (por exemplo, 519 × 520 no bin no mesmo dia; manifesto PG mudando de 29 para 30 numa mudança em andamento). Fontes geradas: a linha `OK: backend gates passed (bin bot: …)` de `backend/scripts/verify-backend-gates.sh:39` e `EXPECTED_PG_INTEGRATION_TESTS` de `backend/scripts/run-pg-integration-tests.sh`.

## Decisão

1. Um Critic independente consolida G4 de C9, C10, C12–C17 (T-05/T-07/T-10/T-15) num único registro, com link para o run de CI (job PG de W0-02) e para a evidência V18 (W0-09).
2. Os docs de planning trocam "C17 pendente" pelo estado real, com link para o registro de G4.
3. Docs deixam de copiar contagens; citam a linha `OK:` do último run de CI e o manifesto PG.

**Alternativa considerada:** um G4 por tarefa (quatro registros). Mais granular, mas as quatro dependem da mesma evidência de CI e V18; um registro reduz repetição. Rejeitada.

## Seams públicos

Nenhum.

## Critérios de aceite

- A1. Registro de G4 com veredito do Critic e links de CI e V18.
- A2. `rg -n -i "C17[^|]{0,40}pendente" backend/docs/planning` sem resultado (fora do histórico).
- A3. Nenhum doc de planning com contagem de teste escrita à mão sem a fonte.

## Dependências

- W0-02 (evidência de CI) e W0-09 (V18). Habilita W1-01.

## Riscos

- Os docs de planning citados têm edição concorrente frequente; fazer a atualização numa única passada, depois de W0-09.

## Validação e rollout

- Só docs. `ok lint` e conferência de links. Rollback: reverter o commit.
