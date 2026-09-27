---
title: Análise de capacidades do módulo org
description: Escopo provisório para cargos, posições, unidades, ocupantes e governança de agentes
tags:
  - research
  - org
  - agents
  - organization
  - backend
status: provisional
---
# Análise de capacidades do módulo `org`

## Propósito e premissas

Esta análise é provisória: identifica responsabilidades e dependências para o futuro módulo `org`, mas não aprova schema, API ou implementação. A proposta separa `org` (estrutura e atribuições) de `agents` (identidade, ciclo de vida e capacidades). As premissas discutidas são catálogo reutilizável de cargos, posições concretas, ocupantes humanos ou agentes, no máximo um ocupante ativo por posição e PostgreSQL desde o início. Cargo pode sugerir capabilities, mas associação de cargo não as ativa até existir autorização verificável e auditada do owner.

A documentação atual chama a etapa agents de `IdentityOnly`, limita o supervisor a owner/agente e lista autenticação verificável do owner e bootstrap seguro como pendências. Consulte [SDD agents](../sdd/agents-module-sdd.md), [pesquisa de capacidades de agents](./agents-capability-research.md), [análise de módulos previstos](../planning/unimplemented-modules-analysis.md), [catálogo de módulos](../architecture/module-catalog.md) e [convenção MVC](../sdd/modules-mvc-convention-sdd.md).

## Lacuna atual

Hoje, `AgentRole` é enum fixo (`Ceo`, `LevelB`, `LevelA`, `Specialist`, `Worker`) e `SupervisorRef` aponta para `OwnerId` ou `AgentId`. `AgentDefinition` agrega identidade, agência, papel, supervisor, lifecycle e capabilities. `validate_hierarchy` verifica papel supervisor, mesma agência e ciclos. Não há entidade de organização/unidade, cargo reutilizável, posição vaga, pessoa humana não-owner como ocupante, assignment datado ou histórico de ocupação. Esse conjunto é maior que um simples aumento de `AgentRole`.

## Responsabilidades que `org` deve cobrir

1. **Organização e unidades:** tenant/agência, departamentos/equipes e relações pai-filho; todas as leituras e mutações isoladas pelo identificador da organização.
2. **Catálogo de cargos:** ID estável, nome, descrição, faixa/nível, responsabilidades e estado/versionamento. Um cargo pode ser usado por várias posições.
3. **Posições:** posto concreto vinculado a organização/unidade e cargo, com supervisor por posição, estado (planejada, aberta, preenchida, congelada, encerrada) e no máximo um ocupante ativo no MVP.
4. **Atribuições:** histórico de ocupação por pessoa humana ou agente, com início/fim, autor, motivo e resultado auditado; transferências e encerramentos devem ser transacionais.
5. **Governança:** operações controladas para criar, ativar, suspender e encerrar unidades/cargos/posições, nomear, transferir e remover ocupantes e consultar o organograma e vagas.
6. **Auditoria organizacional:** ator, escopo, alvo, ação, timestamp, motivo e resultado. Essa trilha registra mudanças de estrutura; `agent_identity_events` permanece dona de mudanças do ciclo de vida de identidade.

`org` não deve duplicar `AgentId`, `AgentLifecycleState` ou `AgentCapabilities`. Posições podem referenciar `AgentId`; pessoas humanas precisam de principal de identidade estável. Não presumir que `OwnerId` identifica todas as pessoas do sistema.

## Limites de autoridade

Posição e cargo descrevem estrutura, responsabilidade e capacidades recomendadas. Hierarquia organizacional, por si só, não prova quem fez a chamada nem autoriza alteração. O SDD de agents mantém capabilities efetivas explícitas. A concessão automática por cargo permanece bloqueada até que o sistema tenha autenticação de owner verificável, política de approval, trilha de autorização e processo de revogação aprovados. O cargo pode declarar capabilities sugeridas/requeridas, mas a mudança efetiva pertence a um workflow separado e auditado.

## Dependências

- **Identidade/autenticação:** autenticar owner e demais atores humanos; `BOT_HTTP_ADMIN_TOKEN` é seam administrativo, não prova identidade humana.
- **Agents:** referenciar identidades como ocupantes e ler lifecycle/capabilities sem criar ou ativar agentes implicitamente.
- **Persistência:** migrações e privilégios próprios. O código atual conecta o store geral via `DATABASE_URL` para `trading_bot`; não se deve reutilizar o migrator global contra `bot_agents`, pois inclui outros domínios.
- **Auditoria:** retenção, acesso, formato e minimização de dados definidos antes de produção.
- **Approval de authority:** necessário antes de atribuir capabilities a partir de cargo.

Runtime, workers, scheduler, tools/sandbox, memória e canais são módulos/fases independentes; não são dependências do primeiro `org`.

## Modelo conceitual

```text
Organization 1—N OrgUnit 1—N Position N—1 JobTitle
Position 1—N AssignmentHistory; no máximo um assignment ativo
Assignment.Occupant = HumanPrincipal | AgentId
Position.supervisor_position_id = outra posição da mesma organização
```

A hierarquia de posições não deve ser duplicada em `AgentDefinition.supervisor`. Durante coexistência, escolher qual árvore é fonte de verdade; qualquer projeção para `agents` precisa de migração validada e regra para divergências.

## Alternativas

| Alternativa | Avaliação |
|---|---|
| **Módulo `org` separado (recomendado)** | Mantém identidade e capacidades em `agents`, e estrutura/ocupação em `org`; suporta cargos editáveis, vagas e ocupantes humanos. Exige integração e migração da árvore fixa atual. |
| Expandir `agents` | Menor wiring inicial, mas mistura identidade com estrutura mutável, ocupação e história. |
| Só estender `AgentRole` | Não representa cargos customizados, posições vazias, vários postos nem ocupantes humanos; atende somente hierarquia fixa. |

## Fases recomendadas

- **0 — decisões de domínio:** se organização equivale a `AgencyId`, identidade humana, fonte da hierarquia, regras de supervisão, headcount e aprovação do owner.
- **1 — núcleo `org` sem grants:** unidades, cargos, posições, assignment humano/agente, constraint de ocupante único, auditoria, consultas e PostgreSQL.
- **2 — integração com `agents`:** validar agentes ocupantes e resolver projeção/migração da hierarquia sem duas fontes divergentes.
- **3 — capabilities aprovadas:** policy declarativa por cargo, aprovação verificável, atualização de capabilities em `agents`, revogação e eventos auditados.
- **4 — runtime e tools:** SDDs separados, threat model, autorização por ação, worker e recuperação.

## Verificação, rollout e rollback

Testes de domínio cobrem ciclos em unidades/posições, supervisor de outra organização, cargo inativo, assignment duplicado, ocupante inelegível e transferências. Testes HTTP verificam autenticação, isolamento e transições inválidas. Testes PostgreSQL em database descartável comprovam constraints e atomicidade de transferência (falha no assignment novo faz rollback do encerramento anterior), migração e cold-start. Não usar um banco de desenvolvimento compartilhado para esses testes.

Rollout aditivo: criar schema e constraints em base isolada, validar/backfill da árvore existente, comparar árvores e definir corte de fonte de verdade antes de aceitar novas mutações. Rollback exige que nenhuma atribuição nova fique sem representação no modelo legado.

## Perguntas que bloqueiam o SDD final

1. `Organization` e `AgencyId` são a mesma entidade ou uma agência pode conter várias organizações/unidades?
2. Qual sistema autentica `HumanPrincipal` e como é criado o primeiro owner?
3. CEO deve reportar diretamente ao owner e os níveis seguintes precisam ser estritos ou podem variar por cargo?
4. A posição representa uma vaga de headcount/orçamento ou apenas um nó hierárquico? Pode uma pessoa/agente ocupar várias posições simultaneamente?
5. Cargo precisa de versionamento imutável quando já referenciado, ou mudanças atualizam todas as posições?
6. A árvore de supervisão atual de `agents` será substituída, projetada ou mantida paralela durante migração?
7. Quais capabilities cada cargo recomenda; qual owner pode aprovar grants, revogação, expiração e exceções?
8. Que cliente precisa da API de organograma, quais operações são prioritárias e quais reads exigem aprovação?
9. Qual schema/database PostgreSQL e papel de runtime serão autorizados para `org`?
10. Qual retenção e visibilidade do histórico pessoal de atribuições são necessárias?

## Estado

`provisional`: a estrutura proposta é coerente com os tipos atuais, mas as decisões acima e a revisão independente ainda são necessárias. Este documento não aprova concessão automática de autoridade nem inicia implementação.