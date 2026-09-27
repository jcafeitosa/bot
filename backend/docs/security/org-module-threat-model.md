---
title: Threat model — módulo org (hierarquia, tenant, revocation epoch, owner auth P1)
description: Modelo de ameaças proporcional do SDD org com achados, mitigações e critérios de aceite testáveis SEC-ORG
tags:
  - security
  - threat-model
  - backend
  - org
  - agents
status: draft
---

# Threat model — módulo `org`

- **Estado:** proposto — aguardando revisão do Critic independente. Este documento **não** está aprovado, não aprova o SDD `org` e não autoriza implementação, migrations, endpoints, grants ou cutover.
- **Tipo:** documentação apenas (G1/design). Nenhum código, teste, config ou migration foi alterado.
- **Autor (Builder):** agente executor de segurança/Blue Team desta sessão. **Critic:** pendente de atribuição pelo Orquestrador (AGENTS.md G4: achados críticos/altos bloqueiam).
- **Data:** 2026-09-27. **Método:** leitura estática local (sem scans, probes ou chamadas a sistemas vivos), STRIDE resumido + casos de abuso por área de foco.
- **Atende:** SDD §15 ("Seções sensíveis à segurança"), itens 1–13, que pedem threat model antes das fatias correspondentes.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01), [provider-credentials-plaintext](./provider-credentials-plaintext.md), [orders-g2-threat-model](./orders-g2-threat-model.md).

## 0. Entradas revisadas

| Entrada | Estado observado | Identificação |
|---|---|---|
| [`sdd/org-module-sdd.md`](../sdd/org-module-sdd.md) | **não rastreado no git**; `status: draft`; "revisão Critic independente, threat model do bot Segurança e aprovação do owner pendentes"; linha "Revisão 2026-09-27 … Ciclo Critic 1" | **versão revisada:** sha256 `f853319e9f75857e32909ef582b484805a969c3899ca8fa18d7b2a979f7c44e3`, 347 linhas (21 seções). Uma versão anterior (`5312c2e9…`, 325 linhas) foi lida primeiro e substituída pelo autor durante esta revisão; os achados abaixo foram **rebaseados** na versão `f853319e…`. |
| [`planning/org-complete-implementation-plan.md`](../planning/org-complete-implementation-plan.md) | não rastreado; `status: draft`; fatias ORG-S01…S18 | sha256 `afb7b66bb4298d1a4f6c3b4d646b01fe707d6626ef4c352bb1cfe302cf77e935` |
| [`planning/org-plan-authority-errata.md`](../planning/org-plan-authority-errata.md) | não rastreado; `status: draft` | sha256 `1aa07e9a1e2abced43baaa0778c485603dcc36191517dc30134a728b80e927d8` |
| [`sdd/http-admin-auth-seam-sdd.md`](../sdd/http-admin-auth-seam-sdd.md), [`sdd/agents-owner-bootstrap-g1-sdd.md`](../sdd/agents-owner-bootstrap-g1-sdd.md) | `status: partial` | HEAD `2d1e3863` |
| Código (HEAD `2d1e3863`; arquivos citados inalterados desde `0880cdd5`; working tree sem mudanças em `backend/src`) | `presentation/http/{admin_auth.rs,register_owner.rs,state.rs,error.rs,cli.rs,routes/agents.rs,routes/graph_admin.rs}`, `modules/agents/{models/hierarchy.rs,controllers/registry.rs,controllers/bot_promotion.rs,adapters/pg_owner_bootstrap.rs}`, `modules/http_bridge/agents.rs`, `core/database/graph_query.rs`, `core/config/http/file.rs`, migrations `0002`, `0005`, `0010` | leitura estática |

**Estado do código `org`:** não existe `backend/src/modules/org/` nem migrations `org` (a auditoria read-only de módulos de 2026-09-27 também registra "Planned, zero code"). O modelo avalia (1) o **design** do SDD draft e (2) os **seams existentes** que o `org` consumirá. Se o SDD mudar depois do hash acima, revalidar §7.

## 1. Escopo e premissas

- Dentro: Organization/OrgUnit/JobTitle/Position/Assignment, policies/approvals/delegação interina, `capability_grants` e `org_revocation_epochs` em PG `org`, projeção Neo4j `graph_domain=org`, HTTP de `org`, dependência de P1 (owner humano) e P3 (principal de agente).
- Fora (documentos próprios): runtime/tool gateway (P3), orders ([orders-g2-threat-model](./orders-g2-threat-model.md)), credenciais ([provider-credentials-plaintext](./provider-credentials-plaintext.md)).
- Premissas do SDD adotadas: PG `org` é o único SoT de grants/epoch e **não há projeção de grants para `agents`** (§4, §5); Neo4j nunca autoriza; writes, grants e cutover ficam fail-closed até P1/P2/P3 (§14, §19).

## 2. Ativos

| ID | Ativo | Por que importa |
|---|---|---|
| A1 | Vínculo owner ↔ Organization (`org_owner_bindings`, `HumanPrincipal`) | Raiz da autoridade humana. |
| A2 | Árvore de supervisão (`positions.supervisor_position_id`, raiz única cujo ocupante é o CEO) | Determina o aprovador de cada policy subordinada e quem é CEO. |
| A3 | Assignments ativos | Assignment aplica automaticamente grants de policy aprovada (§7.5) — efeito equivalente a conceder capability. |
| A4 | Policy versions + approvals (hash/version) | Envelope de autonomia. |
| A5 | `capability_grants` + `org_revocation_epochs` | Decisão allow/deny de cada efeito. |
| A6 | `org_audit_events` | Não-repúdio e investigação. |
| A7 | Credenciais de owner/CEO humanos (P1) e de agentes (P3) | Personificação. |
| A8 | `issuer+subject` | Dado pessoal restrito (§12). |
| A9 | Projeção Neo4j `graph_domain=org` | Confidencialidade/integridade das consultas. |
| A10 | `AgentCapabilities` em `agents` (`promote_runtime_bot`, `consult_jev`) | Continuam SoT próprio (§5); hoje definíveis pelo corpo da requisição. |

## 3. Atores

| Ator | Confiança | Capacidades relevantes |
|---|---|---|
| Owner humano autenticado (pós-P1) | Alta | Aprova policy global com o CEO, transfere ownership, suspende/revoga. |
| CEO (ocupante da posição raiz; humano até P3, agente depois) | Média-alta | Segunda aprovação da policy global. |
| Superiores diretos (humanos; agentes após P3) | Média | Aprovam policies descendentes. |
| Especialistas/workers (agentes) | Baixa | Executam ações; alvo de prompt injection. |
| Portador de `BOT_HTTP_ADMIN_TOKEN` | Seam, **não** identidade | Hoje acessa rotas mutantes; com token vazio, **qualquer cliente** (F-ADM-01). |
| Operador de infra/DBA | Alta, fora da app | PG/Neo4j/env/backups/restore. |
| Membro de outra organização | Baixa | Credencial válida para A tentando alcançar B. |
| Atacante de rede | Nenhuma | Alcança a porta HTTP se exposta. |

## 4. Fronteiras de confiança

```text
[Humano / agente] --(TB1: HTTP + credencial P1/P3)--> [presentation/http: auth, DTO]
   --(TB2: AuthContext tipado)--> [controllers org: invariantes + matriz de autorização]
   --(TB3: tx PG SERIALIZABLE, role DML)--> [PG org: SoT + audit + outbox + epoch]
   --(TB4: outbox/projector core::database)--> [Neo4j graph_domain=org (derivado)]
[IdP externo] --(TB5: iss/aud/exp/assinatura/JWKS)--> [verificador P1]
[Runtime/tool gateway P3] --(TB6: grant+epoch lidos no PG primário antes do efeito)--> [PG org]
[Operador] --(TB7: env/bootstrap/runbook/restore)--> [PG, config]
```

## 5. Casos de abuso

### 5.a Escalada de autoridade

| ID | Caso | Situação no SDD `f853319e` / código |
|---|---|---|
| AB-A1 | Ator cria posição sob si e atribui ocupante controlado; o assignment aplica grants aprovados automaticamente → capability sem decisão do nível competente. | **Aberto.** §7.4/§7.5 (último bullet) e §10 "Administração" listam operações sem definir **quem** pode executá-las. |
| AB-A2 | Re-parent de `supervisor_position_id` (própria posição ou de ancestral) para trocar o aprovador ou assumir a raiz (virar "CEO"). | **Aberto.** §7.3 cobre ciclo/lock, não autorização do re-parent nem troca de ocupante da raiz. |
| AB-A3 | Autoatribuição de humano a posição superior. | **Aberto** (§10). |
| AB-A4 | Owner e CEO como mesmo principal. | **Mitigado no design** (§7.5 (a): principais distintos; owner ocupando qualquer posição bloqueia a policy global). Resíduo: uma pessoa com duas identidades no IdP (F-ORG-06). |
| AB-A5 | Aprovação atribuída a agente por `AgentId` no body/header. | **Mitigado no design** (§7.5 "Aprovador agente": rejeitado até P3). |
| AB-A6 | Ciclo por write skew. | **Mitigado no design** (§7.3 lock + SERIALIZABLE + teste concorrente). |
| AB-A7 | Principal com várias posições soma grants de envelopes distintos (combinação tóxica). | **Aberto** (§4, §7.4; §7.5 (b)(c) trata só aprovação). |
| AB-A8 | Mapeamento de papéis (D-HIER, ex.: `level_b` → novo tier) promove quem passa a aprovar. | **Pendente de decisão** (§3 D-HIER, §13); regra "sem alias silencioso" presente. |
| AB-A9 | Baseline `agents`: `promote_runtime_bot` no corpo do registro; `promoted_by` escolhido pelo cliente; CEO pode reportar a qualquer `owner_id` textual; vários CEOs. | **Código atual:** `http_bridge/agents.rs:30-41, 135-140`; `bot_promotion.rs:5-25`; `hierarchy.rs:50-51`. §5 mantém `AgentCapabilities` como SoT de `agents`. |

### 5.b Tenant spoofing

| ID | Caso | Situação |
|---|---|---|
| AB-B1 | `OrgId` de path/query/body sem vínculo com o principal. | Parcial: §10 exige leitura "filtrada pelo `org_id` do actor autenticado" e §8 item 4 binding por request; faltam escopo tipado e política 404/403 (F-ORG-07). |
| AB-B2 | Leituras sem auth com agency na query (padrão atual). | Código: `routes/agents.rs:20-26, 57-63, 75-85`. |
| AB-B3 | Actor/owner do corpo (padrão atual). | Código: `routes/agents.rs:44`, `register_owner.rs:5-16`. SDD §8 item 3 proíbe no `org`. |
| AB-B4 | SELECT de repositório sem `org_id`. | FKs compostas cobrem escrita (§11); leitura depende de disciplina (F-ORG-07). |
| AB-B5 | Traversal Neo4j cruzando organizações. | **Mitigado no design** (§6: todo nó com `org_id`; §10: teste cross-org obrigatório). Código atual não segue o padrão (`graph_query.rs:25-34, 36-50`; `graph_admin.rs` ignora `BOT_HTTP_AGENCY_ID`) — F-ORG-10. |
| AB-B6 | Idempotency key reutilizada entre orgs. | Aberto (§7.5 grants citam idempotency key sem escopo; §10 comandos). |

### 5.c Revocation epoch

| ID | Caso | Situação |
|---|---|---|
| AB-C1 | TOCTOU checagem→efeito. | **Reconhecido** (§7.5 "Janela checagem→efeito"), estreitamento delegado a P3. Resíduo: efeitos **internos** do `org` podem ser atômicos e isso não está exigido (F-ORG-04). |
| AB-C2 | Epoch lido de réplica/cache. | Aberto: SDD não exige primário. |
| AB-C3 | Epoch não checado em todo efeito. | Mitigado no design (§8 contrato de runtime). |
| AB-C4 | Restore/PITR/rollback volta epoch e reativa grants. | Aberto (§17 runbook, §19 rollback) — F-ORG-05. |
| AB-C5 | Granularidade: revogação ancestral não invalida descendentes se o epoch for por escopo. | Aberto: `org_revocation_epochs` listada em §11 sem chave/escopo/monotonicidade. |
| AB-C6 | Expiração com relógio da app. | Aberto — F-ORG-14. |
| AB-C7 | Token humano roubado válido até expirar. | P1 lista "expiry/revogação" (§14) sem requisito por request — F-ORG-02. |
| AB-C8 | CEO-agente comprometido suspende/revoga em massa (§7.5 (d) permite revogação a owner e CEO) antes de haver autenticação de agente. | Aberto — F-ORG-17 (DoS). |

### 5.d Dependência de P1 (owner humano)

| ID | Caso | Situação |
|---|---|---|
| AB-D1 | Usar `BOT_HTTP_ADMIN_TOKEN`/`VerifiedProductOwner` como owner. | Proibido no SDD (§3 D-OWNER-BIND, §8). |
| AB-D2 | Herdar o modo fail-open do seam admin. | Código: `admin_auth.rs:91-94` — ver [F-ADM-01](./admin-http-auth-fail-open.md). |
| AB-D3 | Migrar o owner singleton textual (`0010`, `source='env_explicit'`) para `HumanPrincipal` sem prova do IdP. | Pendente (D-OWNER-BIND). |

### 5.e Outros

| ID | Caso | Situação |
|---|---|---|
| AB-E1 | Injeção Cypher/SQL. | Código atual parametrizado (`graph_query.rs`, sqlx `bind`) — manter. |
| AB-E2 | Vazamento em erros/logs/`Debug`. | Código: `error.rs:211-214, 233-238, 258-262`; `admin_auth.rs:7`, `core/config/http/file.rs:3`. |
| AB-E3 | Auditoria adulterável. | **Mitigado no design** (§11: runtime só `INSERT`/`SELECT` + teste). |
| AB-E4 | `principal_ref_hash` reversível por enumeração. | Aberto (§6) — F-ORG-08. |

## 6. Mitigações e critérios de aceite testáveis (SEC-ORG)

"Audit event" = linha em `org_audit_events` com `correlation_id`, actor verificado (pseudonimizado), ação, alvo, decisão e razão.

| ID | Mitigação | Critério de aceite testável |
|---|---|---|
| **SEC-ORG-01** | Actor só do contexto autenticado; DTOs com `deny_unknown_fields`, sem campos de autoridade (`actor`, `owner_id`, `approved_by`, `org_id`). | Credencial válida do principal P + corpo com `actor`/`owner_id`/`approved_by` de outro → **400** `unknown_field`; audit registra P. Teste HTTP no entrypoint real. |
| **SEC-ORG-02** | OrgId do path só seleciona; associação do principal à org verificada no PG a cada request; resposta uniforme. | Membro de A em `/orgs/{B}/…` → **404** `org_not_found` com corpo idêntico ao de org inexistente; zero linhas de B lidas (asserção no repositório de teste); audit `org.access_denied` razão `cross_tenant`, sem subject bruto. |
| **SEC-ORG-03** | Escopo tipado `AuthorizedOrgScope` (construível só pela camada de auth) exigido por todo repositório; FKs compostas. | (a) Teste de visibilidade: sem construtor público fora da auth; (b) PG isolado: position com unit de outra org → violação de FK; (c) cada SELECT com teste de duas orgs. Opcional: RLS com `SET LOCAL app.org_id` e teste sem a variável → 0 linhas. |
| **SEC-ORG-04** | Cypher `org` parametrizado e com `org_id` + `graph_domain='org'` em todos os nós do caminho (§6, §10). | Fixture A/B com `REPORTS_TO` indevida A→B: traversal de A retorna só nós de A; ID com payload Cypher → 400/vazio e grafo intacto; verificação estática: nenhuma query do seam `org` montada com `format!`. |
| **SEC-ORG-05** | Matriz de autorização estrutural no SDD: operar só na **subárvore estrita** do ator; assign/transfer/re-parent/troca do ocupante da raiz exigem o aprovador do nível-alvo (raiz: owner); sem autoatribuição; sem re-parent da própria posição ou de ancestrais. | Ocupante de nível intermediário tenta: atribuir a posição par/superior/raiz → **403** `forbidden_scope`; atribuir a si → **403** `self_assignment_forbidden`; re-parent da própria posição/ancestral → **403**; nenhuma linha muda; audit de negação. |
| **SEC-ORG-06** | Assignment feito por quem não é aprovador do nível não ativa grants (`reconciling` até confirmação). | Assignment por não aprovador → grants `reconciling` e gateway nega; confirmação do aprovador elegível → `active`, com audit. |
| **SEC-ORG-07** | Anti-ciclo/same-org (§7.3). | Duas transações concorrentes A→B e B→A → uma confirma, outra **409** `hierarchy_conflict` após ≤ N retries; CTE recursiva sem ciclo; parent de outra org rejeitado. |
| **SEC-ORG-08** | Separação de funções (§7.5). | Mesmo `(issuer, subject)` como owner e CEO → **409** `approver_not_independent`, policy `draft`; owner ocupando qualquer posição → policy global bloqueada; aprovador que ocupa posição da subárvore → **403**; hash diferente → **409** `policy_hash_mismatch`; aprovação de agente sem credencial P3 → **403**. |
| **SEC-ORG-09** | Sem autoaprovação; subset monotônico. | Autor aprovando → **403**; policy descendente acima da ancestral → **422** `policy_exceeds_ancestor`; delegação interina aprovando → **403**. |
| **SEC-ORG-10** | `org_revocation_epochs(org_id, scope_kind, scope_id, epoch BIGINT NOT NULL)`; incremento `UPDATE … SET epoch = epoch + 1 … RETURNING` na mesma transação da revogação; trigger rejeita decremento. | Property test: epochs estritamente crescentes; `UPDATE` com valor menor pela role de runtime → erro; falha injetada após revogar → nem revogação nem incremento persistem. |
| **SEC-ORG-11** | Epoch checado a cada efeito, no PG **primário**, sem cache permissivo. | Após commit da revogação (n→n+1), a **próxima** autorização com n em cache → deny `stale_epoch`; com worker de outbox parado o deny é igual; PG indisponível → deny `authz_unavailable` em ≤ 2 s; teste de config: seam de authz não aceita pool de réplica. |
| **SEC-ORG-12** | Efeitos internos do `org` (aplicar grant, mutação estrutural) leem o epoch com `FOR SHARE` na mesma transação; efeitos externos seguem o contrato P3 com janela máxima **W** documentada (proposta W ≤ 250 ms) + kill switch. | Teste concorrente revogação × efeito interno: efeito confirma antes da revogação ou é negado, nunca depois com epoch antigo. P3: métrica `authz_epoch_check_to_effect_ms` p99 ≤ W. |
| **SEC-ORG-13** | Revogação ancestral invalida descendentes (epoch da org ou verificação da cadeia inteira). | Revogar policy intermediária → próxima ação de worker da mesma cadeia negada; outra cadeia inalterada (se escopado). |
| **SEC-ORG-14** | Anti-rollback: restore/PITR exige epoch bump (ex.: `+2^32`) e grants `reconciling` antes de reabrir; rollback de migration nunca decrementa/remove epoch. | Rehearsal em PG isolado: restaurar snapshot antigo + procedimento → grants revogados depois do snapshot continuam negados; readiness só verde após bump auditado. |
| **SEC-ORG-15** | P1: `principal_auth_epoch`/session version por principal no PG, checado por request; access token curto; validação completa de claims. | Após logout-all, transferência de owner ou revogação administrativa, token anterior → **401** no próximo request; TTL ≤ 15 min; leeway ≤ 60 s; testes: expirado, `nbf` futuro, `iss`/`aud` errados, `alg` inesperado/`none`, assinatura inválida, rotação JWKS, replay (se houver `jti`/nonce). |
| **SEC-ORG-16** | Relógio autoritativo = `now()` do PG para expiração. | Relógio da app deslocado ±10 min não muda decisões. |
| **SEC-ORG-17** | Auditoria cobre allow **e** deny de mutações/aprovações/revogações (append-only já no §11). | Cada operação gera 1 evento com `correlation_id`; serialização sem token de fixture nem subject bruto; teste de privilégio do §11. |
| **SEC-ORG-18** | Erros genéricos e `Debug` redigido. | Falha de PG → **503** `org_store_unavailable` sem texto do driver; `format!("{:?}", auth_context)` sem token; logs capturados sem token/subject. |
| **SEC-ORG-19** | `principal_ref = HMAC-SHA256(k, issuer‖subject)`, `k` fora de PG/Neo4j. | Chaves diferentes → refs diferentes; payload de outbox/projeção sem `issuer`/`subject`. |
| **SEC-ORG-20** | Fail-closed até P1: rotas `org` mutantes e leituras privadas inexistentes ou **503** `owner_auth_required` sem verificador P1, **independentemente** de `BOT_HTTP_ADMIN_TOKEN`; sem modo de auth desabilitada (ver SEC-ADM-01/14). | Com token admin válido e sem P1 → nenhuma escrita, 503/401; com token vazio → idem; feature `org` habilitada sem verificador → startup falha. |
| **SEC-ORG-21** | Autorização por posição atuante (sem união entre posições). | Principal com posições X (cap a) e Y (cap b): ação que exige a+b → deny; ação com cap a declarando Y → deny. |
| **SEC-ORG-22** | Delegação interina limitada (§10). | Cross-org → **400**; subdelegação → **403**; após revogar o delegante, próxima ação do delegado → deny `stale_epoch`. |
| **SEC-ORG-23** | Owner de `org` só por enrollment P1, nunca por string de env. | Só `BOT_PRODUCT_OWNER_BOOTSTRAP_ID`/ACK → nenhuma linha em `org_owner_bindings`; logs de conflito com referência pseudonimizada. |
| **SEC-ORG-24** | Idempotency keys com `UNIQUE (org_id, key)`. | Mesma key em A e B → comandos independentes. |
| **SEC-ORG-25** | Mapeamento de papéis (D-HIER) por relatório aprovado pelo owner, nunca automático. | Migração de fixture com `level_b` existentes: nenhum passa a aprovar policy de novo tier sem entrada explícita no relatório aprovado (hash); valor não mapeável bloqueia a migração. |
| **SEC-ORG-26** | Revogação/suspensão por agente só com credencial P3; antes disso, só humanos (owner/CEO humano). | Revogação atribuída a `AgentId` sem credencial P3 → **403**; revogação por owner humano funciona. |

## 7. Achados (SDD `f853319e…`; código HEAD `2d1e3863`)

Severidade: **Crítico/Alto bloqueiam** a fase indicada até correção ou aceite formal autorizado (AGENTS.md G4).

| ID | Sev. | Achado | Local | Correção | Critérios |
|---|---|---|---|---|---|
| **F-ORG-01** | **Alto** | Sem matriz de autorização para mutações estruturais (criar/re-parent posição, assign/transfer/unassign, troca do ocupante da raiz/CEO, publicar JobTitle). Como assignment aplica grants aprovados automaticamente e o aprovador deriva da árvore, quem atribui ou re-parenta concede capabilities e escolhe aprovadores. Bloqueia ORG-S dependentes de writes estruturais e de policies. | SDD §7.3, §7.4, §7.5 (último bullet), §10 "Administração" | Adicionar a matriz ao SDD (subárvore estrita; aprovador do nível-alvo; raiz só pelo owner; sem autoatribuição/re-parent de ancestrais; re-parent como alto impacto com preflight). | SEC-ORG-05, 06 |
| **F-ORG-02** | **Alto** | Revogação de credenciais humanas sem requisito verificável: P1 cita "expiry/revogação" mas não exige checagem por request nem TTL; token de owner/CEO roubado segue válido até expirar. Bloqueia o fechamento de P1. | SDD §14 (linha P1), §20 | Especificar em P1 `principal_auth_epoch`/session version por request, TTL curto, logout-all e revogação na transferência de owner. | SEC-ORG-15 |
| **F-ORG-03** | **Alto** | (Código existente; bloqueia reuso pelo `org`) Autoridade vinda do cliente: `owner_id` do corpo, `promote_runtime_bot` definido pelo próprio registrante, `promoted_by` escolhido pelo cliente; o token admin não identifica principal e pode estar ausente (F-ADM-01). | `routes/agents.rs:38-48`; `http_bridge/agents.rs:30-41`; `register_owner.rs:5-16, 32-48`; `bot_promotion.rs:5-25` | `org` não reutiliza esses seams; débito registrado para `agents` G1 (capabilities só por fluxo autorizado). | SEC-ORG-01, 20 |
| F-ORG-04 | Médio | Epoch residual: `org_revocation_epochs` sem chave/escopo/tipo/monotonicidade; leitura no primário não exigida; efeitos internos do `org` não exigidos atômicos com o epoch. | SDD §7.5, §8, §11 | Especificar conforme SEC-ORG-10…13. | SEC-ORG-10, 11, 12, 13 |
| F-ORG-05 | Médio | Sem garantia anti-rollback do epoch em restore/PITR/rollback. | SDD §17 (runbook), §19 | Procedimento de bump + `reconciling`. | SEC-ORG-14 |
| F-ORG-06 | Médio | Separação de funções compara principais, mas não impede a mesma pessoa com duas identidades no IdP (owner e "CEO humano" até P3). | SDD §7.5 (a), "Aprovador agente" | P1: vínculo de identidade verificada por pessoa (ou declaração auditada + aprovação fora de banda) para o CEO humano. | SEC-ORG-08, 15 |
| F-ORG-07 | Médio | Derivação/escopo de tenant sem mecanismo tipado nem política 404/403; SELECT sem `org_id` não tem defesa. | SDD §8 item 4, §10, §11 | Escopo tipado, 404 uniforme, RLS opcional. | SEC-ORG-02, 03 |
| F-ORG-08 | Médio | `principal_ref_hash` sem chave (linkável/reversível). | SDD §6 | HMAC com chave fora dos bancos. | SEC-ORG-19 |
| F-ORG-09 | Médio | Múltiplas posições: união de grants indefinida (combinação tóxica). | SDD §4, §7.4 | Autorização por posição atuante. | SEC-ORG-21 |
| F-ORG-10 | Médio | (Código) Traversal Neo4j sem filtro de agency em `root`/intermediários; `LIST_AGENTS` e `/admin/graph/*` retornam todas as agências mesmo com `BOT_HTTP_AGENCY_ID`. O SDD já exige o filtro; o padrão existente não deve ser copiado. | `core/database/graph_query.rs:25-34, 36-50`; `routes/graph_admin.rs` | Filtro em todo nó; binding nas rotas advisory. | SEC-ORG-04 |
| F-ORG-11 | Médio | (Código) Leituras de agents sem auth, com agency da query. | `routes/agents.rs:20-26, 57-63, 75-85` | Auth + escopo por principal (ver SEC-ADM-05). | SEC-ORG-02 |
| F-ORG-12 | Médio | (Código) Token admin estático compartilhado sem principal/expiração; `Debug` derivado com token. Detalhes em F-ADM-01. | `admin_auth.rs:7-12`; `core/config/http/file.rs:3-8` | Ver SEC-ADM-08; token admin nunca credencial do `org`. | SEC-ORG-18, 20 |
| F-ORG-13 | Médio | (Código) CEO pode reportar a qualquer `owner_id` textual, independente do owner verificado; vários CEOs por agência. Relevante para o cutover. | `hierarchy.rs:50-51`; `http_bridge/agents.rs:135-140` | Validar no cutover (§13 item 3); no `org`, raiz única já exigida (§7.3). | SEC-ORG-07, 25 |
| F-ORG-14 | Baixo | Relógio autoritativo para expiração não fixado; código atual usa `at_ms` da app. | SDD §7.5, §10 | `now()` do PG. | SEC-ORG-16 |
| F-ORG-15 | Baixo | Conflito de bootstrap loga IDs brutos do owner. | `pg_owner_bootstrap.rs:33-37` | Referência pseudonimizada. | SEC-ORG-23 |
| F-ORG-16 | Baixo | Erros HTTP com mensagem bruta de driver PG/Neo4j. | `error.rs:211-214, 233-238, 258-262` | Mensagens genéricas. | SEC-ORG-18 |
| F-ORG-17 | Baixo | Suspensão/revogação permitida ao CEO (§7.5 (d)) sem exigir credencial de agente quando o CEO for agente → DoS por agente comprometido. | SDD §7.5 (d) | Mesma exigência P3 das aprovações. | SEC-ORG-26 |

**Contagem:** Crítico 0 · Alto 3 (F-ORG-01, 02, 03) · Médio 10 · Baixo 4.

### Endereçado na revisão `f853319e` (registrado para o Critic)

Estes pontos, apontados na leitura da versão anterior, estão agora cobertos no design e viram critérios de teste: dual control com principais distintos e bloqueio quando o owner ocupa posição (§7.5 (a)); aprovação de agente rejeitada até P3 (§7.5); reconhecimento da janela TOCTOU (§7.5); tabela `org_revocation_epochs` listada (§11); filtro de `org_id` em traversal Neo4j + teste cross-org (§6, §10); auditoria append-only por privilégio com teste (§11); `VerifiedProductOwner` não autoriza `org` (§3 D-OWNER-BIND); remoção da projeção de grants para `agents` (§5); plano alinhado à errata (plano, "Review Focus" item 4).

## 8. Dependência de owner auth (P1)

**O que o `org` assume de P1:** verificador que valida `iss`, `aud`, `exp`/`nbf`, assinatura e rotação de chaves e produz `HumanPrincipal(issuer, subject)`; enrollment/bootstrap do owner por organização sem endpoint público de autoelevação (D-OWNER-BIND); revogação de sessão/credencial por principal com checagem por request (F-ORG-02); `AuthContext` tipado até os controllers. Autenticação de **agentes** fica em P3 (§14) — até lá, aprovações (e, por este modelo, revogações) atribuídas a agentes são rejeitadas.

**O que quebra sem P1:** não há actor verificável; dual control, audit de ator, isolamento de tenant e não-repúdio viram declarações do cliente. O único seam disponível (`BOT_HTTP_ADMIN_TOKEN`) é compartilhado, pode estar ausente (fail-open, [F-ADM-01](./admin-http-auth-fail-open.md)) e não identifica quem age; o owner singleton atual é uma string de env.

**Bloquear até P1 aprovado e implementado (com Critic):** todas as rotas HTTP mutantes do `org` e leituras privadas/histórico (SEC-ORG-20); approvals humanos e publicação de policy global; transferência de owner; cutover (§13); qualquer consumo de grants/epoch pelo runtime (também depende de P3).

**Pode prosseguir antes de P1:** tipos e invariantes puros (§14 "Não dependem"), schema aditivo sem writes (condicionado a P2) e revisão deste modelo.

## 9. Limitações

- SDD em draft, não rastreado e em edição ativa durante esta revisão; hash revisado registrado em §0.
- Não há código `org`; achados de código são dos seams existentes.
- Não verificados: roles/privilégios PG reais, logging do servidor PG, deploy/bind real, IdP candidato. Nenhum teste executado.

## 10. Próximos passos

1. Orquestrador atribui Critic de segurança independente (até 3 ciclos).
2. Architect incorpora F-ORG-01 ao SDD e F-ORG-02 ao escopo de P1 antes do LGTM de G1; F-ADM-01 corrigido antes de qualquer rota `org`.
3. Cada SEC-ORG vira teste red→green na fatia correspondente (AGENTS.md G3).
