---
title: SDD — Módulo org para estrutura e governança de agentes
description: Organizações, unidades, cargos, posições, ocupações, autoridade e auditoria integradas a agents e core
tags:
  - sdd
  - backend
  - org
  - agents
  - governance
status: draft
---
# SDD — Módulo `org` para estrutura e governança de agentes

- **Estado:** draft — revisão independente e aprovação do owner pendentes; não autoriza implementação.
- **Escopo:** sistema organizacional completo: unidades, catálogo de cargos, posições, atribuições, delegação, governança e auditoria. Recrutamento/seleção não pertence a `org`.
- **Fontes:** [[research/org-module-capability-analysis]], [[sdd/agents-module-sdd]], [[sdd/agents-pg-registry-sdd]], [[sdd/modules-mvc-convention-sdd]], [[sdd/core-services-integration-sdd]], [[sdd/database-module-integration-sdd]], [[sdd/http-admin-auth-seam-sdd]], [[architecture/module-catalog]].
- **Regra de segurança:** mutações e grants falham fechados até autenticação verificável e bootstrap do owner. Declaração conversacional de que a pessoa é owner não autentica uma requisição.

## 1. Contexto e problema

O produto define governança humana e de agentes. `agents` implementa hoje identidade e ciclo de vida; `AgentRole` é enum fixo e `SupervisorRef` aponta para owner ou agente. Não há unidades, cargos reutilizáveis, posições vagas, pessoas humanas genéricas, histórico de ocupação ou fonte organizacional de supervisão. Expandir `AgentDefinition` misturaria identidade estável com uma estrutura mutável e versionada.

O backend também deve ser mantido por agentes de IA. Portanto ownership, seams, migrations, validações, testes, erros e gates operacionais precisam ser explícitos e reproduzíveis; agentes de IA recebem somente permissões deliberadas, iguais às regras de runtime, sem autoridade implícita do ambiente de desenvolvimento.

A meta do produto é que os ocupantes de IA sejam proativos e autônomos **dentro dos limites definidos pelo cargo/posição e pelas policies aprovadas**. O owner humano de cada agência e o CEO configuram conjuntamente os limites globais; o líder de cada departamento configura escopos subordinados dentro desses limites. Agentes podem iniciar tarefas, analisar e executar ações via ferramentas apenas quando a capability efetiva e a policy server-side permitirem. Nenhum agente pode ampliar seu cargo, policy ou autoridade. Runtime, scheduler e tools são parte do sistema completo, mas têm SDDs e gates próprios; esta especificação de `org` define governança e escopo, não implementa esses mecanismos.

A hierarquia de produto aprovada é Owner → CEO → Level C → Level B → Level A → especialistas/workers. Level C é superior a Level B; Level A define os limites de especialistas/workers. Os enums atuais de `agents` ainda não representam Level C e precisam ser migrados para os nomes canônicos sem renomeação inferida. A precedência organizacional acima é aprovada; o trabalho de implementação deve atualizar modelos, dados, testes e migração juntos.

## 2. Objetivos

1. Modelar organização e unidades subordinadas de forma tenant-safe.
2. Definir cargos reutilizáveis, imutáveis por versão após publicação.
3. Modelar posições concretas e grafo canônico de supervisão organizacional.
4. Associar pessoas humanas e agentes a posições com histórico auditável e consistência transacional.
5. Representar delegação e política de capabilities sem confundir estrutura com autenticação ou autoridade efetiva.
6. Integrar com `agents`, `core::config`, `core::database`, `core::error` e presentation conforme MVC do backend.
7. Migrar a supervisão fixa atual somente após reconciliação, ensaio isolado, autoridade explícita e rollback validado.
8. Manter schema, documentação e verificações compreensíveis e verificáveis por agentes de IA.

## 3. Não objetivos

- Recrutamento, candidatos, seleção, folha, payroll, orçamento e planejamento financeiro de headcount.
- Criar login, armazenar senha ou escolher implicitamente um provedor de identidade.
- Implementar scheduler, runtime de workers, tool gateway, memória, canais ou execução financeira dentro de `org`; essas capacidades fazem parte do sistema completo quando aprovadas, mas são módulos/SDDs separados e sujeitos às policies de `org` e aos próprios gates.
- Conceder capabilities pela mera ocupação de uma posição.
- Cortar a árvore `agents.supervisor` antes do gate explícito de migração.

## 4. Decisões de domínio aprovadas pelo usuário

- Uma `Organization` corresponde a uma `AgencyId` legada (1:1). `OrgId` é o identificador canônico futuro; relação de mapeamento fica preservada.
- Pessoas humanas são referenciadas por `HumanPrincipal { issuer, subject }`, identificador estável fornecido pelo sistema autenticador. O owner é um principal com vínculo de bootstrap à organização.
- `org` será a fonte canônica alvo de supervisão administrativa por posição; `agents` mantém identidade, estado de ciclo de vida e capabilities efetivas.
- Cada posição tem no máximo um assignment ativo; um principal pode ocupar várias posições simultâneas.
- Cargos publicados são imutáveis; mudança semântica gera nova versão. Posições permanecem na versão antiga até atualização explícita.
- Agente pausado pode continuar ocupando posição, mas suas ações continuam proibidas pelo lifecycle de `agents`. Agente aposentado encerra assignments ativos em operação transacional e auditada.
- Posição ocupada não pode ser congelada nem aposentada: encerra-se/transfere-se o assignment primeiro.
- O sistema completo inclui policies versionadas de autonomia/capabilities por nível/cargo. Policy global exige duas aprovações independentes do owner humano e CEO autenticados sobre o mesmo hash/version; qualquer mudança cria versão nova e invalida approvals anteriores. Level C aprova policies de Level B; Level B de Level A; Level A de especialistas/workers, sempre como subconjunto da policy ancestral. Owner/CEO vagos impedem mudança da policy global; superior direto vago impede policy descendente nova/ampliada. Nenhum nível inferior amplia limite ou delega approval.
- Assignment aplica automaticamente grants já aprovados para a policy version exata; assignment e approval são eventos distintos. O schema PostgreSQL `org` é SoT de grants e `revocation_epoch`; `agents` recebe apenas projeção compatível/read model. Runtime consulta `org` diretamente antes de autorizar. Revogação committed no `org` incrementa a epoch e nega imediatamente novas ações; outbox propaga a projeção, sem janela de permissão. Se o runtime não alcançar/validar a epoch no SoT, nega.
- PostgreSQL usa schema dedicado `org` no cluster central e é a fonte autoritativa transacional. Migrations pertencem a `modules/org` e são executadas pelo runner único de `core::database`; papel e grants são de menor privilégio.
- Neo4j é usado no sistema completo como projeção complementar e reconstruível do grafo organizacional, nunca como SoT de escrita, auth, approval ou autorização de ações. `org` usa somente os seams de projeção/query do `core::database`; não importa `neo4rs` nem abre pool próprio.
- Fluxo: comando autenticado → validação/invariantes → transação PG no schema `org` grava SoT + audit + outbox → commit → projector `core::database` faz `MERGE` idempotente no subgrafo `graph_domain=org`; eventos carregam aggregate/version para ordenar e detectar projeção atrasada.
- Rebuild é unidirecional PG→Neo4j e pode substituir somente `graph_domain=org`; dados de `agents`, `bots` e `code` ficam isolados. Neo4j não envia mudanças de volta ao PG.
- Neo4j indisponível: writes/decisões transacionais seguem em PG; outbox acumula e leitura que dependa de traversal retorna estado unavailable/degraded, nunca resultado vazio enganoso. Rebuild/replay restaura projeção antes de marcar `projection_current`.
- PostgreSQL indisponível: mutações, approvals, tasks e autorização que dependam de policy falham fechadas; Neo4j não serve como fallback autoritativo. Readiness separa SoT PG e projeção Neo4j, com status/lag do projector.
- O backend é mantido por agentes de IA por meio de ownership explícito, contratos e invariantes legíveis, testes determinísticos, documentação OpenKnowledge, Graphify atualizado, preflight em operações de alto impacto e revisão independente.

## 5. Fronteiras de ownership

| Domínio | Fonte de verdade |
|---|---|
| `org` | PostgreSQL schema `org` é SoT de Organization/Agency map, OrgUnit, JobTitle/version, Position/supervisor, Assignments, delegações, capability policies/approvals, `org_audit_events` e outbox de projeção. Neo4j recebe subgrafo derivado `graph_domain=org` para traversal/consulta analítica; nunca é SoT de escrita/autorização. |
| `agents` | AgentId, owner/agency legacy durante transição, display identity, lifecycle e eventos próprios de identidade/lifecycle. Recebe grants efetivos de `org` como projection/read model para compatibilidade; não é SoT de grant/policy/epoch nem pode autorizar sem consultar o PostgreSQL SoT de `org`. `org` referencia `AgentId`; não cria agente nem altera lifecycle. |
| Identity provider / authentication | Validação de credenciais/tokens e identidade autenticada `issuer+subject`; nunca inferir identidade de `owner_id` no body. |
| `core::database` | Configuração central, pool/runner, transações, migrações e health/bootstrap compartilhados; sem pool paralelo dentro de `org`. |
| Presentation HTTP | DTOs, autenticação de requests, tradução de erro; handlers delegam casos de uso aos controllers de `org`. |
| Runtime de agents/bots | Execução posterior via SDD de runtime; usa assignment/lifecycle de `agents` como dados do agente, mas o action/tool gateway obtém policy, grant e `revocation_epoch` do PostgreSQL SoT de `org` antes de cada efeito. Cache/read model de `agents` não autoriza sozinho. |

Não manter duas árvores de supervisão graváveis. Até o cutover, `AgentDefinition.supervisor` continua fonte ativa e a árvore de posições é somente estrutural/sem integração de autoridade. Após cutover aprovado, `Position.supervisor_position_id` torna-se fonte canônica e a representação antiga é removida ou read-only derivada.

## 6. Modelo conceitual

```text
PostgreSQL `org` (SoT): Organization 1—N OrgUnit (árvore acíclica)
Organization 1—N JobTitle; JobTitle 1—N JobTitleVersion (publicada imutável)
OrgUnit 1—N Position; Position N—1 JobTitleVersion
Position 0..1 Assignment ativo; AssignmentHistory 1—N por Position
Assignment.Occupant = HumanPrincipal(issuer, subject) | AgentId
Position.supervisor_position_id -> Position da mesma Organization, ou null para topo
JobTitleVersion 0..N CapabilityPolicyVersion (draft → aprovadas → revogadas)
OrgUnit/Position 0..N ScopedDelegation (actor level → bounded child policy)
CapabilityPolicyApproval -> owner + CEO autenticados + versão/hash + decisão + timestamp
Policy lineage: Owner+CEO → Level C → Level B → Level A → Specialist/Worker

Neo4j `graph_domain=org` (projeção reconstruível, read/query only):
(:Organization {org_id})-[:CONTAINS]->(:OrgUnit {unit_id})
(:OrgUnit)-[:CONTAINS]->(:Position {position_id})-[:HAS_TITLE]->(:JobTitleVersion {title_id, version})
(:Position)-[:REPORTS_TO]->(:Position)
(:HumanPrincipal {principal_ref_hash})-[:OCCUPIES]->(:Position)
(:Agent {agent_id, agency_id})-[:OCCUPIES]->(:Position)
A projeção contém apenas IDs estáveis, role/title version, lifecycle/assignment state e timestamps estritamente necessários; sem nome/email, issuer+subject bruto, prompt, capability secrets, approval evidence, tokens ou dados pessoais. `REPORTS_TO`/`OCCUPIES` são vistas derivadas do commit/outbox PG; nenhuma query Neo4j decide autorização ou grants.
```

Tipos de domínio preferidos: `OrgId`, `OrgUnitId`, `JobTitleId`, `JobTitleVersion`, `PositionId`, `AssignmentId`, `HumanPrincipal`, `AgentId`. Identificadores são normalizados/validados; strings de provider não são usadas como autorização sem validação pelo IdP.

## 7. Estados e invariantes

### Organization e OrgUnit

- `organization_agency_map` mantém para sempre uma linha por `OrgId`, com `legacy_agency_id NOT NULL UNIQUE`; `AgencyId` legado não fica nulo nem é removido após cutover. O ID legado permanece como chave de compatibilidade até um SDD separado aprovar sua retirada.
- Cada agência/Organization tem exatamente um usuário humano designado como owner ativo; esse vínculo aponta para `HumanPrincipal(issuer, subject)` autenticado e é único por organização no bootstrap. O owner é a autoridade humana superior. Transferência exige aprovação do owner atual e do novo owner, ambos autenticados, com evento append-only; recuperação excepcional fica em runbook separado e não é autoatendida pelo endpoint comum.
- Limites de autonomia/capabilities são hierárquicos e monotônicos: owner + CEO configuram o limite superior da agência; Level C define limites subordinados para Level B; Level B define limites para Level A; Level A define limites para especialistas e workers. Cada nível pode restringir, nunca ampliar, o limite herdado. Uma policy descendente pode restringir, nunca exceder, o conjunto de ações, ferramentas, recursos, orçamento/quantidade e risco permitido pelo superior. A interseção de policies ancestrais é o limite efetivo aplicado server-side.
- Policy de cargo/posição define capabilities máximas permitidas e limites tipados, não instruções executáveis. O nível superior delega escopo a subordinados autenticados/ativos; o agente não pode alterar a própria policy, aprovar a própria delegação ou elevar limites.
- A policy global de agência requer aprovação explícita do owner autenticado e do CEO autenticado conforme regra conjunta do usuário. Líderes departamentais aprovam/ajustam somente policies dentro dos limites herdados. Versões aprovadas e delegações têm ator, escopo, prazo/estado, hash e audit event.
- A nomenclatura oficial do produto é `Owner → CEO → Level C → Level B → Level A → Specialist/Worker`. O código atual usa `LevelB`/`LevelA` sem `LevelC`; implementação deve adicionar Level C e migrar enum/dados com migration revisada, mantendo identificadores serializados compatíveis ou migration explícita. Não é permitido mapear Level C para Level B por alias sem decisão/migration.
- Policies são efetivas por versão aprovada; cada assignment referencia a versão aplicada. Se policy, posição ou assignment mudar, grants antigos ficam em estado de reconciliação/revogação e não podem ampliar autoridade durante a pendência. Agentes não podem autoaprovar, autoelevar ou alterar os próprios limites.
- Unidades pertencem a uma organização; parent-child não cruza organização e não forma ciclos.
- Encerramento de unidade exige resolver posições ativas filhas; sem cascata destrutiva.

### JobTitle

- Estados de versão: `draft`, `published`, `retired`; conteúdo de versão publicada é imutável.
- Nova revisão é criada como draft, validada e publicada; não reescreve assignments históricos.
- Descrição/responsabilidade são dados não executáveis. Capabilities são policy tipada, sem strings de comando ou prompt.

### Position

- Estados: `planned`, `open`, `filled`, `frozen`, `retired`; `filled` deriva do assignment ativo e não é gravado como verdade independente se puder ser calculado.
- `supervisor_position_id` é nulo apenas para raiz permitida; caso contrário aponta para posição da mesma organização.
- Grafo supervisionado é acíclico. Toda mutação adquire lock transacional da organização antes da validação/leitura da árvore e executa em isolamento `SERIALIZABLE`, com retry limitado somente para serialization failure. Teste concorrente deve provar que duas operações que juntas criariam ciclo não podem ambas confirmar. Restrições de nível/cargo são policy explícita versionada, não comparação de strings.
- Position frozen/retired não aceita nova atribuição; posição ocupada precisa ser desocupada antes de freeze/retire.
- MVP funcional não aplica budget/headcount.

### Assignment

- Um ocupante por posição no máximo, imposto por índice único parcial ou estratégia equivalente no PostgreSQL.
- Ocupante pode ter várias posições ativas.
- Transferência/encerramento é append-only e transacional no schema `org`: fechar assignment antigo, inserir novo, recalcular grants efetivos em `org` e escrever audit + outbox no mesmo commit. Grants têm estado `active`, `revoked` ou `reconciling` e sempre referenciam policy version, origem e idempotency key. Se o mirror em `agents` ou Neo4j estiver atrasado, o gateway consulta grants/epoch autoritativos em `org`; nunca usa um mirror desatualizado para permitir ação. Sem alcançar o SoT `org`, a ação é negada. Replay de projections é ordenado por aggregate/agent e idempotente; revogação compensatória preserva grants ativos de outras origens.
- Não há delete físico de assignment histórico.
- Paused Agent pode continuar alocado; ações são negadas pela verificação de `agents.lifecycle`. Retired Agent não mantém assignment ativo; todas as atribuições são encerradas de forma atômica e com eventos auditáveis.
- Referências HumanPrincipal não implicam acesso à API. Toda operação valida o actor autenticado separadamente.

### Capability policy

- Policy version é ligada a JobTitleVersion e enumera capabilities tipadas, origem, justificativa e limites. Políticas não podem conceder capabilities fora da allowlist/restrições do sistema.
- Estado inicial `draft`; policy global só fica ativa após duas aprovações independentes (owner e CEO autenticados) para o mesmo `OrgId`, hash e versão; cada aprovação individual armazena actor, timestamp e prova de autorização. CEO ou owner ausente/vago deixa a policy sem aprovação completa. Qualquer alteração do envelope global exige nova policy version e ambas as aprovações. Policies subordinadas são aprovadas somente pelo superior direto autenticado (Level C para Level B; Level B para Level A; Level A para especialistas/workers), são subconjunto da ancestral e não podem ampliar limites; delegação interina não aprova policy. Aprovação própria do autor é proibida para toda policy que altere/expanda limites.
- Assignment só aplica automaticamente uma policy já aprovada e não revogada. Mudança de cargo/policy não altera grants existentes silenciosamente; cria diff que requer reaprovação conforme regra explicitada.
- Grant e `revocation_epoch` são autoritativos em `org`. Revoke/expiry invalida imediatamente a autorização, registra causalidade e impede novas ações até confirmação da epoch atual; projeções em `agents` são read models. Revogar uma origem preserva grants válidos de outras origens, calculados no SoT `org`.
- Enquanto owner auth/bootstrap ou integração de capabilities não existir, a feature fica fail-closed e não emite grants.

## 8. Autenticação, autorização e bootstrap

### Enforcement da autonomia hierárquica (runtime futuro)

- `task.start` é capability separada de `tool.invoke`/cada ação; o agente pode decidir iniciar tarefa sem trigger/delegação por tarefa somente se `task.start` estiver no envelope efetivo. Criar task não autoriza suas ferramentas ou efeitos.
- Cada tentativa de iniciar/continuar tarefa ou executar ação consulta identidade autenticada do agente, assignment atual, lifecycle, todas as policy versions/delegações ancestrais, approvals requeridos para cada nível e a `revocation_epoch` global/escopada.
- O limite efetivo é a interseção mais restritiva da cadeia; nenhum descendente concede recurso, tool, ação, orçamento, taxa ou risco ausente no ancestral.
- A autorização é feita server-side em cada ação. Cache deve ser invalidado por policy/assignment/lifecycle change e só pode permitir acesso se confirmar a epoch atual; qualquer indisponibilidade da fonte de policy, versão desconhecida, assignment inconsistente, outbox de revogação pendente ou epoch desatualizada resulta em deny/cancel. Não existe grace period.
- Aprovações globais exigem owner+CEO independentes sobre o mesmo hash; aprovação subordinada vem do superior direto ativo conforme Level C→B→A→Specialist/Worker. Ausência/vacância do aprovador impede ampliar limites.
- Agentes não alteram próprios dados, policies, aprovações ou audit events. Nenhuma delegação interina aprova policies ou subdelega.
- `org` controla policies e delegações; um futuro action/tool gateway executa o check imediatamente antes do efeito, registra decision/event e aplica approval gates para classes de alto impacto. Não aceitar uma policy declarada pelo agente como fonte de autoridade.
- Owner/CEO podem suspender uma policy/delegação e revogar descendentes. No instante do commit da revogação em `org`, a `revocation_epoch` muda e todos os executores devem negar imediatamente novas ações com epoch antiga; indisponibilidade/desatualização da epoch também nega. Outbox propaga a revogação e remove grants espelhados com replay idempotente. O status operacional só declara `revocation_complete` após confirmação de todos os executores/consumidores registrados; esse status tardio nunca posterga o deny inicial.

### Tarefas proativas e execução autônoma (contrato para SDDs de runtime)

- O produto permite que o agente decida iniciar tarefa sem delegação por tarefa ou trigger previamente cadastrado, desde que `task.start` esteja na policy efetiva herdada e que escopo, quotas e estado do agente estejam válidos. Iniciar tarefa não concede tools nem amplia capabilities.
- A criação, planejamento e cada chamada de tool são decisões verificadas separadamente. Cada ação revalida actor/assignment, lifecycle, versão/hash da policy de todos os ancestrais, delegação válida, quotas e revocation epoch no servidor imediatamente antes do efeito.
- Alteração de cargo, assignment, lifecycle ou policy invalida a autorização em cache e provoca revalidação/cancelamento de tarefas em curso. Falha de autenticação, consulta de policy, sincronização, versão desconhecida, outbox pendente de revogação ou ancestry inconsistente resulta em deny/cancel; nenhuma permissividade de fallback.
- Policies limitam allowlist de task/tool/action, recursos/targets, frequência, duração, gasto/orçamento, número de efeitos e risco acumulado. Agentes não podem alterar policies, aprovar sua própria policy, controlar sua identidade nem emitir decisão de approval.
- Owner e CEO aprovam independentemente a policy global sobre mesmo hash/version. Level C aprova a policy de Level B; Level B aprova a de Level A; Level A aprova as policies de especialistas/workers. Cada approval deriva de principal autenticado, superior ativo, escopo da organização, versão e audit event; policy descendente é subconjunto da ancestral. Delegação interina não aprova policy, e vacância do superior bloqueia qualquer policy subordinada nova/ampliada.
- Ações financeiras, pagamentos, produção, mudanças de credenciais/permissões e outras ações high-impact permanecem negadas por padrão, fora da autoridade de `org`. Nenhuma policy de cargo, posição, delegação, aprovação owner+CEO ou capability `task.start` as habilita. Qualquer futuro domínio financeiro/de produção exige SDD próprio, threat model, capabilities específicas e aprovação humana explícita por operação/limite; sem todos esses gates, o runtime deve negar.
- Runtime registra task, policy version/hash, principal/agente, decisões allow/deny, efeitos, timestamps, resultado, revogações e approval IDs; não armazena prompts/PII além do necessário. Falhas/timeout/cancelamento deixam estado final visível e não fazem retry com novos efeitos sem idempotency contract.

Este contrato define a fronteira de segurança, não implementa scheduler, tool gateway ou runtime, que requerem SDDs próprios, threat model e testes de revogação/concurrency.


O usuário declara ser o owner humano; a aplicação só reconhece essa autoridade após validar credencial com o provedor de identidade escolhido e associar o subject autenticado como owner daquela Organization. `BOT_HTTP_ADMIN_TOKEN`, `BOT_HTTP_OWNER_ID` e `BOT_HTTP_AGENCY_ID` são seams administrativos/configuração, não prova do owner.

Gates obrigatórios antes de qualquer write endpoint:

1. escolher/implementar provider adapter de autenticação e validação issuer/audience/expiry/signature;
2. definir bootstrap de primeira organização e vínculo de owner sem endpoint público de autoelevação;
3. actor é sempre derivado de contexto autenticado server-side, nunca aceito do body;
4. validar Organization/Agency binding e autorização da ação a cada request;
5. negar por padrão token ausente, inválido, expired, organization mismatch ou policy não aprovada;
6. auditoria registra actor autenticado, ação, alvo, decisão e correlation ID sem token/segredo.

Reads também têm escopo organizacional; endpoints públicos ou leitura anônima precisam de decisão independente e default privado.

## 9. Seams e estrutura MVC

```text
modules/org/
  mod.rs
  models/       # OrgId, OrgUnit, JobTitleVersion, Position, Assignment, policies, errors
  controllers/  # use cases: create/publish title, create/report position, assign/transfer, approve/revoke policy, queries
  adapters/     # Postgres repository via core pool; IdP principal verifier adapter somente quando provider escolhido
```

- Models sem HTTP, SQLx, tokio ou apresentação.
- Controllers coordenam casos de uso e validações; adapters implementam persistência e integração.
- HTTP handlers em `presentation/http/routes/org.rs`, DTOs OpenAPI e API state/composition root; handlers não contêm regras do domínio.
- A camada usa apenas `core::database` pool/transactions; proibido pool, migration runner, dotenv ou bootstrap duplicado.
- Public service interfaces são pequenas: read/query service; command service; policy approval service; repository ports quando necessários. Evitar abstrações genéricas sem segundo consumidor.
- Entre `org` e `agents`, usar ID tipado/API pública estreita. Sem acesso direto a mapas internos de registry.

## 10. API de domínio

O SDD define categorias; paths/payloads exatos são refinados por schema OpenAPI antes de implementação:

- **Leitura:** organização, árvore de unidades/posições, cargos publicados, posição/ocupante atual, posições abertas, cadeia de supervisão, atribuições/histórico e capability policy status.
- **Administração:** create/update unit, draft/publish/retire job title version, create/update/open/freeze/retire position, assign/unassign/transfer occupant.
- **Delegação interina:** create/revoke delegação temporária para executar somente ações previamente autorizadas no escopo, organização e prazo registrados. Delegação não concede capability, não aprova policy/grant, não altera cargo/posição e não permite subdelegação ou ampliação transitiva. Só o titular ativo com authority aprovada pode delegar execução dentro do envelope já aprovado. Delegação interina não substitui o superior como aprovador de policy subordinada e não habilita approvals de policy/grant. Expiração, revogação do delegante, assignment/lifecycle inválido, epoch desatualizada ou policy ancestral revogada invalida a delegação imediatamente. Se owner/CEO/nível aprovador estiver vago ou indisponível, policy nova/ampliada não publica; approvals administrativos aguardam substituição autenticada e auditada, sem herança automática.
- **Delegação:** endpoints de create/revoke registram delegante, delegado, escopo, allowlist de ações, início/fim, policy version e motivo auditável; runtime verifica a delegação a cada ação.
- **Capability governance:** draft policy, calcular/mostrar diff e hash, aprovação independente owner+CEO para policy global ou superior direto para policy subordinada, rejeição, publish somente após approvals completos, revoke/expire com revocation epoch e outbox, inspect grants efetivos por origem/versão/estado. API registra que owner+CEO aprovaram o mesmo hash/version; approvals nunca são substituídos por token admin.
- **Task/runtime contract (implementado por SDDs separados):** `task.start` e `tool.invoke` são capacidades distintas; GET/read de org não inicia tarefas. Todo gateway de runtime consulta assignments, lifecycle, ancestralidade, approvals e epoch antes de cada efeito; policy unavailable/unknown/pending revoke ⇒ deny/cancel sem retry permissivo.
- Todas as mutations: auth verificada, `OrgId`/agency binding, optimistic concurrency/version token, idempotency para comandos repetíveis, erro tipado `unauthorized/forbidden/conflict/validation/unavailable`. Transfer/cutover/owner/policy global exigem preflight, approvals e correlation IDs; erros não expõem principals brutos nem secrets.
- Transfer/cutover são comandos de alto impacto: preflight/read-only plan mostrando impacto, confirmação owner e audit trail; preflight não reserva nem muta.

Nenhum endpoint de escrita será ativado no código antes de auth/bootstrap e policy gates aprovados. Contrato API final requer teste HTTP contra entrypoint real.

## 11. Persistência PostgreSQL

- Schema PostgreSQL dedicado `org` no database central configurado pelo `core`; migrations SQL ficam versionadas sob ownership do módulo e são registradas/aplicadas exclusivamente pelo runner central.
- Não criar pool independente. Papel da aplicação tem DML necessário; role de migration/admin é separado quando o ambiente suportar. Runtime não recebe superuser nem `CREATE EXTENSION`.
- Tabelas conceituais no schema `org`: `organization_agency_map`, `org_units`, `job_titles`, `job_title_versions`, `positions`, `assignments`, `capability_policy_versions`, `capability_policy_approvals`, `capability_grants` (authority SoT), `org_audit_events`, `org_outbox` (projection/event delivery) e `org_projection_checkpoint`. Todos os grants efetivos e a `revocation_epoch` ficam em `org`; nenhum grant autoritativo depende de outro DB.
- Constraints: FK composta (OrgId + IDs) para tenant integrity, unique active assignment per position, unique job title version, parent/supervisor same-org, checks de states/temporal order, índices para current assignments e subtree traversal.
- Ciclos de árvore e regras multirow são validados em controller dentro de transação com lock/serializable strategy documentada; constraints/indexes fornecem última defesa.
- Transfer e retire+end-assignments atualizam assignment, grants autoritativos, `revocation_epoch`, audit e outbox no mesmo PG schema `org` transaction. Não existe write de grants em outro database nem promessa de atomicidade distribuída. Projeções para `agents` e Neo4j são eventuais, idempotentes e não autorizam; runtime valida `org` SoT a cada ação, falhando fechado se indisponível.
- Testes/integration usam PostgreSQL descartável isolado com mesma versão/extensões necessárias; nunca aplicar migration/teste de rollback ao `trading_bot` compartilhado.
- `DATABASE_URL`/schema/role são configurações validadas por `core::config`; secrets não entram em logs, docs, fixtures ou respostas.

## 12. Auditoria e privacidade

- `org_audit_events` é append-only para atribuição, policy, approval, delegação e cutover. Eventos contêm IDs estáveis, actor principal verificado, action, target, timestamp, correlation/request ID, motivo categorizado e resultado.
- Não copiar nome/email/PII do IdP para `org` sem necessidade explícita. `issuer+subject` é dado restrito: logs operacionais usam identificador pseudonimizado; auditoria armazena a referência mínima protegida. Nenhuma exceção registra valor bruto em logs; acesso administrativo excepcional ao principal requer motivo, approval e evento auditado.
- Leituras de histórico são escopadas e auditadas quando retornam dados pessoais sensíveis.
- Retenção, expurgo, legal hold e exportação são decisão de privacidade/organização e gate de produção. O schema preserva capacidade de pseudonimizar/remover referência humana quando permitido sem reescrever eventos financeiros/organizacionais indevidamente.
- Nunca guardar bearer/token/provider secret em auditoria.

## 13. Integração e migração da hierarquia atual

Fase de coexistência: criar Organization mapping 1:1 com AgencyId e schema aditivo; importar cargos/posições/supervisores a partir de `AgentDefinition.role/supervisor` para fonte candidate; manter `agents.supervisor` autoritativo e bloquear mutações que produzam árvores divergentes.

Cutover só após:

1. migration inventory e ownership aprovados;
2. snapshot/version da árvore antiga em ambiente isolado;
3. validação de cada supervisor, papel/nivel, agência, ciclo, CEO-owner e orphans;
4. relatório de divergências e resolução explícita por owner;
5. rehearsal de migration/cutover/rollback com testes e métricas de reconciliação;
6. approval do owner autenticado e janela operacional aprovada;
7. switch de leitura única e retenção read-only do snapshot para rollback;
8. smoke tests HTTP/API, health, assignments e permissions;
9. rollback se divergência, falha de leitura ou grant inesperado; reabilitar árvore antiga antes de aceitar mutações não representáveis nela.

A instância do app nunca escolhe silenciosamente árvore vencedora nem reconcilia autoridade automaticamente.

## 14. Observabilidade e operação

- Métricas: contagens de org/unit/positions abertas e ocupadas, assignment conflicts, denied mutations por classe (sem principal labels), transações/rollback, migration/cutover duration e drift entre agentes/posições durante coexistência.
- Readiness reporta indisponibilidade do schema `org` quando a feature é habilitada; `liveness` não falha por uma dependency opcional não usada.
- Logs redigidos e audit events correlacionados; alerta em drift, grants efetivos sem approval, duplicidade (deve ser impossível), falhas de outbox e rollback.
- Runbook cobre backup/restore, migration, bootstrap owner, revoke policy, drift e rollback cutover; capacidade/storage e retention precisam de valores definidos no SRE gate.

## 15. Verificação

### Domain/unit

- Organization↔AgencyId 1:1, owner humano único, isolamento em cada use case, ciclos de OrgUnit/Position, supervisor same-org, JobTitle version imutável, posição ocupada não congela/retira, um assignment ativo por posição e múltiplas posições por ocupante.
- Policy global não ativa com uma só assinatura: testes exigem owner+CEO autenticados sobre hash/version idêntico; cargo/assignment não aprova policy; Level C→B→A→specialists/workers só permite policy descendente subconjunto.
- Delegação interina só executa ações preaprovadas, expira/revoga, nunca aprova policy nem subdelega.
- `task.start` permite criar task sem tool; tool.invoke exige policy/capability própria. Alteração de position, policy ou lifecycle invalida autorização e cancela/revalida tasks em curso.
- Revocation epoch bloqueia imediatamente mesmo com outbox atrasada; provider/policy/ancestry indisponível, unknown ou stale ⇒ deny/cancel, sem grace period.

### PostgreSQL isolated

- Migrations fresh/upgrade, check/FK/unique constraints, concurrent assignment conflict, transfer rollback, retire rollback, atomic audit, policy approval/grant linkage, outbox replay/idempotency, cold-start e snapshot consistency.

### HTTP/auth

- Route real; credenciais válidas/inválidas/expiradas; actor vem do contexto autenticado; spoofing de body; owner/org mismatch; read isolation; writes fail-closed antes do auth gate; policies globais exigem as duas aprovações corretas do mesmo hash/version; policy inferior só é aceita pelo superior direto; `task.start` não implica `tool.invoke`; revocation epoch bloqueia novas ações mesmo enquanto outbox está pendente; conflict/concurrency; erros não expõem secrets/principal bruto.
- Teste runtime/gateway separado confirma que alteração de assignment/policy ou lifecycle durante tarefa cancela ou revalida antes do próximo efeito, e indisponibilidade/ancestry desconhecida nega.
- Testes provam que nenhuma policy de cargo, assignment ou delegação habilita finanças/pagamentos/produção sem SDD específico e aprovação humana por operação/limite.

### System and agent maintainability

- `cargo fmt --check`, Clippy `-D warnings`, `verify-backend-gates.sh`, PostgreSQL isolated suite, import-direction check, OpenAPI validation, Graphify update/query and OpenKnowledge links/lint.
- Cada mudança tem teste que chama entrypoint real; nenhuma fonte de env ou texto do source substitui teste comportamental. Cada fatia tem Builder + Critic independente; teste/código do próprio autor não serve de aprovação.

## 16. Rollout e rollback

1. Adicionar schema/migrations aditivas sem ativar writes.
2. Implementar/read-only organization views e comparar mapeamento sem autoridade.
3. Integrar owner auth/bootstrap e habilitar writes com feature gate fail-closed.
4. Habilitar posições/assignments sem capability grants; observar conflitos, drift e auditoria.
5. Habilitar policy approvals e capability application somente após threat model, revoke/recovery e audit review.
6. Cutover da supervisão em gate operacional separado, após rehearsal e owner approval.
7. Runtime/tools permanecem gates/SDDs próprios; nenhuma paridade de produto é implicada.

Rollback: desligar feature gate de writes; reverter versão de app/schema apenas por migration reversível comprovada; preservar audit/history e snapshot; rollback de grant revoga apenas grants originados pela policy revertida, preservando outras origens. Cutover rollback segue §13.

## 17. Perguntas ainda abertas / bloqueios

- Provedor, protocolo e claims de autenticação, audience/issuer, revogação e bootstrap seguro do owner.
- Quem administra aplicação das migrations em cada ambiente, database central/schema alvo, role DML e base PostgreSQL descartável disponível.
- APIs/consumidores concretos, versão de OpenAPI, acesso e política de leitura de histórico.
- Modelo exato de capability catalog, origem/precedência de grants, approval separation, expiração, revogação, emergency break-glass e compatibilidade de consumers em `agents`.
- Forma de outbox/consumer se `org` e `agents` não compartilham a mesma transação PostgreSQL.
- Retenção, export, pseudonymization e legal hold do audit/human principal.
- SLI/SLO, métricas/alertas, runbook, RPO/RTO, janela de cutover e ownership operacional.

Esses bloqueios impedem endpoints de escrita, grants e cutover, não impedem revisão deste SDD conceitual. Antes de implementação mutante, cada item precisa decisão documentada, responsável, critério de verificação e Critic aprovado.

## 18. Critérios de aceitação do design

- Ownership e seams não duplicam identidade/hierarquia/policy.
- Owner autenticado é requisito demonstrável para todas as mutações privilegiadas.
- Nenhum assignment sozinho concede autoridade; grants derivam de policy version aprovada.
- Tenant isolation é reforçado por API, controller e constraints SQL.
- Transfer, retirement, approval/revoke e audit são consistentes e recuperáveis.
- Schema aditivo e cutover são etapas diferentes com rollout/rollback.
- Migrations usam somente core runner e role de privilégio mínimo.
- Cada invariant pode ser provado por teste determinístico em domínio/HTTP/PG isolado.
- Documentação OpenKnowledge e Graphify correspondem ao código e podem orientar um novo agente sem contexto de conversa.
- SDD continua `draft` até revisão Critic e aprovação explícita do owner.
