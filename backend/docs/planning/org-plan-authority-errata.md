---
title: Errata — autoridade de grants e stores do plano org
description: "Correção normativa do plano org: grants e revocation epoch são autoritativos em PostgreSQL org"
tags:
  - planning
  - org
  - agents
  - postgres
  - neo4j
status: draft
---
# Errata — plano de implementação de org

**Estado:** draft; parte integrante da leitura do plano até a atualização convergida do documento base.

## Correção normativa

Esta errata substitui qualquer menção no plano `[[planning/org-complete-implementation-plan]]` que indique que grants ficam cross-database, tornam-se ativos somente após confirmação no `agents`, ou que o `agents` read model é a fonte de autorização.

- PostgreSQL schema `org` é a única fonte autoritativa de `capability_grants` e `revocation_epoch`.
- `agents` e Neo4j recebem projeções/read models; estado ausente, atrasado ou divergente nessas projeções nunca autoriza uma ação nem mantém uma autorização revogada.
- O runtime lê o grant, assignment, policy ancestry e epoch atuais do PG `org` imediatamente antes de cada efeito. Se não puder consultar/validar o SoT, nega/cancela.
- A outbox é gravada na transação local PG de `org` e serve apenas à propagação/rebuild das projeções. Não existe grant SoT nem transação de autoridade em outro database.
- Neo4j down degrada somente consultas que requerem traversal; PG writes/read models diretos continuam. PG down bloqueia mutações, approvals e decisões de autorização.
- `task.start` é uma mutation/capability separada de `tool.invoke`: task start grantado não autoriza ferramentas/efeitos; cada efeito repete a checagem no SoT.

## Ordem normativa

1. Este texto prevalece sobre frases legadas ou referências de Review Focus conflitantes no plano base.
2. A implementação só começa após SDD e plano reconciliados, Critic independente aprovado e owner aprovar explicitamente ambos.
3. A errata não aprova código, migrations, grants, endpoints ou cutover.
