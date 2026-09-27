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

- **Estado:** draft — revisão Critic independente, threat model do bot Segurança e aprovação do owner pendentes; não autoriza implementação. Nenhum gate está aprovado.
- **Revisão 2026-09-27:** errata [[planning/org-plan-authority-errata]] incorporada (§4, §7.5, §8, §11); dependências P1/P2/P3 explícitas (§14); decisões pendentes do owner (§3); seções sensíveis marcadas `[SEGURANÇA]` (§15). Ciclo Critic 1: separação aprovador×beneficiário e autenticação de principal de agente (§7.5, P3), sem projeção de grants para `agents` (§5), filtro de tenant no traversal (§10), tier aditivo (§3). Ciclo Critic 3: matriz de autorização estrutural (§7.3, F-ORG-01), SEC-ORG-06 e SEC-ORG-21 adotados (§4, §7.4), [threat model do `org`](../security/org-module-threat-model.md) declarado normativo, com SEC-ORG-03/10/14/19 incorporados (§15). Ciclo Critic 4 e veredito do Segurança sobre `905ed5e7` ([threat model](../security/org-module-threat-model.md) `c1d21216`): transfer como unassign+assign, encerramento só com posição vaga e sem descendentes ativos, re-parent de `OrgUnit` (§7.3, F2/F3); assignment pendente inerte com TTL (SEC-ORG-28, F-ORG-18) e regra (c) ligada ao pendente (SEC-ORG-27) (§7.4, §7.5); epoch em mudanças estruturais e autoridade monotônica (SEC-ORG-29, F-ORG-19) (§7.5); aprovação amarrada à posição superior (F1, proposta pendente do SEC-ORG-30) (§7.5, §11, §20); SEC-ORG-02 e SEC-ORG-18 (§8, §10); parâmetros `D-SEC-*` (§3); gates P3-cat/P3-c1/P3-c2/P3-b/P3-a (§14).
- **Escopo:** unidades, catálogo de cargos, posições, atribuições, delegação, policies de autonomia e auditoria. Recrutamento/seleção não pertence a `org`.
- **Plano:** [[planning/org-complete-implementation-plan]] (fatias ORG-S01…S18).
- **Fontes:** [[research/org-module-capability-analysis]], [[sdd/agents-module-sdd]], [[sdd/agents-pg-registry-sdd]], [[sdd/agents-owner-bootstrap-g1-sdd]], [[sdd/http-admin-auth-seam-sdd]], [[sdd/graph-projection-outbox-sdd]], [[sdd/modules-mvc-convention-sdd]], [[sdd/core-services-integration-sdd]], [[planning/unimplemented-modules-analysis]], [[architecture/module-catalog]], [[architecture/unified-neo4j-graph-strategy]].
- **Regra de segurança:** mutações e grants falham fechados até autenticação verificável e bootstrap do owner. Declaração conversacional de que a pessoa é owner não autentica uma requisição.

## 1. Contexto

`agents` implementa identidade e ciclo de vida. No código atual:

- `AgentRole` é enum fixo `Ceo | LevelB | LevelA | Specialist | Worker` (`modules/agents/models/hierarchy.rs`), serializado como `ceo|level_b|level_a|specialist|worker` em Rust, HTTP (`AgentRoleBody`), PostgreSQL (`CHECK` em `0002_agents_bots_scaffold.sql`) e Neo4j (`a.role` em `neo4j_agent_hierarchy.rs`). `validate_hierarchy` exige CEO→owner, LevelB→CEO, LevelA→LevelB, Specialist/Worker→LevelA.
- `SupervisorRef` aponta para `OwnerId` ou `AgentId`; `AgentCapabilities { consult_jev, promote_runtime_bot }` é gravado no próprio `AgentDefinition`.
- O owner "verificado" é `VerifiedProductOwner`: singleton PostgreSQL (`product_owner_bootstrap`, `0010`) criado por env + ACK, identificado por `OwnerId` textual — **não** é identidade humana por IdP nem vínculo por organização.
- A projeção Neo4j usa a outbox central `graph_projection_outbox` (`0009`) com `graph_domain` por subgrafo.

Não há unidades, cargos reutilizáveis, posições vagas, pessoas humanas genéricas, histórico de ocupação nem fonte organizacional de supervisão. Expandir `AgentDefinition` misturaria identidade estável com estrutura mutável e versionada.

Meta do produto: ocupantes de IA proativos e autônomos **dentro dos limites do cargo/posição e das policies aprovadas**. Owner e CEO configuram o limite global; cada superior configura escopos subordinados dentro desse limite. Nenhum agente amplia cargo, policy ou autoridade. Runtime, scheduler e tool gateway têm SDDs próprios (P3); este SDD define governança e escopo.

## 2. Objetivos e não objetivos

**Objetivos:** organização e unidades tenant-safe; cargos versionados e imutáveis após publicação; posições com grafo canônico de supervisão; atribuições de humanos e agentes com histórico auditável e transacional; policies e delegação sem confundir estrutura com autenticação; integração com `agents`, `core::config`, `core::database`, `core::error` e presentation (MVC); migração da supervisão atual somente após reconciliação, ensaio e rollback validados; artefatos verificáveis por agentes de IA.

**Não objetivos:** recrutamento, folha, orçamento/headcount; login, senha ou escolha implícita de IdP; scheduler, runtime, tool gateway, memória, canais ou execução financeira dentro de `org`; conceder capability pela mera ocupação; cortar `agents.supervisor` antes do gate de cutover.

## 3. Decisões pendentes de aprovação do owner

Estas decisões **não** estão aprovadas. O restante do SDD foi escrito para não depender da opção escolhida, exceto onde indicado.

### D-HIER — nível "Level C" (registro de decisão proposto, status: Proposed)

**Contexto.** O rascunho anterior tratava como aprovada a hierarquia `Owner → CEO → Level C → Level B → Level A → Specialist/Worker`. O `AGENTS.md` raiz (normativo) define `proprietário humano → CEO → Level B → Level A → especialistas/trabalhadores`, e o mesmo vale para [[planning/unimplemented-modules-analysis]] §2 e para o código (§1). Não há registro verificável da aprovação de Level C.

**Opções.**

| Opção | Descrição | Custo / risco |
|---|---|---|
| A — Adotar Level C | Novo nível entre CEO e Level B. `AGENTS.md` atualizado depois, via docs sync autorizado; ADR aceito. | Migração serializada em Rust/HTTP/PG `CHECK`/Neo4j/fixtures (ORG-S15); mapear dados existentes; risco de escalonamento se `level_b` for promovido indevidamente. |
| B — Mapear nos níveis existentes | Sem Level C; "líder de departamento" = Level B (ou Level A por unidade). | Sem migração de enum; pode não expressar a estrutura desejada. |
| C — Nível como dado de `org` | Profundidade/tier como atributo versionado de `JobTitleVersion`, sem enum em `org`; `AgentRole` segue como está até o cutover. | Compatível com A ou B; adia a migração de `agents`, mas exige regra explícita de correspondência tier↔`AgentRole` no cutover. |

**Recomendação do autor (não é decisão):** C para as fatias de estrutura, combinada com B até o owner confirmar A. **Consequências:** A exige ADR aceito, atualização do `AGENTS.md` e threat model do mapeamento; B/C mantêm o `AGENTS.md` válido. Regra invariável em qualquer opção: nenhum alias silencioso Level C↔Level B.

**Dependentes:** ORG-S09/S10 (cadeia de aprovação de policies) e ORG-S15/S16 (migração/cutover). ORG-S01…S08 não dependem de D-HIER **em nenhuma opção**: o schema do S04 não tem coluna de nível/tier. Se o owner escolher C, o tier entra de forma aditiva no S09: coluna nova anulável em `job_title_versions` por migration própria e **nova versão de cargo** publicada com o tier; versões já publicadas nunca são reescritas, e versão sem tier não admite policy subordinada até ganhar versão nova.

**ADR:** o `AGENTS.md` pede ADR "quando aplicável", mas o repositório não tem diretório nem formato de ADR. Este registro fica no SDD até o owner decidir; o arquivo ADR vai para um local definido pelo owner.

> [SEGURANÇA] requer threat model do bot Segurança — mapear papéis serializados muda quem aprova policies (escalonamento de privilégio).

### Demais decisões pendentes

- **D-OUTBOX — outbox de `org`.** Padrão proposto: projeção Neo4j via outbox central `graph_projection_outbox` com `graph_domain=org` (seam já existente). Alternativa: tabela `org.org_outbox` própria, somente se P3 exigir ordenação por agente e ack de consumidores para `revocation_complete` que a outbox central não suporte. Em ambos os casos a outbox é escrita na mesma transação PG do SoT.
- **D-OWNER-BIND — owner por organização.** O SDD exige um owner humano por organização (`HumanPrincipal`); o código tem um owner de produto singleton por env. P1 define se o singleton é substituído, se vira o owner da organização legada, ou se coexiste; até lá `VerifiedProductOwner` **não** autoriza mutações de `org`.
- **D-OWNER-POSITION — owner ocupando posições.** O owner pode ocupar posições? Default fail-closed: owner na posição raiz sempre bloqueia a policy global (owner≠CEO), e owner em qualquer outra posição também bloqueia até decisão (§7.5).
- **D-SEAM-AGENTS — import entre módulos.** `org::models` usa `AgentId`, `AgencyId` e `AgentLifecycleState` das reexportações públicas de `modules::agents` (tipos puros, sem I/O), ou os tipos são movidos para `application_contracts` conforme [[sdd/modules-mvc-convention-sdd]] §2.4. Precisa de acordo antes do ORG-S01.
- **`D-SEC-*` — parâmetros de segurança** ([threat model](../security/org-module-threat-model.md) §6.4): `D-SEC-W` (janela checagem→efeito externo), `D-SEC-TTL` (vida do access token humano), `D-SEC-LEEWAY` (tolerância de relógio em `exp`/`nbf`), `D-SEC-AUTHZ-TIMEOUT` (tempo até deny com PG indisponível), `D-SEC-EPOCH-BUMP` (incremento de epoch em restore/PITR) e `D-SEC-PENDING-TTL` (prazo do assignment pendente). Este SDD não fixa números; as sugestões do threat model não são vinculantes.

## 4. Decisões de domínio (propostas no draft; confirmadas somente com a aprovação do owner)

- `Organization` ↔ `AgencyId` legado 1:1; `OrgId` é o ID canônico futuro e o mapeamento é preservado.
- Pessoas humanas: `HumanPrincipal { issuer, subject }`, fornecido pelo autenticador (P1). O owner é um principal com vínculo de bootstrap à organização.
- `org` será a fonte canônica alvo da supervisão administrativa por posição; `agents` mantém identidade, lifecycle e as `AgentCapabilities` atuais.
- No máximo um assignment ativo por posição; um principal pode ocupar várias posições, sujeito à separação de funções de §7.5. **Sem união de autoridade entre posições (SEC-ORG-21):** todo comando e toda ação nomeiam a **posição atuante**, e escopo, aprovações e grants são avaliados somente a partir dela; ação que precise de capabilities de duas posições é negada.
- Cargo publicado é imutável; mudança semântica gera nova versão; posições ficam na versão antiga até atualização explícita.
- Agente pausado pode continuar ocupando posição (ações negadas pelo lifecycle de `agents`); agente aposentado encerra os assignments ativos na mesma transação auditada.
- Posição ocupada não congela nem aposenta: primeiro encerra ou transfere o assignment.
- **Autoridade (errata incorporada):** o schema PostgreSQL `org` é a **única** fonte autoritativa de `capability_grants` e `revocation_epoch`. Neo4j recebe apenas projeção estrutural; **não há projeção de grants para `agents`** (§5). Nenhuma projeção ou cache autoriza ação nem mantém autorização revogada. Não existe grant SoT nem transação de autoridade em outro database. O runtime lê grant, assignment, ancestralidade de policy e epoch atuais no PG `org` antes de cada efeito; sem SoT, nega/cancela.
- PostgreSQL: schema dedicado `org` no cluster central, migrations executadas pelo runner único de `core::database`, roles de menor privilégio.
- Neo4j: projeção complementar e reconstruível `graph_domain=org`, nunca SoT de escrita, auth, approval ou autorização. `org` usa os seams de projeção/query de `core::database`; não importa `neo4rs`.
- Fluxo: comando autenticado → invariantes → transação PG (SoT + audit + outbox) → commit → projector faz `MERGE` idempotente com aggregate/version. Rebuild unidirecional PG→Neo4j, restrito a `graph_domain=org`.
- Neo4j indisponível: writes e decisões seguem em PG; outbox acumula; leituras que exigem traversal retornam `unavailable/degraded`, nunca vazio. PostgreSQL indisponível: mutações, approvals e autorização falham fechadas.

## 5. Fronteiras de ownership

| Domínio | Fonte de verdade |
|---|---|
| `org` | Schema PG `org`: organization↔agency map, OrgUnit, JobTitle/versão, Position/supervisor, Assignment, delegações, policies/approvals, grants, `revocation_epoch`, audit. Neo4j `graph_domain=org` é derivado. |
| `agents` | `AgentId`, agency/owner legados na transição, identidade, lifecycle e eventos próprios. As `AgentCapabilities` atuais (`consult_jev`, `promote_runtime_bot`) continuam SoT em `agents` para os consumidores atuais até uma migração explícita, que exige SDD próprio. **Não existe projeção/read model de grants de `org` em `agents`**: foi removida nesta revisão por não ter consumidor real, e a menção da errata a read model em `agents` está superada. Grants de `org` não leem nem sobrescrevem `AgentCapabilities`; nenhuma capability tem duas fontes. `org` referencia `AgentId`; não cria agente nem altera lifecycle. |
| IdP / autenticação (P1) | Validação de credencial e `issuer+subject`; identidade nunca inferida de `owner_id` no body. |
| `core::database` | Config, pool, transações, runner de migrations, outbox/projeção e health. Sem pool paralelo em `org`. |
| Presentation HTTP | DTOs, autenticação de request, tradução de erro; handlers delegam aos controllers de `org`. |
| Runtime/tool gateway (P3) | Consulta policy, grant e epoch no PG `org` antes de cada efeito; cache/read model não autoriza sozinho. |

Não há duas árvores de supervisão graváveis. Até o cutover, `AgentDefinition.supervisor` é a fonte ativa e a árvore de posições é só estrutural; após o cutover aprovado, `Position.supervisor_position_id` é canônica e a antiga fica read-only derivada ou é removida.

## 6. Modelo conceitual

```text
PostgreSQL `org` (SoT): Organization 1—N OrgUnit (árvore acíclica)
Organization 1—N JobTitle; JobTitle 1—N JobTitleVersion (entidade; número `JobTitleVersionNo`; publicada imutável)
OrgUnit 1—N Position; Position N—1 JobTitleVersion
Position 0..1 Assignment ativo; histórico 1—N por Position
Assignment.Occupant = HumanPrincipal(issuer, subject) | AgentId
Position.supervisor_position_id -> Position da mesma Organization, ou null para raiz
JobTitleVersion 0..N CapabilityPolicyVersion (draft → aprovada → revogada)
OrgUnit/Position 0..N InterimDelegation (execução apenas, com prazo)
CapabilityPolicyApproval -> principal autenticado + versão/hash + posição superior + assignment do aprovador (proposta SEC-ORG-30, §7.5) + decisão + timestamp
Policy lineage: Owner+CEO → níveis subordinados conforme D-HIER → Specialist/Worker

Neo4j `graph_domain=org` (derivado, read/query only):
(:Organization)-[:CONTAINS]->(:OrgUnit)-[:CONTAINS]->(:Position)-[:HAS_TITLE]->(:JobTitleVersion)
(:Position)-[:REPORTS_TO]->(:Position)
(:HumanPrincipal {principal_ref})-[:OCCUPIES]->(:Position)
(:Agent {agent_id, agency_id})-[:OCCUPIES]->(:Position)
```

Todo nó projetado carrega `org_id`, e toda query de traversal filtra pelo `org_id` do actor (`graph_domain=org` é domínio, não tenant). A projeção contém só IDs estáveis, versão de título, estado de assignment/lifecycle e timestamps necessários; sem nome, email, `issuer+subject` bruto, prompts, evidência de approval, tokens ou PII. `principal_ref = HMAC-SHA256(k, issuer‖subject)`, com `k` vindo de `core::config` e guardado fora de PG e Neo4j (SEC-ORG-19); hash sem chave é proibido.

## 7. Estados e invariantes

### 7.1 Organization e OrgUnit

- `organization_agency_map`: uma linha por `OrgId`, `legacy_agency_id NOT NULL UNIQUE`, mantida após o cutover até um SDD próprio aprovar a retirada.
- Unidade pertence a uma organização; parent-child não cruza organização nem forma ciclo; encerrar unidade exige que ela não tenha posições nem unidades descendentes ativas, senão `409 active_descendants` (sem cascata destrutiva; §7.3).

> [SEGURANÇA] requer threat model do bot Segurança — owner único por organização e transferência de owner.

- Cada organização tem exatamente um owner humano ativo (`HumanPrincipal` autenticado). Transferência exige aprovação autenticada do owner atual e do novo, com evento append-only; recuperação excepcional fica em runbook próprio, fora do endpoint comum.

### 7.2 JobTitle

- Estados de versão `draft → published → retired`; conteúdo publicado é imutável; nova revisão nasce draft e não reescreve assignments históricos.
- Descrição e responsabilidades são dados não executáveis; capabilities são policy tipada (§7.5), nunca comandos ou prompts.

### 7.3 Position

- Estados `planned | open | filled | frozen | retired`; `filled` é derivado do assignment ativo.
- Cada organização tem exatamente uma `OrgUnit` raiz e exatamente uma `Position` raiz, a única com `supervisor_position_id` nulo; seu ocupante é o **CEO** para fins de aprovação global. O owner não é posição e fica acima da raiz. As demais posições apontam para posição da mesma organização.
- O grafo é acíclico. Toda mutação da árvore adquire lock transacional da organização e roda em `SERIALIZABLE`, com retry limitado apenas para serialization failure. Um teste concorrente prova que duas edições que juntas formariam ciclo não confirmam ambas.
- `frozen/retired` não aceitam atribuição; posição ocupada não congela nem aposenta (`409 position_occupied`); aposentar (encerrar) posição também exige que não haja posições descendentes ativas (`409 active_descendants`). MVP sem budget/headcount.

> [SEGURANÇA] normativo: F-ORG-01, F-ORG-18, F-ORG-19, SEC-ORG-05, SEC-ORG-06, SEC-ORG-21, SEC-ORG-27, SEC-ORG-28, SEC-ORG-29 do [threat model do `org`](../security/org-module-threat-model.md).

**Matriz de autorização estrutural.** Como assignment aplica grants aprovados e o aprovador deriva da árvore, atribuir ou re-parentar equivale a conceder capability e escolher aprovador; por isso toda mutação estrutural segue esta matriz.

Definições:

- **Posição atuante (A):** posição declarada no comando e ocupada pelo actor com assignment ativo. O owner atua como `Owner`, sem posição, com escopo na organização inteira. Actor agente só com credencial P3; antes disso, apenas humanos.
- **Subárvore estrita de A:** descendentes de A, sem A.
- **Aprovador do nível de T:** ocupante ativo de `parent(T)` atuando a partir dela; para a raiz, o owner.
- **Ocupante ativo:** assignment `confirmed` e não encerrado. Assignment `pending_approver_confirmation` ou `expired` não torna ninguém ocupante ativo em nenhuma regra desta matriz (SEC-ORG-28, §7.4).
- **Escopo de unidades de A:** unidades descendentes estritas de `unit(A)`, a unidade que contém A; o owner alcança todas.

| Operação | Quem pode | Negações obrigatórias (nenhuma linha muda; deny auditado) | Efeito nos grants |
|---|---|---|---|
| Criar unidade ou posição sob X | A com X = A ou X na subárvore estrita de A (unidade: X = `unit(A)` ou no escopo de unidades de A; o nó criado cai no escopo estrito); owner | X fora do escopo → `403 forbidden_scope`; segunda raiz → `409` | posição nasce vaga, sem grants |
| Encerrar unidade U ou aposentar posição T | A com T na subárvore estrita de A (U no escopo de unidades de A); owner | fora do escopo, T = A, T ancestral de A ou U = `unit(A)`/ancestral → `403 forbidden_scope`; raiz nunca encerra → `403 forbidden_scope`; T ocupada, inclusive por assignment pendente → `409 position_occupied`; posição ou unidade descendente ativa → `409 active_descendants` | sem grants a revogar (T vaga, sem descendentes ativos); bump de epoch do escopo (SEC-ORG-29) |
| Abrir/congelar posição T | A com T na subárvore estrita de A; owner | T = A ou T ancestral de A → `403 forbidden_scope`; congelar T ocupada → `409 position_occupied` | congelar exige T vaga |
| Assign/unassign em T não raiz | aprovador do nível de T; owner | T fora da subárvore estrita, par, superior ou raiz → `403 forbidden_scope`; ocupante = actor → `403 self_assignment_forbidden` | assign: ativos no commit só se o actor é o aprovador do nível de T, é elegível por §7.5 (b)(c) e a policy de T tem aprovação vigente do superior direto atual (§7.5); senão (owner, aprovador inelegível, aprovação de outro superior) ficam `reconciling` até confirmação do aprovador elegível (SEC-ORG-06) ou nova aprovação (§7.5). Unassign: grants de T revogados; vacância de T sobe o epoch da subárvore (SEC-ORG-29) |
| Transfer de ocupante de O para T (O, T não raiz) | quem satisfaz **as duas** linhas: unassign em O e assign em T; owner | O ou T fora do escopo do actor → `403 forbidden_scope` (o aprovador de T não puxa ocupante de posição fora da própria subárvore estrita); ocupante = actor → `403 self_assignment_forbidden` | uma transação: unassign em O (grants de O revogados, epoch) + assign em T com a ativação da linha de assign (§7.4) |
| Assign/unassign/transfer envolvendo a raiz (troca do CEO) | somente owner | qualquer outro actor → `403 forbidden_scope`; owner como ocupante → `403 self_assignment_forbidden` (preserva owner≠CEO) | ativos no commit (o owner é o aprovador da raiz); troca do ocupante da raiz é troca do superior das posições filhas (§7.5, SEC-ORG-29) |
| Re-parent de posição T (pai O → N) | A com T na subárvore estrita de A e N = A ou N na subárvore estrita de A | T = A ou T ancestral de A (própria posição ou ancestrais) → `403 forbidden_scope`; N fora do escopo → `403 forbidden_scope`; raiz nunca re-parenta; ciclo → `409 hierarchy_conflict` | alto impacto com preflight; bump de epoch da subárvore de T na mesma transação (SEC-ORG-29); grants de T `reconciling` até o ocupante ativo de N aprovar a policy de T para o par com N (§7.5; comando auditado, que o actor emite se ocupa N e é elegível); policies da subárvore de T revalidadas como subconjunto da nova cadeia, senão `reconciling` |
| Re-parent de `OrgUnit` U (pai O → N) | A com U no escopo de unidades de A e N = `unit(A)` ou N no escopo de unidades de A | U = `unit(A)` ou U ancestral de `unit(A)` → `403 forbidden_scope`; N fora do escopo → `403 forbidden_scope`; unidade raiz nunca re-parenta; ciclo → `409 hierarchy_conflict` | mesmas regras do re-parent de posição: alto impacto com preflight; bump de epoch do escopo de U na mesma transação (SEC-ORG-29); grants das posições de U cujo superior direto ou cadeia de policy mudar ficam `reconciling` até nova aprovação do superior direto atual (§7.5) |
| Trocar a versão de cargo de T | aprovador do nível de T; owner | fora do escopo → `403 forbidden_scope` | bump de epoch da subárvore de T (SEC-ORG-29); grants da versão antiga `reconciling`; a nova só gera grants com policy aprovada pelo superior direto atual de T para o par (versão de policy, `parent(T)`); aprovação feita sob outro superior não vale (§7.5, proposta pendente do SEC-ORG-30) |
| Draft/publish/retire de `JobTitleVersion` | owner; CEO atuando a partir da raiz | qualquer outro → `403` | publicar não aprova policy nem ativa grants |
| Criar a raiz de unidade/posição | somente owner, no bootstrap (S12) | — | — |

Delegação interina não executa operações desta matriz. Em qualquer linha, posição atuante sustentada só por assignment pendente ou expirado → `403 assignment_pending` (SEC-ORG-28). Toda negação muda zero linhas e gera audit event de deny (SEC-ORG-17).

### 7.4 Assignment

- Um ocupante ativo por posição (índice único parcial); um ocupante pode ter várias posições.
- Transferência/encerramento é append-only e transacional: fecha o antigo, insere o novo, recalcula grants em `org`, grava audit + outbox no mesmo commit. Sem delete físico de histórico. **Transfer de O para T = unassign em O + assign em T** numa transação só, cada metade autorizada pela sua linha da matriz (§7.3); se uma metade falha, as duas são desfeitas.
- `HumanPrincipal` em assignment não dá acesso à API; o actor de cada operação é validado separadamente.
- Assign, transfer e unassign seguem a matriz de §7.3: escopo na subárvore estrita da posição atuante, aprovador do nível-alvo, raiz somente pelo owner, sem autoatribuição; transfer exige autoridade na origem e no destino.
- **Decisão SEC-ORG-06 (adotada):** assignment feito por quem não é o aprovador elegível do nível de T é gravado com `confirmation_state = pending_approver_confirmation`; os grants correspondentes ficam `reconciling`, e o gateway nega, até o aprovador elegível confirmar. A confirmação é um comando auditado próprio; vacância do aprovador mantém o bloqueio.
- **Decisão SEC-ORG-28 (interpretação (a) confirmada pelo Segurança, com condições; F-ORG-18):** assignment `pending_approver_confirmation` é inerte. Não confere autoridade estrutural (não conta como assignment ativo da posição atuante nem como ocupante ativo para "aprovador do nível"), aprovação nem delegação; tentativa → `403 assignment_pending`. O confirmador precisa ser o aprovador elegível **e** um principal distinto do autor do assignment. Não há autoconfirmação, nem do owner que também ocupe `parent(T)` → `403 confirmer_not_independent`. O pendente vence em `D-SEC-PENDING-TTL` (§3), medido com `now()` do PG: vencido, já nega na leitura e é encerrado como `expired`, com audit. O owner sempre desfaz ou reatribui, e nenhuma operação de owner depende de confirmação pendente. Com `parent(T)` vago, o owner preenche a cadeia a partir da raiz (`ActiveOnCommit`), sem deadlock.
- **Decisão SEC-ORG-21 (adotada):** autoridade por posição atuante, sem união de grants entre as posições de um principal. Cada comando e cada ação carregam `acting_position_id`; o servidor verifica o assignment ativo do actor nela e avalia escopo e grants apenas dessa posição.

### 7.5 Capability policy e grants

> [SEGURANÇA] requer threat model do bot Segurança — escalonamento de privilégio (aprovações, subset, autoaprovação).

- Policy version liga-se a `JobTitleVersion` e enumera capabilities tipadas, limites, origem e justificativa, sempre dentro da allowlist do sistema. O catálogo tipado é fechado em P3.
- **Regra única de aprovação:** a policy global da organização só fica ativa com duas aprovações independentes, do owner e do CEO (ocupante ativo da `Position` raiz), ambos autenticados, sobre o mesmo `OrgId`, versão e hash. Qualquer mudança cria versão nova e invalida approvals anteriores. Policy subordinada é aprovada somente pelo ocupante ativo da posição superior direta (cadeia conforme D-HIER) e é subconjunto da ancestral; a aprovação vale para o par (versão de policy, posição superior direta), conforme "Vínculo da aprovação" abaixo. O autor não aprova a própria policy e delegação interina não aprova.
- **Separação de funções (aprovador × beneficiário):** (a) owner e CEO precisam ser principais distintos; se o owner ocupa a posição raiz, não há segunda aprovação independente e a policy global fica bloqueada. Até o owner decidir o contrário (§20), owner que ocupe **qualquer** posição também bloqueia a policy global. (b) Um principal, humano ou agente, não aprova policy de posição que ocupa nem de posição cuja subárvore contenha posição que ocupa, porque a interseção ancestral o beneficiaria. **Exceção única:** o CEO (ocupante da `Position` raiz) é elegível **somente** para a policy global da própria organização e **somente** em conjunto com a aprovação do owner distinto exigida em (a); para qualquer policy subordinada, o CEO segue a regra geral. A regra é verificada na aprovação e de novo na publicação. (c) Se um aprovador passar a ocupar posição coberta por policy que aprovou, o assignment nasce `pending_approver_confirmation` (§7.3, §7.4) e os grants dessa policy não se aplicam a ele, nem depois da confirmação, até existir versão nova aprovada por outro aprovador elegível; aprovação da versão nova por ele mesmo → `403` (SEC-ORG-27). (d) Sem aprovador elegível, policy nova ou ampliada fica bloqueada, sem fallback para outro nível nem para token admin; suspensão e revogação (restritivas) continuam permitidas a owner e CEO; CEO agente só com credencial P3 (SEC-ORG-26). A exceção de (b) existe porque a policy global cobre a raiz e o CEO é beneficiário por construção; a mitigação é a aprovação obrigatória do owner distinto.
- **Vínculo da aprovação (F1 do Critic; proposta pendente do SEC-ORG-30, não é decisão final):** a aprovação de policy subordinada fica amarrada a (versão de policy, posição superior `parent(T)`, assignment do ocupante que aprovou), e `capability_policy_approvals` guarda os três (§11). Grants de T só ficam ativos com aprovação vigente para o `parent(T)` atual e para o assignment ativo atual dessa posição. Reuso da `JobTitleVersion` sob outro superior, re-parent que troque `parent(T)`, troca da versão de cargo de T e troca de ocupante ou vacância de `parent(T)` levam os grants afetados a `reconciling`, com bump de epoch na mesma transação (SEC-ORG-29). O gateway nega até o superior direto atual aprovar o par por comando auditado próprio. As regras (b) e (c) são avaliadas contra a posição e o assignment desse aprovador atual, nunca contra os do aprovador original. Confirmação com o Segurança pendente (§20).
- **Aprovador agente (P3):** no código atual o CEO é um agente (`AgentRole::Ceo`), e os superiores diretos costumam ser agentes. Uma aprovação de agente só é aceita quando atribuída verificavelmente a um `AgentId` por autenticação de principal de agente definida em P3: credencial verificada pelo runtime/tool gateway e vinculada a `AgentId`, assignment atual e lifecycle. `AgentId` no body, em header livre ou em campo como `promoted_by` (padrão atual de `/bots/runtime/promote`) não prova autoria. Regra interina: até a liberação da aprovação por agente (depois de P3-a, com credencial P3-c2; §14), aprovações de agente são rejeitadas com erro estável; assim, policy global exige CEO humano distinto do owner, e policy subordinada exige superior humano.
- Limite efetivo = interseção de toda a cadeia ancestral (ações, ferramentas, recursos, quantidade/orçamento, frequência, risco): `efetivo(T) = policy(T) ∩ efetivo(parent(T))`, recalculado a cada decisão (gateway, read API e efeitos internos do `org`) a partir do PG primário (SEC-ORG-29). Nenhum descendente amplia; agente não altera a própria policy; grant `active` antigo não autoriza ação acima da interseção atual. Um property test prova que `efetivo(T) ⊆ efetivo(A)` para todo ancestral A, isto é, que a autoridade nunca cresce por mudança estrutural.
- Assignment aplica automaticamente apenas grants de policy version já aprovada e não revogada; assignment e approval são eventos distintos. Mudança de cargo/policy não altera grants silenciosamente: gera diff que exige reaprovação; grants antigos ficam `reconciling` sem ampliar autoridade.

> [SEGURANÇA] requer threat model do bot Segurança — revogação e `revocation_epoch`.

- Grants têm estado `active | revoked | reconciling`, referenciam policy version, origem, posição atuante e idempotency key com escopo `UNIQUE (org_id, key)` (SEC-ORG-24), e ficam só em PG `org`. Revoke/expiry commitado incrementa `revocation_epoch` e nega imediatamente novas ações; a outbox apenas propaga para as projeções. **SEC-ORG-29 (interpretação (b) confirmada pelo Segurança, com condições; F-ORG-19):** também incrementam, na mesma transação, o epoch (ou versão de cadeia) do escopo afetado: re-parent de posição ou unidade, troca de versão de cargo de T ou de ancestral, nova versão ou revogação de policy de ancestral, e vacância, troca de ocupante ou encerramento de ancestral. Assim, nenhum cache mantém escopo mais largo. Revogar uma origem preserva grants válidos de outras origens. Sem grace period.
- **Janela checagem→efeito (TOCTOU):** "nega imediatamente" vale para toda checagem feita depois do commit da revogação. Um efeito que já passou pela checagem antes do commit não é desfeito por `org`. Efeitos **internos** do `org` (aplicar grant, mutação estrutural) leem o epoch com `FOR SHARE` na mesma transação e não têm janela (SEC-ORG-12). Para efeitos externos, P3-b define como estreitar a janela até o máximo `D-SEC-W` (§3) (checagem imediatamente antes do efeito, efeitos curtos e idempotentes, compensação quando existir) e como auditar efeitos concluídos após o commit.
- **Epoch (SEC-ORG-10, 11, 13, 16):** `org_revocation_epochs(org_id, scope_kind, scope_id, epoch BIGINT NOT NULL)`; o incremento é `UPDATE … SET epoch = epoch + 1 … RETURNING` na mesma transação da revogação ou da mudança estrutural de SEC-ORG-29, e um trigger rejeita decremento. Leitura sempre no PG primário, nunca em réplica ou cache permissivo; PG indisponível ⇒ deny `authz_unavailable` em até `D-SEC-AUTHZ-TIMEOUT` (§3). Revogação ancestral invalida os descendentes da cadeia. Expiração usa `now()` do PG, não o relógio da app.
- **Anti-rollback (SEC-ORG-14):** restore/PITR exige bump do epoch ≥ `D-SEC-EPOCH-BUMP` (§3) e todos os grants `reconciling` antes de reabrir; rollback de migration nunca decrementa nem remove epoch; readiness só fica verde depois do bump auditado.
- Enquanto P1, P3-cat e P3-c1 não estiverem implementados e aprovados, a feature fica fail-closed e não emite grants; nenhum efeito externo consome grants antes de P3-c2 e P3-b (§14).

## 8. Autorização, autonomia e bootstrap

> [SEGURANÇA] requer threat model do bot Segurança — derivação do actor, isolamento de tenant e bootstrap do owner.

**Bootstrap e autenticação (P1).** A aplicação só reconhece o owner depois de validar a credencial no IdP escolhido e associar o subject autenticado como owner daquela organização. `BOT_HTTP_ADMIN_TOKEN`, `BOT_HTTP_OWNER_ID`, `BOT_HTTP_AGENCY_ID` e `VerifiedProductOwner` são seams administrativos, não prova do owner. Antes de qualquer write endpoint:

1. adapter de autenticação com validação de issuer/audience/expiry/signature;
2. bootstrap da primeira organização e do owner sem endpoint público de autoelevação;
3. actor sempre derivado do contexto autenticado no servidor, nunca do body;
4. binding Organization/Agency e autorização verificados a cada request; a camada de auth produz o escopo tipado `AuthorizedOrgScope`, único construível fora de testes por ela, exigido por todo método de repositório (SEC-ORG-03); org alheia e org inexistente recebem a mesma resposta `404 org_not_found`, sem ler linhas da outra org e com audit `org.access_denied` (SEC-ORG-02);
5. deny por padrão (token ausente, inválido, expirado, organização divergente, policy não aprovada);
6. audit com actor, ação, alvo, decisão e correlation ID, sem token ou segredo; `Debug`/`Display` de contexto de auth, principal e erros redigidos, e falha de PG → `503 org_store_unavailable` sem texto do driver (SEC-ORG-18).

Leituras também têm escopo organizacional; leitura pública exige decisão própria, com padrão privado.

**Contrato para o runtime (implementado nos SDDs de P3-a e P3-b, não em `org`).**

- `task.start` é capability separada de `tool.invoke` e de cada ação; iniciar tarefa não autoriza ferramentas nem efeitos.
- Cada início/continuação de tarefa e cada efeito revalida no servidor, imediatamente antes: identidade do agente autenticada conforme P3, assignment, lifecycle, versões/hash da cadeia de policies e limite efetivo recalculado (SEC-ORG-29), delegação válida, quotas e `revocation_epoch` lidos do PG `org`.
- Mudança de cargo, assignment, lifecycle ou policy invalida cache e força revalidação ou cancelamento de tarefas em curso. SoT indisponível, versão desconhecida, ancestralidade inconsistente, revogação pendente ou epoch desatualizada ⇒ deny/cancel.
- `revocation_complete` só é declarado após ack de todos os consumidores registrados; esse status tardio nunca adia o deny inicial.
- Ações financeiras, pagamentos, produção e mudança de credenciais/permissões permanecem negadas por padrão e fora da autoridade de `org`; nenhuma policy, delegação, aprovação owner+CEO ou `task.start` as habilita. Exigem SDD, threat model e aprovação humana por operação/limite próprios.
- O runtime registra task, policy version/hash, principal/agente, decisões allow/deny, efeitos e approval IDs, sem prompts/PII além do necessário; falha, timeout ou cancelamento deixam estado final visível, e não há retry com novo efeito sem contrato de idempotência.

## 9. Seams públicos e estrutura MVC

```text
modules/org/
  mod.rs
  models/       # IDs, OrgUnit, JobTitleVersion, Position, Assignment, OccupantRef, erros; policies depois de P3
  controllers/  # casos de uso: estrutura, assignment, policy/approval, delegação, consultas
  adapters/     # repositório Postgres via pool do core; outbox/projeção via core
```

- Models sem HTTP, SQLx, tokio ou apresentação. Controllers coordenam casos de uso e transações; adapters fazem persistência e integração.
- HTTP em `presentation/http/routes/org.rs` e DTOs OpenAPI, com acesso via `ApiState`/bridge; rotas não importam `modules::org::`. Hoje `check-import-direction.sh` só bloqueia `modules::agents::` em rotas, então o S01 acrescenta a regra equivalente para `modules::org::`. A prova é do Builder e reproduzível: autoteste do script contra um fixture com import proibido (script aceitando `SRC` alternativo), com a saída `FAIL` registrada na entrega.
- Só `core::database` para pool, transações, migrations e outbox; proibido pool, runner, dotenv ou bootstrap duplicados.
- Serviços pequenos: query service, command service, policy approval service; ports de repositório só quando necessários.
- Com `agents`: somente IDs tipados e API pública estreita (D-SEAM-AGENTS).

**Seams que o owner deve acordar antes do TDD** (detalhados por fatia no plano): tipos de ID (incluindo `JobTitleVersionNo`, número monotônico da entidade `JobTitleVersion`) e regras de normalização; raiz única de unidade e de posição; `OccupantRef`; `ActingContext::{Owner, Position(PositionId)}`; `StructuralOp` e a função pura da matriz de §7.3, que devolve a decisão de ativação (`ActiveOnCommit | PendingApproverConfirmation | NoGrants`); `AuthorizedOrgScope`; `OrgDomainError`; state machines de JobTitleVersion/Position/Assignment; assinaturas de validadores puros; ports de repositório; forma dos comandos (actor, expected version, idempotency key, reason code); contrato de erro HTTP; payload de projeção; esquema de policy/grant e chave de aprovação `(policy_version, superior_position, approver_assignment)` (proposta SEC-ORG-30; após P3-cat).

## 10. API de domínio

Categorias; paths e payloads exatos saem do OpenAPI antes de implementar cada fatia HTTP:

- **Leitura:** organização, árvore de unidades/posições, cargos publicados, ocupante atual, posições abertas, cadeia de supervisão, histórico de atribuições, status de policy. Toda leitura, inclusive traversal Neo4j, é filtrada pelo `org_id` do actor autenticado; teste de traversal cross-org obrigatório. Org alheia ou inexistente → `404 org_not_found` uniforme (SEC-ORG-02).
- **Administração:** create/close/re-parent de unidade; draft/publish/retire de versão de cargo; create/open/freeze/retire (encerrar)/re-parent de posição; troca de versão de cargo; assign/unassign, transfer (unassign na origem + assign no destino, numa transação) e confirmação de assignment pendente. Autorização exclusivamente pela matriz de §7.3 (SEC-ORG-05, 06, 21, 28); negações com `403 forbidden_scope`, `403 self_assignment_forbidden`, `403 assignment_pending` (posição atuante só com assignment pendente ou expirado) ou `403 confirmer_not_independent`; conflitos com `409 position_occupied`, `409 active_descendants` ou `409 hierarchy_conflict`. Toda negação sai sem mudança de linhas e com audit de deny.

> [SEGURANÇA] requer threat model do bot Segurança — delegação de autoridade.

- **Delegação interina:** o titular ativo delega a **execução** de ações já autorizadas, com escopo, organização e prazo registrados. Não concede capability, não aprova policy/grant, não altera cargo/posição, não subdelega nem amplia transitivamente. Expiração, revogação do delegante, assignment/lifecycle inválido, epoch desatualizada ou policy ancestral revogada a invalidam imediatamente. O runtime verifica a delegação a cada ação.
- **Governança de capabilities:** draft, diff/hash, approvals (§7.5), reaprovação do par (versão de policy, superior direto atual) após reuso, re-parent ou troca de superior (proposta SEC-ORG-30), rejeição, publish após approvals completos, revoke/expire com epoch, inspeção de grants por origem/versão/estado. Token admin nunca substitui approvals.
- **Todas as mutações:** actor autenticado, `acting_position_id` explícito (exceto owner), DTOs com `deny_unknown_fields` e sem campos de autoridade (SEC-ORG-01), binding `OrgId`/agency, token de versão otimista, idempotency em comandos repetíveis, erros tipados `unauthorized | forbidden | conflict | validation | unavailable` sem principal bruto nem segredo, e `Debug` redigido (SEC-ORG-18). Transfer, cutover, owner e policy global exigem preflight read-only (sem reserva nem mutação), confirmação e correlation ID.

Nenhum endpoint de escrita é ativado antes dos gates aplicáveis (§14). O contrato final exige teste HTTP contra o entrypoint real.

## 11. Persistência PostgreSQL

- Schema `org` no database central configurado pelo `core`; migrations versionadas em `backend/src/core/database/migrations/` (próxima sequência livre no momento do S04; `0011` já é `0011_monitor_supervisor_snapshot.sql`), aplicadas somente pelo runner central.
- Todo método de repositório recebe `AuthorizedOrgScope` e filtra por `org_id`; cada SELECT tem teste com duas organizações (SEC-ORG-03). RLS com `SET LOCAL app.org_id` é opcional.
- Tabelas conceituais: `organization_agency_map`, `org_owner_bindings`, `org_units`, `job_titles`, `job_title_versions`, `positions`, `assignments`, `capability_policy_versions`, `capability_policy_approvals`, `capability_grants`, `org_revocation_epochs` (§7.5, SEC-ORG-10), `interim_delegations`, `org_audit_events`; outbox conforme D-OUTBOX. `assignments` inclui `acting_position_id` do autor, o autor, `confirmation_state` (`pending_approver_confirmation | confirmed | expired`), o confirmador (distinto do autor) e o vencimento do pendente (SEC-ORG-06, SEC-ORG-28). `capability_policy_approvals` inclui versão e hash da policy, `superior_position_id` (nulo só na policy global) e `approver_assignment_id` (proposta SEC-ORG-30, §7.5); aprovação cujo par não corresponde ao `parent(T)` e ao assignment ativo atuais não ativa grants.
- Constraints: FKs compostas (`OrgId` + ID) para integridade de tenant; assignment ativo único por posição; versão de cargo única e imutável depois de publicada; parent/supervisor na mesma organização; checks de estado e ordem temporal; índices para assignments atuais e traversal.
- Ciclos e regras multi-linha são validados no controller dentro da transação (lock + `SERIALIZABLE`); constraints são a última defesa.
- Transfer (unassign na origem + assign no destino), retire+encerramento de assignments e as mudanças estruturais de SEC-ORG-29 atualizam assignment/estrutura, grants, epoch, audit e outbox em uma transação do schema `org`, sem atomicidade distribuída.

> [SEGURANÇA] requer threat model do bot Segurança — roles de banco de menor privilégio.

- Role de runtime só com DML necessário; role de migration separada quando o ambiente suportar; sem superuser nem `CREATE EXTENSION` no runtime (P2). Audit append-only por privilégio: em `org_audit_events` a role de runtime só tem `INSERT`/`SELECT`, sem `UPDATE`/`DELETE`/`TRUNCATE`, e um teste com essa role prova que alterar ou remover falha.
- Testes de integração usam PostgreSQL descartável e isolado; nunca aplicar migration ou rollback de teste ao `trading_bot` compartilhado. `DATABASE_URL`, schema e role vêm de `core::config` validado; segredos não entram em logs, docs, fixtures ou respostas.

## 12. Auditoria e privacidade

> [SEGURANÇA] requer threat model do bot Segurança — PII de principais humanos e integridade da auditoria.

- `org_audit_events` é append-only para atribuição, policy, approval, delegação, owner e cutover: IDs estáveis, actor verificado, ação, alvo, timestamp, correlation ID, motivo categorizado e resultado.
- Audit cobre allow **e** deny de mutações, aprovações e revogações (SEC-ORG-17).
- Sem copiar nome/email do IdP. `issuer+subject` é dado restrito: logs, audit, outbox e projeção usam somente `principal_ref` (HMAC com chave fora dos bancos, SEC-ORG-19); acesso excepcional ao valor bruto exige motivo, approval e evento auditado. Nunca guardar bearer, token ou segredo.
- Leituras de histórico com dados pessoais são escopadas e auditadas. Retenção, expurgo, legal hold e exportação são gate de produção; o schema permite pseudonimizar a referência humana sem reescrever eventos.

## 13. Coexistência e cutover da hierarquia atual

> [SEGURANÇA] requer threat model do bot Segurança — reconciliação de autoridade no cutover.

Coexistência: mapping 1:1 com `AgencyId`, schema aditivo, importação candidata de posições/supervisores a partir de `AgentDefinition.role/supervisor`; `agents.supervisor` segue autoritativo e mutações que gerem árvores divergentes são bloqueadas.

Cutover só depois de: (1) inventário de migrations e ownership aprovado; (2) snapshot versionado da árvore antiga em ambiente isolado; (3) validação de supervisor, papel/nível (D-HIER), agência, ciclo, CEO-owner e órfãos; (4) relatório de divergências resolvido explicitamente pelo owner; (5) rehearsal de migração/cutover/rollback com métricas de reconciliação; (6) aprovação do owner autenticado sobre o hash do preflight e janela operacional; (7) troca para leitura única com snapshot read-only para rollback; (8) smoke HTTP, health, assignments e permissões; (9) rollback em caso de divergência, falha de leitura ou grant inesperado, reabilitando a árvore antiga antes de aceitar mutações não representáveis nela. A aplicação nunca escolhe a árvore vencedora nem reconcilia autoridade automaticamente.

## 14. Dependências externas (P1, P2, P3)

Os IDs P1/P2/P3 são gates, não prioridades. Cada gate bloqueia apenas o que depende dele; o mapeamento por fatia está no plano.

| Gate | Conteúdo | Partes deste SDD que dependem |
|---|---|---|
| **P1** — autenticação do owner / IdP | Somente principais **humanos**: provider, issuer/audience, assinatura/rotação, expiry/revogação, mapeamento para `HumanPrincipal`, bootstrap por organização (D-OWNER-BIND); session epoch por principal checado a cada request, TTL ≤ `D-SEC-TTL` e leeway ≤ `D-SEC-LEEWAY` (§3), logout-all e revogação na transferência de owner (F-ORG-02, SEC-ORG-15); identidade por pessoa para o CEO humano distinto do owner (F-ORG-06); owner só por enrollment (SEC-ORG-23). | §7.1 owner/transferência; §8 bootstrap; §10 todas as mutações e leituras privadas; aprovações humanas (§7.5); cutover (§13). P1 **aprovado** é pré-requisito de ORG-S07, S10, S12 e S13. |
| **P2** — roles de DB / migrations / PG isolado | Schema `org`, role DML vs migration (audit sem `UPDATE`/`DELETE`), runner central, PG descartável com versão alinhada (código exige PG 18; divergência da CI `pg16` tratada na tarefa T-CI-02). | §11 inteiro; §7.3 concorrência; §7.4 transações; outbox (D-OUTBOX); projeção Neo4j; cutover. |
| **P3-cat** — catálogo de capabilities | Esquema tipado e fechado de capability/ação/recurso/limite, sem enforcement; protocolo de approval. | §7.5 policies (ORG-S09–S11). |
| **P3-c1** — credencial básica de agente (depois de P1) | Credencial emitida pelo servidor, vinculada a `AgentId` + lifecycle, com revogação imediata; ainda sem efeito em aprovações. Regra interina: só humanos aprovam. | ORG-S09–S11, com aprovação só humana (§7.5). |
| **P3-c2** — credencial vinculada a assignment e epoch (depois de P3-c1, ORG-S05/S06 e S11) | **Autenticação de principal de agente** completa: credencial ligada a `AgentId`, assignment atual, lifecycle e `revocation_epoch`; única prova aceita em aprovações, revogações (SEC-ORG-26) e ações de agente. | §7.5 aprovações e revogações por agente; §8. |
| **P3-b** — tool gateway (depois de P3-c2) | Checagem de grant/epoch antes de cada efeito; janela TOCTOU (`D-SEC-W`); outbox/ack de revogação; deny de alto impacto. | §7.5 TOCTOU; §8 contrato de runtime. |
| **P3-a** — runtime/scheduler (depois de P3-c2 e P3-b) | `task.start`/`tool.invoke`; cancelamento e revalidação. Libera ORG-S14, S17 e a aprovação por agente em S10. | §8; delegação interina (§10); aprovação por agente (§7.5). |
| **D-HIER** (decisão do owner) | §3. | Cadeia de approvals (§7.5), migração de papéis e cutover (§13). |

A divisão de P3 segue [[planning/master-plan]] §3.19–§3.21 e §4.4. "P3" sem sufixo designa o pacote; a credencial de agente aceita em aprovações e revogações é a do P3-c2, e a aprovação por agente só é liberada depois de P3-a.

Não dependem de P1/P2/P3 nem de D-HIER (em qualquer opção, ver §3): tipos de domínio e invariantes puros de estrutura e assignment (§7.1 exceto owner, §7.2, §7.3 exceto concorrência PG, §7.4 exceto transação/grants), incluindo a matriz de §7.3 e a decisão de ativação como funções puras sobre snapshot em memória.

## 15. Seções sensíveis à segurança (threat model normativo)

O [threat model do `org`](../security/org-module-threat-model.md) é **normativo** para `org`: cada SEC-ORG é critério de aceite da fatia mapeada na tabela de rastreabilidade do plano; em divergência, prevalece o threat model até revisão conjunta. A versão lida é `c1d21216`, que já traz o rebase do SEC-ORG-08 com a exceção única do CEO e o veredito de segurança sobre `905ed5e7` (nenhum Crítico/Alto aberto no design; F-ORG-18/19, Médios, incorporados nesta revisão). O threat model em si continua aguardando o Critic independente, e mudanças nele se propagam a este SDD e ao plano. Os marcadores `[SEGURANÇA]` apontam para ele.

| # | Tema | Onde |
|---|---|---|
| 1 | Escalonamento de privilégio: matriz de autorização estrutural (subárvore estrita, aprovador do nível-alvo, raiz só pelo owner, sem autoatribuição, sem re-parent da própria posição/ancestrais, publicação de cargo só owner/CEO; SEC-ORG-05); assignment por não aprovador fica `reconciling` (SEC-ORG-06); posição atuante sem união (SEC-ORG-21); approvals, subset monotônico, autoaprovação, separação aprovador×beneficiário (owner≠CEO; sem aprovar policy da própria posição ou subárvore; sem aprovador elegível ⇒ bloqueio; SEC-ORG-08/09); pendente inerte, confirmador distinto e TTL (SEC-ORG-28); regra (c) ligada ao pendente (SEC-ORG-27); aprovação amarrada à posição superior (proposta SEC-ORG-30) | §4, §7.3, §7.4, §7.5, §10, §11 |
| 2 | Revogação e `revocation_epoch` sem grace period; tabela e monotonicidade (SEC-ORG-10); anti-rollback em restore (SEC-ORG-14); epoch em mudanças estruturais e autoridade monotônica (SEC-ORG-29) | §7.3, §7.5, §8, §17, §19 |
| 3 | Isolamento de tenant (FKs compostas, `AuthorizedOrgScope` em todo repositório com SEC-ORG-03, binding por request, `404` uniforme de SEC-ORG-02, traversal Neo4j filtrado por `org_id`) | §6, §8, §10, §11 |
| 4 | Delegação de autoridade interina | §10 |
| 5 | Bootstrap, unicidade e transferência de owner; derivação do actor | §7.1, §8 |
| 6 | Mapeamento de papéis / Level C (quem passa a aprovar) | §3 D-HIER, §13 |
| 7 | Reconciliação de autoridade no cutover | §13 |
| 8 | Roles de banco de menor privilégio | §11 |
| 9 | PII de principais (`principal_ref` HMAC, SEC-ORG-19), integridade de audit, vazamento via projeção Neo4j, `Debug` e erros redigidos (SEC-ORG-18) | §6, §8, §10, §12 |
| 10 | Duas fontes de capability: evitada, sem projeção de grants para `agents` (`AgentCapabilities` segue SoT própria) | §5 |
| 11 | Ações de alto impacto sempre negadas fora de SDD próprio | §8 |
| 12 | Autenticação de principal de agente (atribuição verificável de aprovações e ações a `AgentId`) | §7.5, §14 (P3) |
| 13 | Janela TOCTOU entre checagem de epoch e efeito | §7.5 |

Os achados de código do threat model (F-ORG-03, 10–13, 15, 16) ficam como débito dos seams existentes; `org` não reutiliza esses seams.

## 16. Riscos principais

| Risco | Mitigação |
|---|---|
| Ciclo por write skew concorrente | Lock por organização + `SERIALIZABLE` + teste concorrente (§7.3). |
| Projeção atrasada usada para autorizar | SoT PG `org` consultado por efeito; deny em qualquer incerteza (§4, §8). |
| Árvores de supervisão divergentes | Uma árvore gravável; cutover com gate (§5, §13). |
| Level C decidido tarde e já embutido no código | D-HIER fora das fatias S01–S08 em qualquer opção; tier aditivo no S09 (§3). |
| Quem atribui ou re-parenta concede capability e escolhe aprovador (F-ORG-01) | Matriz de §7.3; ativação só pelo aprovador elegível (SEC-ORG-06). |
| Mesmo principal aprova e se beneficia | Separação de funções (§7.5); aprovação de agente só com credencial de P3. |
| Aprovação herdada por reuso de cargo, re-parent ou sucessão no superior (F1) | Aprovação amarrada a versão de policy, posição superior e assignment do aprovador; grants `reconciling` até nova aprovação (proposta SEC-ORG-30, §7.5). |
| Cache mantém escopo mais largo após mudança estrutural (F-ORG-19) | Bump de epoch na mesma transação e limite efetivo recalculado na decisão (SEC-ORG-29, §7.5). |
| Owner singleton confundido com owner por organização | D-OWNER-BIND em P1; `VerifiedProductOwner` não autoriza `org` (§3, §8). |
| Outbox duplicada ou inconsistente com a central | D-OUTBOX; reutilizar a outbox central por padrão (§3). |

## 17. Observabilidade e operação

- Métricas: posições abertas/ocupadas, conflitos de assignment, mutações negadas por classe (sem label de principal), rollbacks, idade/lag da outbox, drift agentes×posições na coexistência, duração de cutover.
- Readiness separa o SoT PG `org` (quando a feature está habilitada) da projeção Neo4j (status/lag); liveness não falha por dependência opcional.
- Alertas: drift, grant efetivo sem approval, falha de outbox, rollback. Runbook: backup/restore com bump de epoch e grants `reconciling` antes de reabrir (SEC-ORG-14), migration, bootstrap/recuperação de owner, revoke, replay, rollback de cutover. SLI/SLO e retenção no gate SRE.

## 18. Validação

- **Domínio (sem DB/auth/runtime):** mapping `OrgId`↔`AgencyId`, raiz única de unidade e de posição, ciclos de OrgUnit/Position, supervisor na mesma organização, imutabilidade de versão publicada, posição ocupada não congela/aposenta, um assignment ativo por posição, múltiplas posições por ocupante, semântica de agente pausado/aposentado recebida como valor, matriz de §7.3 (um positivo e um negativo por linha, incluindo re-parent de `OrgUnit`, encerramento com ocupante ou descendentes ativos e transfer sem autoridade na origem), assignment pendente inerte e expirado (SEC-ORG-28), property test de monotonicidade da autoridade efetiva (SEC-ORG-29) e ausência de união entre posições.
- **Policy (após P3):** uma só assinatura não ativa policy global; hash/versão divergentes; aprovador errado, vago ou o próprio autor; owner ocupando a posição raiz; aprovador que ocupa a posição-alvo ou posição da subárvore; ausência de aprovador elegível bloqueia; aprovação de agente sem credencial P3 ou com `AgentId` no body é rejeitada; subset descendente; aprovação feita sob outro superior ou por ocupante anterior de `parent(T)` não ativa grants (proposta SEC-ORG-30); aprovador que passa a ocupar posição coberta pela própria policy (regra (c), SEC-ORG-27); delegação interina não aprova nem subdelega; epoch bloqueia com outbox atrasada.
- **PostgreSQL isolado (P2):** migrations fresh/upgrade, FK/check/unique, assignment concorrente, rollback de transfer e retire, audit atômico, `UPDATE`/`DELETE` em audit negado à role de runtime, replay idempotente da outbox, cold start.
- **HTTP/auth (P1):** rota real; credenciais válidas/inválidas/expiradas; actor do contexto; spoofing de body; organização divergente; isolamento de leitura, inclusive traversal cross-org negado; writes fail-closed antes do gate; erros sem segredo ou principal bruto.
- **Runtime (P3, suíte separada):** mudança de assignment/policy/lifecycle cancela ou revalida antes do próximo efeito; alto impacto negado.
- **Gates do repositório:** `./scripts/verify-backend-gates.sh` (`cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `check-import-direction.sh`, testes do bin `bot` e integração), suíte PG isolada, OpenAPI, Graphify e links OpenKnowledge. Cada fatia tem Builder e Critic independentes.

## 19. Rollout e rollback

1. Tipos e invariantes puros (sem efeito em runtime).
2. Schema/migrations aditivas sem writes habilitados.
3. Views read-only autenticadas e comparação de mapeamento, sem autoridade.
4. Owner auth/bootstrap; writes de estrutura/assignment atrás de feature gate fail-closed, sem grants.
5. Policies e grants somente depois de P3-cat e P3-c1 (aprovação só humana), threat model e revisão de revoke/recovery; aprovação por agente só depois de P3-a (§14).
6. Cutover da supervisão em gate operacional próprio.
7. Runtime/tools seguem seus SDDs; nenhuma paridade de produto está implícita.

Rollback: desligar o feature gate de writes; nunca decrementar nem remover epoch (SEC-ORG-14); reverter app/schema apenas por migration reversível comprovada; preservar audit, histórico e snapshot; rollback de grant revoga só grants da policy revertida. Cutover segue §13.

## 20. Perguntas abertas / bloqueios

- D-HIER, D-OUTBOX, D-OWNER-BIND, D-OWNER-POSITION e D-SEAM-AGENTS (§3).
- P3-c1/P3-c2: mecanismo de credencial do principal de agente (emissão, rotação, revogação, vínculo com assignment/lifecycle).
- P1: provedor, protocolo, claims, revogação por request (SEC-ORG-15) e bootstrap seguro.
- Assignment feito pelo owner em posição não raiz fica pendente de confirmação do aprovador do nível (SEC-ORG-06); confirmar se o owner aceita essa fricção ou prefere que sua assinatura conte como confirmação (exigiria emenda ao threat model). O Segurança confirmou a interpretação (a) com as condições de SEC-ORG-28 (§7.4).
- **F1 / SEC-ORG-30 (a confirmar com o Segurança, porque o threat model é normativo):** o vínculo da aprovação a (versão de policy, posição superior, assignment do aprovador) e os gatilhos de `reconciling` de §7.5 (reuso sob outro superior, re-parent, troca de versão de cargo, troca de ocupante ou vacância do superior) são proposta até o Segurança publicar o SEC-ORG-30. O LGTM de ORG-S03 na linha de troca de versão de cargo e o G1 de ORG-S09/S10 dependem dessa confirmação.
- `D-SEC-*` (§3): os valores de `D-SEC-W`, `D-SEC-TTL`, `D-SEC-LEEWAY`, `D-SEC-AUTHZ-TIMEOUT`, `D-SEC-EPOCH-BUMP` e `D-SEC-PENDING-TTL` são decisão do owner (threat model §6.4).
- P2: quem aplica migrations por ambiente, role DML e PG descartável; versão PG (18 × `pg16` na CI) segue na tarefa T-CI-02.
- P3-cat/P3-b: catálogo de capabilities, precedência de origens, expiração, break-glass, consumidores e ack de revogação; quais mudanças só restringem (aprovação única do superior) e quais tocam o envelope global (owner+CEO), sem evasão lateral.
- Retenção, exportação, pseudonimização e legal hold; SLI/SLO, RPO/RTO e janela de cutover.

Esses bloqueios impedem writes, grants e cutover, não a revisão deste SDD nem as fatias puras.

## 21. Critérios de aceite do design

- Ownership e seams não duplicam identidade, hierarquia, policy nem capability.
- Owner autenticado é requisito demonstrável de toda mutação privilegiada; assignment sozinho não concede autoridade.
- Grants e epoch têm um único SoT (PG `org`); projeções nunca autorizam.
- Tenant isolation reforçado em API, controller (`AuthorizedOrgScope`) e constraints SQL.
- Toda mutação estrutural é autorizada pela matriz de §7.3; nenhum assignment ativa grants sem o aprovador elegível; assignment pendente é inerte; autoridade avaliada por posição atuante e nunca cresce por mudança estrutural (SEC-ORG-29).
- Schema aditivo e cutover são etapas distintas com rollback.
- Cada invariante é provável por teste determinístico no domínio, no PG isolado ou no HTTP.
- D-HIER decidido pelo owner antes das fatias dependentes; seções `[SEGURANÇA]` com threat model antes das fatias correspondentes.
- SDD continua `draft` até Critic independente e aprovação explícita do owner.
