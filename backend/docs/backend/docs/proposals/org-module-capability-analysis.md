---
type: proposal
description: Análise provisória do módulo org para estruturar cargos, posições, ocupantes e governança de agentes
status: provisional
tags:
  - proposal
  - org
  - agents
  - organization
  - backend
---
# Análise do módulo `org` para o sistema de agentes

## Objetivo e estado

Esta é uma análise arquitetural provisória, não uma especificação aprovada nem autorização para implementar. O módulo `org` deve modelar a estrutura organizacional e suas atribuições; `agents` continua dono de identidade, ciclo de vida e capacidades efetivas. Premissas acordadas até agora: catálogo de cargos reutilizáveis, posições concretas, ocupantes humanos ou agentes, no máximo um ocupante ativo por posição, PostgreSQL durável e capacidades sugeridas pelo cargo sem grant automático até haver aprovação verificável de owner.

## O que existe hoje

O código de `agents` tem `AgentRole` como enum fixo `Ceo`, `LevelB`, `LevelA`, `Specialist`, `Worker`; `SupervisorRef` aponta a `OwnerId` ou `AgentId`; `AgentDefinition` combina identidade, agência, papel, supervisor, estado de ciclo de vida e capabilities. `validate_hierarchy` verifica papel do supervisor, mesma agência e ciclos. Isso suporta a cadeia atual, mas não representa departamentos/unidades, cargos reutilizáveis, vagas organizacionais, pessoas que não sejam owners, atribuições com datas ou histórico de ocupação. As capacidades de agente estão explicitamente opt-in no modelo atual.

Fontes internas: [SDD agents](../sdd/agents-module-sdd.md), [pesquisa de capacidades](../research/agents-capability-research.md), [análise de módulos não implementados](../planning/unimplemented-modules-analysis.md), [catálogo de módulos](../architecture/module-catalog.md) e [convenção MVC](../sdd/modules-mvc-convention-sdd.md).

## Responsabilidade recomendada

`org` deve ser fonte de verdade para:

- **Organização e unidades:** tenant/agência, departamentos/equipes e parent-child, com isolamento obrigatório por organização.
- **Catálogo de cargos:** chave estável, nome, descrição, nível/faixa organizacional e versão/estado (draft, active, retired). Um cargo descreve responsabilidades e requisitos, sem executar política sozinho.
- **Posições:** posto concreto ligado a organização/unidade e cargo; supervisor por posição; estado (planned/open/filled/frozen/retired); capacidade nominal (MVP: uma pessoa); eventual identificador de orçamento/headcount se requisito de produto confirmar.
- **Atribuições:** histórico append-only de atribuições pessoa/agente → posição, com início/fim, ator que fez a mudança, motivo e estado. Constraint deve garantir no máximo uma ocupação ativa por posição e impedir atribuição simultânea incompatível ao mesmo ocupante conforme decisão futura.
- **Governança de mudanças:** criar/editar/ativar/aposentar cargos e posições; nomear, transferir, suspender e remover ocupantes; cada transição validada e auditada.
- **Leituras:** organograma por unidade, ocupante atual, vagas abertas, cadeia supervisor/subordinado e histórico de atribuição, sempre filtrados pela organização.

`org` não deve duplicar `AgentDefinition`, `AgentLifecycleState`, `AgentCapabilities` ou a autenticação do owner. A posição pode referenciar `AgentId`; uma pessoa humana precisa de um tipo estável próprio ou de uma integração futura com identidade de pessoas. Não usar `OwnerId` como identidade universal de todo ocupante sem decisão de produto.

## Dependências e módulos auxiliares

1. **Identidade/autenticação de pessoas e owner (bloqueador):** identificar ator humano em requests e validar autorização de mudanças organizacionais. `BOT_HTTP_ADMIN_TOKEN` opcional é um seam administrativo, não prova identidade do owner nem autoridade hierárquica.
2. **`agents`:** referência para ocupantes que são agentes e leitura de lifecycle/capabilities; posição ocupada não cria identidade nem ativa agente.
3. **Auditoria:** ator, alvo, organização, ação, resultado e timestamp com retenção definida. Pode começar como trilha própria de `org`; não misturar com `agent_identity_events`, que registra apenas lifecycle da identidade.
4. **Persistência central `core::database`:** schema/migrations versionados, transações, constraints/índices, backups e health. O runtime atual aceita PostgreSQL `trading_bot` por `DATABASE_URL`; não apontar o migrator global para `bot_agents`, pois executa migrações de outros domínios.
5. **Política/aprovação de capabilities (fase posterior):** cargo armazena capabilities requeridas/sugeridas e justificativas. Uma concessão efetiva deve permanecer em `agents` e exigir autorização verificável do owner, revisão e evento auditado. Até esse gate, assignment não concede autoridade automaticamente.

Módulos futuros que não são pré-requisito do primeiro `org`: policy engine independente se as regras excederem gates simples, runtime/worker, scheduler, ferramentas/sandbox, memória e canais externos. A pesquisa de capacidades classifica estes como fases posteriores.

## Regras de domínio a formalizar

- Toda entidade e leitura/escrita fica escopada por `OrgId`/agência.
- A árvore de unidades e a hierarquia de posições são acíclicas.
- Supervisor de posição deve pertencer à mesma organização e respeitar faixa/nível compatível; regras de exceção (CEO/owner, posições consultivas) precisam ser confirmadas.
- Cargo pode ser referenciado por muitas posições; cargo ativo é versionado ou imutável após uso. Alterações históricas não podem reescrever o significado de atribuições antigas.
- Uma posição possui zero ou um assignment ativo; encerrar e iniciar nova atribuição deve ser transacional.
- Um agente aposentado ou pausado não deve ser atribuído a uma posição que exige atividade; decidir se agente pausado mantém posição histórica ou se deve ser transferido.
- Deleções devem ser lógicas/terminais quando há atribuições ou auditoria associadas.
- Hierarquia organizacional não é prova de autenticação. Toda mutação valida ator, escopo, ação e estado no servidor.
- Cargo não concede `AgentCapabilities` por si só. Aprovação do owner e mudança explícita de capabilities são eventos separados.

## Modelo conceitual

`Organization 1—N OrgUnit 1—N Position N—1 JobTitle`

`Position 1—N AssignmentHistory`, com no máximo um assignment ativo.

`Assignment.Occupant = HumanPrincipal | AgentId`; a forma concreta do principal humano depende do contrato de identidade. `Position.supervisor_position_id` define governança da posição; não substitui `AgentDefinition.supervisor` até uma migração deliberada. Durante coexistência, deve haver uma única fonte de verdade por relação e uma estratégia explícita para evitar duas árvores divergentes.

## Alternativas

1. **`org` separado (recomendado):** owns cargos, posições, unidades e atribuições; `agents` retains identidade/capabilities. Seams separados e relacionamento por IDs. Benefício: evita misturar identidade com estrutura mutável; custo: integração e possível migração da hierarquia atual.
2. **Expandir `agents`:** adiciona cargos e posições à identidade. Menos wiring imediato, porém `AgentDefinition` vira agregador de identidade, estrutura e histórico; lifecycle do agente e ocupação passam a confundir-se.
3. **Usar só `AgentRole`:** não atende cargos personalizados, posições vazias, ocupantes humanos ou história de designações; serve apenas se o produto quiser uma cadeia fixa sem organograma real.

## Fases sugeridas

- **Fase 0 — decisões bloqueadoras:** identidade humana/owner, semantics de agência/org, governança de posições e autoridade de mudanças; acordo de API/seams; source of truth da hierarquia durante migração.
- **Fase 1 — núcleo org sem grants:** CRUD validado de cargo/unidade/posição; assignments humanos/agentes; uma ocupação ativa; auditoria; consultas de organograma; PostgreSQL com constraints; integração HTTP protegida por auth verificada.
- **Fase 2 — alinhamento agents:** projeção/consulta da relação entre position supervisor e `AgentDefinition`; migração controlada da árvore atual, com validação de ciclos e rollback.
- **Fase 3 — aprovação de capabilities:** políticas propostas por cargo, workflow explícito de approval, updates a `AgentCapabilities`, audit trail, revogação e testes de autoridade.
- **Fase 4 — operação e runtime:** workers, scheduler, tools ou memória apenas em SDDs separados, com threat model e gates próprios.

## Verificação e rollout

Testes unitários devem cobrir constraints do modelo: ciclo em unidades/posições, supervisor cross-org, position sem cargo ativo, duplicate active assignment, ocupante pausado/retirado e histórico de transfer. Testes HTTP devem validar autenticação, isolamento entre organizações e respostas para transitions inválidas. PostgreSQL isolado deve comprovar constraints e transações de transfer: se fechar assignment antigo ou inserir novo falhar, ambos fazem rollback. Testar cold-start/hydration e compatibilidade de IDs de agentes em base descartável; nunca usar `trading_bot` com dados existentes para rollback tests.

Rollout: migration aditiva e schema versionado em base isolada; backfill da hierarquia existente com validação e comparação; leitura dual apenas por período explicitamente limitado; switch de fonte de verdade após reconciliação; rollback para árvore antiga antes de aceitar mutações novas que não possam ser representadas nela. Não migrar ou alterar dados compartilhados sem aprovação operacional.

## Perguntas abertas para fechar SDD

1. Uma organização é sempre a `AgencyId` atual, ou agência e entidade legal/org são conceitos diferentes?
2. Qual identidade estável representa humanos além do owner: principal de autenticação existente ou novo `PersonId`?
3. A árvore administrativa do produto (owner → CEO → níveis) é a mesma árvore de posições, ou são relações separadas?
4. Regras de supervisor: CEO sempre reporta ao owner; quais níveis e exceções podem existir por cargo?
5. Cargo tem versionamento e quais atributos, além de nome/descrição/nível, são necessários para o MVP?
6. Há headcount/orçamento, múltiplas ocupações por posição em casos específicos ou apenas exatamente uma?
7. Um agente pode ocupar múltiplas posições? E uma pessoa humana pode ocupar mais de uma ao mesmo tempo?
8. Quais ações exigem aprovação do owner, e como autenticar/registrar esse owner?
9. Para capabilities: o cargo recomenda grants ou define política; qual processo aprova, revoga e expira grants?
10. A primeira entrega é apenas domínio/Rust+PostgreSQL ou também API administrativa; qual cliente consumirá o organograma?
11. Qual database PostgreSQL será a fonte de verdade e como oferecer ambiente isolado de migração/testes?
12. Quais retenção, expurgo e requisitos de auditoria aplicam-se a histórico de atribuições e decisões?

## Estado

`provisional`: documentação e código confirmam que o módulo `org` ainda não existe e `agents` mantém um enum hierárquico fixo. Esta análise propõe fronteiras e fases; não aprova implementação, schema, endpoints nem concessão de autoridade.
