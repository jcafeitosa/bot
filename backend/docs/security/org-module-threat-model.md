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

- **Estado:** aprovado com follow-up pelo Critic (TM-ORG ciclo 2); follow-ups N1-N8 aplicados, aguardando ciclo 3. Este documento não aprova o SDD `org` e não autoriza implementação, migrations, endpoints, grants ou cutover.
- **Tipo:** documentação apenas (G1/design). Nenhum código, teste, config ou migration foi alterado.
- **Autor (Builder):** agente executor de segurança/Blue Team desta sessão. **Critic:** Critic de segurança independente, ENTREGA TM-ORG (ciclo 2: aprovado com follow-up; ciclo 3 pendente). AGENTS.md G4: achados críticos/altos bloqueiam.
- **Data:** 2026-09-27. **Método:** leitura estática local (sem scans, probes ou chamadas a sistemas vivos), STRIDE resumido + casos de abuso por área de foco.
- **Atende:** SDD §15 ("Seções sensíveis à segurança"), itens 1–13, que pedem threat model antes das fatias correspondentes.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01), [owner-auth-idp-threat-model](./owner-auth-idp-threat-model.md) (P1), [provider-credentials-plaintext](./provider-credentials-plaintext.md), [orders-g2-threat-model](./orders-g2-threat-model.md).
- **Normatividade:** o SDD `905ed5e7` (§15) e o plano `9180a4d2` declaram este documento normativo para `org`. Isso não o aprova: ele continua aguardando o Critic, e cada mudança aqui reabre as fatias afetadas (plano, "Review Focus" item 3).

## 0. Entradas revisadas

| Entrada | Estado observado | Identificação |
|---|---|---|
| [`sdd/org-module-sdd.md`](../sdd/org-module-sdd.md) | não rastreado no git; `status: draft`; linha de revisão "Ciclo Critic 3: matriz de autorização estrutural (§7.3, F-ORG-01), SEC-ORG-06 e SEC-ORG-21 adotados (§4, §7.4), threat model declarado normativo, com SEC-ORG-03/10/14/19 incorporados (§15)" | **versão revisada:** sha256 `905ed5e7b80a5cc7bd1384c679944e02e4d0add3e4fe56a25ffbae32d14b9668`, 381 linhas (21 seções). Histórico: `5312c2e9…` (325 linhas) → `f853319e…` (347) → `03bd020a…` (346) → `905ed5e7…`. Este documento foi rebaseado em `905ed5e7`; o rebase intermediário em `03bd020a` foi substituído. |
| [`planning/org-complete-implementation-plan.md`](../planning/org-complete-implementation-plan.md) | não rastreado; `status: draft`; fatias S01…S18; nova seção "Traceability: threat model SEC-ORG → slice" (SEC-ORG-01…26) | sha256 `9180a4d239e3882e3651977125be18ef156de5b452c37b62a2cf941787f6866f`, 308 linhas |
| [`planning/org-plan-authority-errata.md`](../planning/org-plan-authority-errata.md) | não rastreado; `status: draft` | sha256 `1aa07e9a1e2abced43baaa0778c485603dcc36191517dc30134a728b80e927d8` (inalterada) |
| [`sdd/http-admin-auth-seam-sdd.md`](../sdd/http-admin-auth-seam-sdd.md), [`sdd/agents-owner-bootstrap-g1-sdd.md`](../sdd/agents-owner-bootstrap-g1-sdd.md) | `status: partial` | HEAD `2d1e3863` |
| Código (lido em HEAD `2d1e3863`; `admin_auth.rs`, `register_owner.rs`, `routes/agents.rs`, `pg_owner_bootstrap.rs` reconferidos por hash em `b003a9b7` e `state.rs` em `b2001a7c`; os demais arquivos citados não foram reconferidos depois de `2d1e3863`) | `presentation/http/{admin_auth.rs,register_owner.rs,state.rs,error.rs,cli.rs,routes/agents.rs,routes/graph_admin.rs}`, `modules/agents/{models/hierarchy.rs,controllers/registry.rs,controllers/bot_promotion.rs,adapters/pg_owner_bootstrap.rs}`, `modules/http_bridge/agents.rs`, `core/database/graph_query.rs`, `core/config/http/file.rs`, migrations `0002`, `0005`, `0010` | leitura estática |

**Estado do código `org`:** não existe `backend/src/modules/org/` nem migrations `org` (a auditoria read-only de módulos de 2026-09-27 também registra "Planned, zero code"). O modelo avalia (1) o **design** do SDD draft e (2) os **seams existentes** que o `org` consumirá. Se o SDD mudar depois do hash acima, revalidar §5, §6 e §7.

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

| ID | Caso | Situação no SDD `905ed5e7` / código |
|---|---|---|
| AB-A1 | Ator cria posição sob si e atribui ocupante controlado; o assignment aplica grants aprovados automaticamente → capability sem decisão do nível competente. | **Mitigado no design** (§7.3 matriz: posição nasce vaga; assign só pelo aprovador do nível de T ou owner; §7.4 SEC-ORG-06). Resíduo: F-ORG-18. |
| AB-A2 | Re-parent de `supervisor_position_id` (própria posição ou de ancestral) para trocar o aprovador ou assumir a raiz (virar "CEO"). | **Mitigado no design** (§7.3: sem re-parent da própria posição/ancestrais; raiz nunca re-parenta; troca do CEO só pelo owner; re-parent com preflight e grants `reconciling`). Resíduo: F-ORG-19. |
| AB-A3 | Autoatribuição de humano a posição superior. | **Mitigado no design** (§7.3 `403 self_assignment_forbidden`; §10). |
| AB-A4 | Owner e CEO como mesmo principal. | **Mitigado no design** (§7.5 (a); default fail-closed de **D-OWNER-POSITION**, §3 linha 68: owner na raiz ou em qualquer posição bloqueia a policy global; §7.3: owner não pode ser ocupante da raiz). Resíduo (mesma pessoa com duas identidades no IdP, F-ORG-06) passou a requisito de P1 (§14). |
| AB-A5 | Aprovação atribuída a agente por `AgentId` no body/header. | **Mitigado no design** (§7.5 "Aprovador agente": rejeitado até P3). |
| AB-A6 | Ciclo por write skew. | **Mitigado no design** (§7.3 lock + SERIALIZABLE + teste concorrente). |
| AB-A7 | Principal com várias posições soma grants de envelopes distintos (combinação tóxica). | **Mitigado no design** (§4 linha 76 e §7.4: posição atuante, sem união — SEC-ORG-21). |
| AB-A8 | Mapeamento de papéis (D-HIER, ex.: `level_b` → novo tier) promove quem passa a aprovar. | **Pendente de decisão** (§3 D-HIER, §13); regra "sem alias silencioso" presente. |
| AB-A10 | Assignment `pending_approver_confirmation` tratado como "ativo" para a matriz (o ocupante pendente atua a partir da posição, aprova ou atribui na subárvore), ou confirmado pelo próprio autor com outro chapéu (owner que também ocupa `parent(T)`). | **Parcial** (§7.3, §7.4): o SDD torna os grants `reconciling`, mas não diz que o assignment pendente não conta como "assignment ativo" da posição atuante, nem exige confirmador distinto do autor, nem prazo — F-ORG-18, SEC-ORG-28. |
| AB-A11 | `JobTitleVersion`/policy reaproveitada sob um superior com envelope menor; consumidor com cache chaveado só no epoch mantém o escopo antigo, mais largo, porque re-parent/troca de versão de ancestral não incrementam epoch. | **Parcial** (§7.5 interseção da cadeia; §7.3 revalidação da subárvore no re-parent; §8 invalidação de cache): falta amarrar mudanças que estreitam o escopo a um incremento de epoch/versão da cadeia — F-ORG-19, SEC-ORG-29. |
| AB-A9 | Baseline `agents`: `promote_runtime_bot` no corpo do registro; `promoted_by` escolhido pelo cliente; CEO pode reportar a qualquer `owner_id` textual; vários CEOs. | **Código atual:** `http_bridge/agents.rs:30-41, 135-140`; `bot_promotion.rs:5-25`; `hierarchy.rs:50-51`. §5 mantém `AgentCapabilities` como SoT de `agents`. |

### 5.b Tenant spoofing

| ID | Caso | Situação |
|---|---|---|
| AB-B1 | `OrgId` de path/query/body sem vínculo com o principal. | **Mitigado no design** (§8 item 4: `AuthorizedOrgScope` por request; 404 uniforme no plano, S07 SEC-ORG-02). O SDD não cita o 404 uniforme (ver G3 em §6.3). |
| AB-B2 | Leituras sem auth com agency na query (padrão atual). | Código: `routes/agents.rs:20-26, 57-63, 75-85`. |
| AB-B3 | Actor/owner do corpo (padrão atual). | Código: `routes/agents.rs:44`, `register_owner.rs:5-16`. SDD §8 item 3 proíbe no `org`. |
| AB-B4 | SELECT de repositório sem `org_id`. | **Mitigado no design** (§11: todo repositório recebe `AuthorizedOrgScope`; teste com duas orgs por SELECT; RLS opcional). |
| AB-B5 | Traversal Neo4j cruzando organizações. | **Mitigado no design** (§6: todo nó com `org_id`; §10: teste cross-org obrigatório). Código atual não segue o padrão (`graph_query.rs:25-34, 36-50`; `graph_admin.rs` ignora `BOT_HTTP_AGENCY_ID`) — F-ORG-10. |
| AB-B6 | Idempotency key reutilizada entre orgs. | **Mitigado no design** (§7.5: `UNIQUE (org_id, key)` — SEC-ORG-24). |

### 5.c Revocation epoch

| ID | Caso | Situação |
|---|---|---|
| AB-C1 | TOCTOU checagem→efeito. | **Reconhecido** (§7.5): efeitos internos com `FOR SHARE` na mesma transação (SEC-ORG-12); efeitos externos delegados a P3. |
| AB-C2 | Epoch lido de réplica/cache. | **Mitigado no design** (§7.5 "Epoch": leitura sempre no primário). |
| AB-C3 | Epoch não checado em todo efeito. | Mitigado no design (§8 contrato de runtime). |
| AB-C4 | Restore/PITR/rollback volta epoch e reativa grants. | **Mitigado no design** (§7.5 "Anti-rollback", §17, §19 — SEC-ORG-14). |
| AB-C5 | Granularidade: revogação ancestral não invalida descendentes se o epoch for por escopo. | **Mitigado no design** (§7.5: tabela com escopo, trigger anti-decremento, revogação ancestral invalida descendentes). |
| AB-C6 | Expiração com relógio da app. | **Mitigado no design** (§7.5: `now()` do PG). |
| AB-C7 | Token humano roubado válido até expirar. | **Requisito registrado em P1** (§14 linha 290: session epoch por request, TTL curto, logout-all, revogação na transferência). Verificação pendente no SDD de P1. |
| AB-C8 | CEO-agente comprometido suspende/revoga em massa (§7.5 (d) permite revogação a owner e CEO) antes de haver autenticação de agente. | **Mitigado no design** (§7.5 (d): CEO agente só com credencial P3 — SEC-ORG-26). |

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
| AB-E4 | `principal_ref_hash` reversível por enumeração. | **Mitigado no design** (§6 linha 120, §12: HMAC com chave de `core::config` fora de PG/Neo4j). |

## 6. Mitigações e critérios de aceite testáveis (SEC-ORG)

### 6.1 Critérios

"Audit event" = linha em `org_audit_events` com `correlation_id`, actor verificado (pseudonimizado), ação, alvo, decisão e razão.

| ID | Mitigação | Critério de aceite testável |
|---|---|---|
| **SEC-ORG-01** | Actor só do contexto autenticado; DTOs com `deny_unknown_fields`, sem campos de autoridade (`actor`, `owner_id`, `approved_by`, `org_id`). | Credencial válida do principal P + corpo com `actor`/`owner_id`/`approved_by` de outro → **400** `unknown_field`; audit registra P. Teste HTTP no entrypoint real. |
| **SEC-ORG-02** | OrgId do path só seleciona; associação do principal à org verificada no PG a cada request; resposta uniforme. | Membro de A em `/orgs/{B}/…` → **404** `org_not_found` com corpo idêntico ao de org inexistente; zero linhas de B lidas (asserção no repositório de teste); audit `org.access_denied` razão `cross_tenant`, sem subject bruto. |
| **SEC-ORG-03** | Escopo tipado `AuthorizedOrgScope` (construível só pela camada de auth) exigido por todo repositório; FKs compostas. | (a) Teste de visibilidade: sem construtor público fora da auth; (b) PG isolado: position com unit de outra org → violação de FK; (c) cada SELECT com teste de duas orgs. Opcional: RLS com `SET LOCAL app.org_id` e teste sem a variável → 0 linhas. |
| **SEC-ORG-04** | Cypher `org` parametrizado e com `org_id` + `graph_domain='org'` em todos os nós do caminho (§6, §10). | Fixture A/B com `REPORTS_TO` indevida A→B: traversal de A retorna só nós de A; ID com payload Cypher → 400/vazio e grafo intacto; verificação estática: nenhuma query do seam `org` montada com `format!`. |
| **SEC-ORG-05** | Matriz de autorização estrutural no SDD: operar só na **subárvore estrita** do ator; assign/transfer/re-parent/troca do ocupante da raiz exigem o aprovador do nível-alvo (raiz: owner); sem autoatribuição; sem re-parent da própria posição ou de ancestrais. **(N2)** Transfer exige autoridade **na origem e no destino** (o actor precisa ser aprovador do nível das duas posições, ou owner); encerrar/congelar/aposentar posição ou encerrar unidade só com o nó **vago e sem descendentes ativos** (sem cascata); re-parent de **`OrgUnit`** segue as mesmas regras do re-parent de posição (unidade na subárvore estrita do actor, novo pai no escopo do actor, nunca a própria unidade ou ancestral, sem ciclo, mesma org). | Ocupante de nível intermediário tenta: atribuir a posição par/superior/raiz → **403** `forbidden_scope`; atribuir a si → **403** `self_assignment_forbidden`; re-parent da própria posição/ancestral → **403**; nenhuma linha muda; audit de negação. **(N2)** (f) transfer de P1 (no escopo do actor) para P2 fora do escopo → **403** `forbidden_scope`, e o inverso também, com P1 e P2 inalteradas; (g) encerrar posição ocupada → **409** `position_not_vacant`; encerrar posição vaga com filha ativa → **409** `has_descendants`; encerrar unidade com posição ou unidade filha ativa → **409** `has_descendants`; (h) re-parent de `OrgUnit` fora do escopo, da própria unidade ou de ancestral → **403**; que formaria ciclo → **409** `hierarchy_conflict`; para pai de outra org → **400**/**404** uniforme. |
| **SEC-ORG-06** | Assignment feito por quem não é aprovador do nível não ativa grants (`reconciling` até confirmação). | Assignment por não aprovador → grants `reconciling` e gateway nega; confirmação do aprovador elegível → `active`, com audit. |
| **SEC-ORG-07** | Anti-ciclo/same-org (§7.3). | Duas transações concorrentes A→B e B→A → uma confirma, outra **409** `hierarchy_conflict` após ≤ N retries; CTE recursiva sem ciclo; parent de outra org rejeitado. |
| **SEC-ORG-08** | Separação de funções (§7.5 (a)(b)(d)), com a **exceção única do CEO** de §7.5 (b): o ocupante da raiz é elegível **somente** para a policy global da própria organização e **somente** junto com a aprovação do owner distinto. | Mesmo `(issuer, subject)` como owner e CEO → **409** `approver_not_independent`, policy `draft`; owner ocupando a raiz ou, pelo default de D-OWNER-POSITION, qualquer posição → policy global bloqueada; aprovador que ocupa a posição-alvo ou posição da sua subárvore → **403**, **exceto** o CEO na policy global da própria org com owner distinto aprovando o mesmo hash → policy ativa; CEO sozinho, CEO com owner que é o mesmo principal, CEO na policy global de outra org ou CEO em policy subordinada cuja subárvore contenha posição que ele ocupa → **403**/bloqueada; hash diferente → **409** `policy_hash_mismatch`; aprovação de agente sem credencial P3 → **403**. |
| **SEC-ORG-09** | Sem autoaprovação; subset monotônico. | Autor aprovando → **403**; policy descendente acima da ancestral → **422** `policy_exceeds_ancestor`; delegação interina aprovando → **403**. |
| **SEC-ORG-10** | `org_revocation_epochs(org_id, scope_kind, scope_id, epoch BIGINT NOT NULL)`; incremento `UPDATE … SET epoch = epoch + 1 … RETURNING` na mesma transação da revogação; trigger rejeita decremento. | Property test: epochs estritamente crescentes; `UPDATE` com valor menor pela role de runtime → erro; falha injetada após revogar → nem revogação nem incremento persistem. |
| **SEC-ORG-11** | Epoch checado a cada efeito, no PG **primário**, sem cache permissivo. | Após commit da revogação (n→n+1), a **próxima** autorização com n em cache → deny `stale_epoch`; com worker de outbox parado o deny é igual; PG indisponível → deny `authz_unavailable` em ≤ **D-SEC-AUTHZ-TIMEOUT**; teste de config: seam de authz não aceita pool de réplica. |
| **SEC-ORG-12** | Efeitos internos do `org` (aplicar grant, mutação estrutural) leem o epoch com `FOR SHARE` na mesma transação; efeitos externos seguem o contrato P3 com janela máxima **W** documentada (valor = **D-SEC-W**, decisão do owner) + kill switch. | Teste concorrente revogação × efeito interno: efeito confirma antes da revogação ou é negado, nunca depois com epoch antigo. P3: métrica `authz_epoch_check_to_effect_ms` p99 ≤ D-SEC-W. |
| **SEC-ORG-13** | Revogação ancestral invalida descendentes (epoch da org ou verificação da cadeia inteira). | Revogar policy intermediária → próxima ação de worker da mesma cadeia negada; outra cadeia inalterada (se escopado). |
| **SEC-ORG-14** | Anti-rollback: restore/PITR exige epoch bump ≥ **D-SEC-EPOCH-BUMP** e grants `reconciling` antes de reabrir; rollback de migration nunca decrementa/remove epoch. | Rehearsal em PG isolado: restaurar snapshot antigo + procedimento → grants revogados depois do snapshot continuam negados; readiness só verde após bump auditado. |
| **SEC-ORG-15** | P1: `principal_auth_epoch`/session version por principal no PG, checado por request; access token curto; validação completa de claims. | Após logout-all, transferência de owner ou revogação administrativa, token anterior → **401** no próximo request; TTL ≤ **D-SEC-TTL**; leeway ≤ **D-SEC-LEEWAY**; testes: expirado, `nbf` futuro, `iss`/`aud` errados, `alg` inesperado/`none`, assinatura inválida, rotação JWKS, replay (se houver `jti`/nonce). |
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
| **SEC-ORG-27** | Regra (c) de §7.5: aprovador que passa a ocupar posição coberta por policy que aprovou não recebe os grants dessa policy até existir versão aprovada por outro aprovador elegível; e esse assignment nasce `PendingApproverConfirmation` (§7.3/§7.4). | Superior S aprova a policy P da posição T; depois S é atribuído a T → assignment `pending_approver_confirmation` e o gateway nega toda capability de P para S; nova versão de P aprovada por outro aprovador elegível → grants de S passam a `active`; aprovação da nova versão pelo próprio S → **403**; audit de cada passo. |
| **SEC-ORG-28** | Assignment pendente (`pending_approver_confirmation`) é inerte e expira: não conta como assignment ativo para a matriz de §7.3 nem para `acting_position_id`, aprovações ou delegação; grants `reconciling`; confirmador precisa ser o aprovador elegível **e** principal distinto do autor do assignment; prazo **D-SEC-PENDING-TTL** (decisão do owner), após o qual o assignment é encerrado como `expired`, com audit; o owner sempre consegue desfazer ou reatribuir. | (a) Ocupante pendente usa T como `acting_position_id` para atribuir/aprovar/delegar → **403** `assignment_pending`; (b) owner que também ocupa `parent(T)` confirma assignment que ele mesmo fez como `Owner` → **403** `confirmer_not_independent`; (b′) **(N5, amarrado a D-OWNER-POSITION)** o mesmo principal, agindo por `Position(parent(T))` com `acting_position_id = parent(T)`, faz o assignment → `ActiveOnCommit` (SDD §7.3 linha 159), com audit que registra a posição atuante e a marca `actor_is_owner = true`; se o owner decidir em D-OWNER-POSITION que owner não ocupa posições, (b) e (b′) viram "owner como ocupante de posição não raiz → **403**"; (c) após o TTL (relógio do PG) → estado `expired`, gateway nega, evento de audit; (d) com `parent(T)` vago, o owner preenche a cadeia até a raiz (raiz é `ActiveOnCommit`) e a confirmação segue — teste prova ausência de deadlock; (e) nenhuma operação de owner/recuperação de owner depende de confirmação pendente. |
| **SEC-ORG-29** | Autoridade efetiva monotônica e recalculada no ponto de decisão: `efetivo(T) = policy(T) ∩ efetivo(parent(T))`, avaliado a cada decisão (gateway/read API) a partir do PG primário; re-parent, troca de versão de cargo de qualquer ancestral, nova versão/revogação de policy ancestral e vacância/encerramento de ancestral incrementam, na mesma transação, o epoch (ou versão de cadeia) do escopo afetado, para que nenhum cache mantenha escopo mais largo. | (a) Property test: para qualquer árvore e policies, `efetivo(T) ⊆ efetivo(A)` para todo ancestral A; (b) re-parent de T para N com envelope menor → a próxima decisão para T e descendentes usa o novo limite, inclusive com cache aquecido, e quem tem o epoch antigo recebe `stale_epoch`; (c) estreitar a policy de um avô → próxima ação do neto acima do novo limite → deny, sem reaprovar o neto; (d) **(N1)** mesma `JobTitleVersion`/policy aprovada sob o superior S1 e reaproveitada sob um segundo superior S2 **sem** aprovação de S2 → grants `reconciling` e gateway **403**; depois que S2 aprova, limite efetivo = interseção da cadeia de S2 (e continua distinto do limite sob S1 quando os envelopes diferem); (e) o enforcement ocorre na decisão, não só no assignment: grant `active` antigo não autoriza ação que excede a interseção atual. Condição de consentimento: ver SEC-ORG-30. |
| **SEC-ORG-30** | **(N1)** A aprovação dos grants de uma posição T fica ligada à tupla (`JobTitleVersion` de T, `parent(T)`, versão da posição aprovadora). Qualquer mudança que ativaria grants sob uma cadeia que não os aprovou (re-parent de T; troca da `JobTitleVersion` de T pelo owner, SDD §7.3 linha 162; mesma `JobTitleVersion` reaproveitada sob outro superior) coloca esses grants em `reconciling` (fail-closed, sem autoridade) até o aprovador atual de `parent(T)` aprovar. Esse aprovador precisa ser principal distinto de quem fez a mudança (mesma regra de confirmador de SEC-ORG-28). **Sucessor:** a aprovação é da posição, não da pessoa; o novo ocupante de `parent(T)` herda as aprovações vigentes, mas recebe uma lista de revisão auditada dos grants que herdou e pode revogar qualquer um, com incremento de epoch na mesma transação. **"Superior direto" (SDD §7.5 linha 182)** = o ocupante ativo de `parent(T)` na cadeia atual de T, e não quem aprovou a mesma `JobTitleVersion` em outra cadeia. | (a) Re-parent de T de O para N → grants de T `reconciling` e gateway **403** até o ocupante de N aprovar; se quem re-parentou é o ocupante de N, a aprovação exige outro principal elegível (ou fica `reconciling`). (b) Owner troca a `JobTitleVersion` de T → grants antigos e novos `reconciling` até o aprovador de `parent(T)` aprovar; aprovação pelo próprio owner que fez a troca → **403** `approver_not_independent`. (c) `JobTitleVersion` aprovada sob S1 e usada sob S2 → `reconciling`/**403** até S2 aprovar (mesmo caso de SEC-ORG-29 (d)). (d) Sucessão em `parent(T)`: grants de T continuam `active`; o sucessor recebe a lista de revisão com um evento de audit; revogar um grant pela lista → epoch incrementado na mesma transação e a próxima ação do ocupante de T é negada com `stale_epoch`. (e) Nenhuma aprovação é aceita se a tupla gravada não bater com a cadeia atual → **409** `approval_chain_mismatch`. |

### 6.2 Mapa SEC-ORG → fatias (G2) e conciliação com o plano `9180a4d2`

A coluna "Este documento" é o mapeamento de segurança; "Plano" copia a tabela "Traceability: threat model SEC-ORG → slice". Divergência = fatia onde o critério precisa valer e o plano não lista.

| SEC-ORG | Este documento | Plano | Divergência |
|---|---|---|---|
| 01 | S07, S10, S12, S13, S14 | S13 (S07 DTOs de leitura) | **Faltam S10, S12, S14**: aprovação, owner/transferência e delegação também recebem DTOs com campos de autoridade. |
| 02 | S07, S13 | S07, S13 | — |
| 03 | S04, S05, S07, S13 | S04, S07 | Menor: S05/S13 usam os mesmos repositórios (coberto pelo port de S04). |
| 04 | S07, S08 | S08 | Menor: traversal exposto na API de leitura (S07 já tem teste cross-org de traversal). |
| 05 | S03, S06, S13 | S03, S06, S13 | — |
| 06 | S03, S05, S06, S11, S13 | S03, S05, S11, S13 | Menor: re-parent deixa grants `reconciling` (S06 já cobre no texto da fatia). |
| 07 | S02, S06 | S02, S06 | — |
| 08 | S09, S10 | S09, S10 | — |
| 09 | S09, S10, S14 | S09, S10 | Menor: delegação interina não aprova (S14 cobre por SEC-ORG-22). |
| 10 | S11 | S11 | — |
| 11 | S11, S17 | S11, S17 | — |
| 12 | S06, S11, S17 | S06, S11, S17 | — |
| 13 | S11 | S11 | — |
| 14 | S11, S18 | S11, S18 | — |
| 15 | P1 | P1 | — |
| 16 | S11, S14 | S11, S14 | — |
| 17 | S05, S10, S11, S12, S13 | S05, S13 | **Faltam S10, S11, S12**: aprovações, revogações e transferência de owner precisam de audit de allow e deny. |
| 18 | S07, S10, S11, S13 | S07, S13 | Menor: erros das APIs de policy/grant. |
| 19 | S05, S08 | S05, S08 | — |
| 20 | S07, S10, S12, S13, S14 | S07, S13 | **Faltam S10, S12, S14**: toda rota mutante fica fail-closed sem P1, não só estrutura. |
| 21 | S01, S03, S10, S11, S13, S14, S17 | S01, S03, S10, S11, S13, S17 | Menor: delegação atua a partir de uma posição (S14). |
| 22 | S14 | S14 | — |
| 23 | P1, S12 | P1, S12 | — |
| 24 | S05, S11, S13 | S05, S13 | **Falta S11**: grants usam `UNIQUE (org_id, key)` (SDD §7.5 linha 190). |
| 25 | S15, S16 | S15, S16 | — |
| 26 | S11 | S11 | — |
| 27 (novo) | S03 (implementa), S05, S13 (persiste), S09, S10, S11 | — (S11 já tem o teste da regra (c) sem ID) | **Não mapeado:** adicionar à tabela (N3). |
| 28 (novo) | S03, S05, S11 (item (c): expiração e grants), S13 | — | **Não mapeado** (N3: S11 incluída). |
| 29 (novo) | S05, S13 (gatilhos de incremento de epoch), S09 (property test), S06, S11, S17 | — | **Não mapeado** (N3). |
| 30 (novo, N1) | S03 (troca de `JobTitleVersion`), S05, S09, S10, S13 | — | **Não mapeado.** |

O plano cumpre a afirmação "SEC-ORG-01…26 rastreados por fatia" (as 26 linhas existem). As divergências acima são de **cobertura incompleta**, não de ausência.

### 6.3 Critérios que geram requisito de design (G3, cabe ao Architect)

Cobertos no texto do SDD `905ed5e7`: SEC-ORG-01 (§10 linha 249), 03 (§8 item 4, §11), 04 (§6 linha 120, §10 linha 243; **parcial**, ver abaixo), 05 e 06 (§7.3, §7.4), 07 (§7.1 linha 127, §7.2 linha 142, §11 linha 259), 08 (§7.5 (a)(b)(d), linha 183), 09 (§7.5 linha 182: autor não aprova, subconjunto da ancestral, delegação não aprova), 20 (§7.5 linha 194, §19 linha 352; **parcial**, ver abaixo), 10, 11, 12 (interno), 13, 16 (§7.5), 14 (§7.5, §17, §19), 15 e 23 (§14 P1), 17 (§12), 19 (§6, §12), 21 (§4, §7.4), 22 (§10), 24 (§7.5), 25 (§13 itens 3-6), 26 (§7.5 (d)).

**Ainda ausentes do SDD** (estão só no plano ou só aqui):

- **SEC-ORG-02**: 404 uniforme para org alheia e inexistente (só no plano, S07).
- **SEC-ORG-18**: `Debug` redigido de contexto de auth (o §10 cobre erros sem principal/segredo, não `Debug`).
- **SEC-ORG-27**: a regra (c) está no SDD, mas o SDD não liga (c) ao estado `PendingApproverConfirmation` de forma testável (o plano S03 liga).
- **SEC-ORG-04 (parcial, N4)**: o SDD exige `org_id` em todo nó e filtro no traversal, mas não traz a regra "nenhuma query Cypher do seam `org` montada com `format!`" (só parametrizada).
- **SEC-ORG-20 (parcial, N4)**: o SDD diz que a feature fica fail-closed sem P1/P3 e que nenhum write é ativado antes do gate, mas não exige que o **startup falhe** com a feature `org` habilitada e sem verificador P1.
- **SEC-ORG-05 (N2)**: a matriz de §7.3 não tem linha para re-parent de `OrgUnit` nem diz que transfer exige autoridade na origem e no destino; "encerrar posição" não exige explicitamente ausência de descendentes (§7.1 linha 127 cobre só unidade).
- **SEC-ORG-28** (F-ORG-18), **SEC-ORG-29** (F-ORG-19) e **SEC-ORG-30** (F-ORG-20): novos.
- **Parâmetros de §6.4**: o SDD (§7.5 linha 193, "ex.: `+2^32`") e o plano (P1 linha 78, "TTL ≤ 15 min, leeway ≤ 60 s"; S17 W) fixam números que vieram deste documento como proposta. Devem passar a referenciar as decisões do owner.

### 6.4 Parâmetros que são decisão do owner (G4)

Os números abaixo não são fixados por este documento. Os valores entre parênteses são só sugestão inicial para a decisão.

| ID | Parâmetro | Usado em | Sugestão (não vinculante) |
|---|---|---|---|
| D-SEC-W | Janela máxima checagem→efeito para efeitos externos | SEC-ORG-12, plano S17 | ≤ 250 ms |
| D-SEC-TTL | Vida máxima do access token humano | SEC-ORG-15, plano P1 | ≤ 15 min |
| D-SEC-LEEWAY | Tolerância de relógio em `exp`/`nbf` | SEC-ORG-15, plano P1 | ≤ 60 s |
| D-SEC-AUTHZ-TIMEOUT | Tempo até deny quando o PG está indisponível | SEC-ORG-11 | ≤ 2 s |
| D-SEC-EPOCH-BUMP | Incremento de epoch em restore/PITR | SEC-ORG-14, SDD §7.5 | ≥ 2^32 |
| D-SEC-PENDING-TTL | Prazo de um assignment `pending_approver_confirmation` | SEC-ORG-28 | 7 dias |

## 7. Achados e estado contra o SDD `905ed5e7`

Severidade: **Crítico/Alto bloqueiam** a fase indicada até correção ou aceite formal autorizado (AGENTS.md G4). "Resolvido no design" significa que o texto do SDD/plano cobre o achado; a verificação continua nos testes da fatia mapeada.

| ID | Sev. | Achado | Local (origem) | Estado em `905ed5e7` | Critérios |
|---|---|---|---|---|---|
| **F-ORG-01** | **Alto** | Sem matriz de autorização para mutações estruturais; quem atribui ou re-parenta concederia capability e escolheria aprovador. | SDD anterior §7.3, §7.4, §7.5, §10 | **Resolvido no design:** matriz em §7.3 (linhas 145-167), §7.4 (linhas 173-175), §10 (linha 243), §15 item 1, §16, §21; plano S03/S06/S13. Resíduos Médios: F-ORG-18, F-ORG-19. | SEC-ORG-05, 06, 28 |
| **F-ORG-02** | **Alto** | Revogação de credenciais humanas sem checagem por request nem TTL. | SDD anterior §14 (P1), §20 | **Resolvido no design, delegado a P1:** §14 linha 290 (session epoch por request, TTL curto, logout-all, revogação na transferência), §20 linha 363; plano P1 linhas 77-78. Continua bloqueando o **fechamento de P1** até existir o SDD de P1 ([owner-auth-idp-threat-model](./owner-auth-idp-threat-model.md), SEC-OWN-07). | SEC-ORG-15 |
| **F-ORG-03** | **Alto** | (Código existente) Autoridade vinda do cliente: `owner_id` do corpo, `promote_runtime_bot` definido pelo registrante, `promoted_by` escolhido pelo cliente; token admin sem principal e opcional (F-ADM-01). Backlog `agents`: **AGT-SEC-01** (ver §7.2). | `routes/agents.rs:38-48`; `http_bridge/agents.rs:30-41`; `register_owner.rs:5-16, 32-48`; `bot_promotion.rs:5-25` | **Aberto (débito de `agents`, fora do `org`).** SDD §15 linha 317 e plano ("Threat-model code findings outside `org`") registram que `org` não reutiliza esses seams, então não bloqueia o SDD `org`; bloqueia G1 de `agents` e qualquer reuso. | SEC-ORG-01, 20 |
| F-ORG-04 | Médio | Epoch sem chave/escopo/monotonicidade, sem primário, efeitos internos não atômicos. | SDD anterior §7.5, §8, §11 | **Resolvido no design** (§7.5 linhas 191-192, §11 linha 257). | SEC-ORG-10…13 |
| F-ORG-05 | Médio | Sem anti-rollback do epoch. | SDD anterior §17, §19 | **Resolvido no design** (§7.5 linha 193, §17 linha 336, §19 linha 357). | SEC-ORG-14 |
| F-ORG-06 | Médio | Mesma pessoa com duas identidades (owner e CEO humano). | SDD §7.5 (a) | **Resolvido no design, delegado a P1** (§14 linha 290; plano P1 linha 77). | SEC-ORG-08, 15 |
| F-ORG-07 | Médio | Escopo de tenant sem mecanismo tipado nem 404/403. | SDD anterior §8, §10, §11 | **Resolvido no design** (§8 item 4, §11 linha 256; 404 uniforme só no plano S07 — ver §6.3). | SEC-ORG-02, 03 |
| F-ORG-08 | Médio | `principal_ref_hash` sem chave. | SDD anterior §6 | **Resolvido no design** (§6 linha 120, §12 linha 273). | SEC-ORG-19 |
| F-ORG-09 | Médio | União de grants entre posições. | SDD anterior §4, §7.4 | **Resolvido no design** (§4 linha 76, §7.4 linha 175 — SEC-ORG-21 adotado). | SEC-ORG-21 |
| F-ORG-10 | Médio | (Código) Traversal Neo4j sem filtro de agency; `/admin/graph/*` ignora `BOT_HTTP_AGENCY_ID`. | `core/database/graph_query.rs:25-34, 36-50`; `routes/graph_admin.rs` | **Aberto (débito de seam existente).** O `org` já exige o filtro (§6, §10); padrão atual não deve ser copiado. | SEC-ORG-04 |
| F-ORG-11 | Médio | (Código) Leituras de agents sem auth, agency da query. | `routes/agents.rs:20-26, 57-63, 75-85` | **Aberto (débito).** | SEC-ORG-02 |
| F-ORG-12 | Médio | (Código) Token admin compartilhado, sem principal/expiração; `Debug` com token. | `admin_auth.rs:7-12`; `core/config/http/file.rs:3-8` | **Aberto (débito)** — ver F-ADM-01. | SEC-ORG-18, 20 |
| F-ORG-13 | Médio | (Código) CEO reporta a qualquer `owner_id`; vários CEOs. | `hierarchy.rs:50-51`; `http_bridge/agents.rs:135-140` | **Aberto (débito)**; tratado no cutover (plano S16 cita F-ORG-13). | SEC-ORG-07, 25 |
| F-ORG-14 | Baixo | Relógio de expiração não fixado. | SDD anterior §7.5, §10 | **Resolvido no design** (§7.5 linha 192: `now()` do PG). | SEC-ORG-16 |
| F-ORG-15 | Baixo | Conflito de bootstrap loga IDs brutos. | `pg_owner_bootstrap.rs:33-37` | **Aberto (débito)**; S12 exige log pseudonimizado para o `org`. | SEC-ORG-23 |
| F-ORG-16 | Baixo | Erros HTTP com texto de driver. | `error.rs:211-214, 233-238, 258-262` | **Aberto (débito)**; o `org` exige erros genéricos (§10, plano S07). | SEC-ORG-18 |
| F-ORG-17 | Baixo | Revogação por CEO agente sem credencial P3. | SDD anterior §7.5 (d) | **Resolvido no design** (§7.5 (d): "CEO agente só com credencial P3 (SEC-ORG-26)"). | SEC-ORG-26 |
| F-ORG-18 | Médio (novo) | Semântica incompleta do assignment pendente (interpretação 1): o SDD torna os grants `reconciling`, mas não diz que o assignment pendente não confere a autoridade estrutural da posição atuante (matriz), aprovação ou delegação; não exige confirmador distinto do autor (owner que também ocupa `parent(T)` confirmaria o próprio assignment); não fixa prazo nem estado final do pendente. | SDD `905ed5e7` §7.3 (linha 159), §7.4 (linha 174), §20 (linha 364) | **Aberto** — incorporar SEC-ORG-28 ao SDD e às fatias S03/S05/S13 antes do LGTM de S05. | SEC-ORG-28 |
| F-ORG-19 | Médio (novo) | Recalcular a interseção em mudanças estruturais (interpretação 2): o SDD define o limite efetivo como interseção da cadeia (§7.5) e manda revalidar a subárvore no re-parent (§7.3), mas não liga re-parent, troca de versão de cargo de ancestral nem vacância a um incremento de epoch/versão de cadeia; o epoch só sobe em revoke/expiry (§7.5 linha 190). Um consumidor com cache chaveado no epoch mantém o escopo mais largo. | SDD `905ed5e7` §7.3 (linha 161), §7.5 (linhas 185, 190), §8 (linha 215) | **Aberto** — incorporar SEC-ORG-29 (S05, S06, S09, S11, S13, S17). | SEC-ORG-29 |
| F-ORG-20 | Médio (novo, N1) | Aprovação não ligada à cadeia: a policy liga-se à `JobTitleVersion` (§7.5 linha 181), e a troca de versão, o re-parent e o reuso da mesma versão sob outro superior podem ativar grants que o superior atual nunca aprovou. "Superior direto" (§7.5 linha 182) é ambíguo quando a versão é reaproveitada. O re-parent já deixa grants `reconciling` até o ocupante de N confirmar (§7.3 linha 161), mas a troca de versão pelo owner (linha 162) e o reuso não exigem consentimento da cadeia atual nem confirmador distinto, e o caso do sucessor não está definido. | SDD `905ed5e7` §7.3 (linhas 161-162), §7.5 (linhas 181, 182) | **Aberto** — incorporar SEC-ORG-30 (S03, S05, S09, S10, S13). | SEC-ORG-30 |

**Contagem (`905ed5e7`):** Crítico 0 · Alto 3 (F-ORG-01 e F-ORG-02 **resolvidos no design**; F-ORG-03 **aberto**, débito de `agents` fora do escopo do `org`) · Médio 13 (abertos: F-ORG-10, 11, 12, 13 como débito de código; F-ORG-18, 19, 20 novos no design) · Baixo 4 (abertos como débito: F-ORG-15, 16).

**Veredito de segurança do SDD `org` `905ed5e7`:** nenhum Crítico/Alto aberto **no design do `org`**. Condições: F-ORG-18, F-ORG-19 e F-ORG-20 (Médios) incorporados antes do LGTM de S03/S05/S06/S09/S10/S11/S13; parâmetros de §6.4 decididos pelo owner; P1 aprovado antes de S07, S10, S11, S12, S13, S14, S16 e S17 (F-ORG-02; plano, tabela de fatias, linhas 106 e 109-116). F-ORG-03 continua Alto aberto em `agents` e bloqueia qualquer reuso daqueles seams.

### 7.1 Interpretações do Architect

| # | Interpretação | Decisão de segurança | Base no SDD | Condições |
|---|---|---|---|---|
| 1 | Assignment feito pelo owner em posição não raiz fica `reconciling` até o superior confirmar. | **CONFIRMA, com condições.** | §7.3 (linha 159): owner → `reconciling` até o aprovador elegível confirmar; §7.4 (linha 174): gateway nega, confirmação é comando auditado, vacância mantém bloqueio; §7.3 (linha 160): raiz é `ActiveOnCommit` pelo owner, com owner vetado como ocupante; §3 D-OWNER-POSITION (linha 68); §20 (linha 364). | Fail-closed dos grants: **atendido** (§7.4). Sem deadlock para o owner: **atendido por construção** (o owner preenche a cadeia a partir da raiz, que é `ActiveOnCommit`; operações de owner não passam pela matriz de posição). D-OWNER-POSITION: **consistente** (o owner nunca ocupa a raiz; em outra posição, bloqueia a policy global). **Faltam** (F-ORG-18 → SEC-ORG-28): (a) assignment pendente não confere autoridade estrutural, aprovação nem delegação a partir de T; (b) confirmador distinto do autor, em especial o owner que também ocupa `parent(T)`; (c) prazo **D-SEC-PENDING-TTL** com estado final `expired` e audit. Se o owner preferir que a própria assinatura valha como confirmação (§20, linha 364), isso exige emenda a este documento e revisão do Critic. |
| 2 | `JobTitleVersion` reaproveitada sob outro superior tem limite efetivo igual à interseção com a cadeia. | **CONFIRMA, com as duas condições (N1):** não-crescimento (SEC-ORG-29) **e** consentimento da cadeia atual (SEC-ORG-30). | §7.5 (linha 185): "Limite efetivo = interseção de toda a cadeia ancestral … Nenhum descendente amplia"; §7.3 (linha 161): re-parent revalida as policies da subárvore como subconjunto da nova cadeia, senão `reconciling`; §7.3 (linha 162): troca de versão deixa grants antigos `reconciling`; §8 (linhas 214-215): revalidação imediatamente antes de cada efeito e invalidação de cache; §7.5 (linha 192): revogação ancestral invalida descendentes. | Monotonicidade: **atendida** no texto. Enforcement no ponto de decisão: **atendido** para o runtime (§8); precisa valer também para os efeitos internos do `org` (S11). **Faltam** (F-ORG-19 → SEC-ORG-29): incremento de epoch/versão de cadeia, na mesma transação, em re-parent, troca de versão de cargo de ancestral, nova versão ou revogação de policy ancestral e vacância de ancestral, para que nenhum cache mantenha o escopo antigo; property test de monotonicidade. **Consentimento** (F-ORG-20 → SEC-ORG-30): a mesma versão sob um segundo superior sem aprovação dele fica `reconciling`/**403** (teste SEC-ORG-29 (d) corrigido); sucessor herda a aprovação da posição com lista de revisão auditada e revogação com epoch. |

### 7.2 Backlog `agents` (G5)

| ID proposto | Origem | Conteúdo | Onde registrar |
|---|---|---|---|
| **AGT-SEC-01** | F-ORG-03 | Remover autoridade vinda do cliente em `agents`: `owner_id` do corpo como prova de owner, `promote_runtime_bot` definido pelo próprio registrante e `promoted_by` escolhido pelo cliente; capabilities só por fluxo autorizado com principal verificado (P1/P3). Critérios: SEC-ORG-01, SEC-OWN-01, SEC-OWN-02. | Checklist "Critérios de fechamento G1" de [`sdd/agents-module-sdd.md`](../sdd/agents-module-sdd.md) (linha 137). Este documento **não** edita aquele SDD (N7); o coordenador encaminha ao Architect o texto abaixo, pronto para colar. |

**Texto pronto para colar** (linha da tabela `| Critério | Evidência atual | Fechado |` em `agents-module-sdd.md`, seção "Critérios de fechamento G1"):

```markdown
| **AGT-SEC-01** — sem autoridade vinda do cliente em `agents` (F-ORG-03): `owner_id` do corpo não prova owner; `promote_runtime_bot` não é definido pelo próprio registrante; `promoted_by` não é escolhido pelo cliente; capabilities só por fluxo autorizado com principal verificado (P1 humano, P3 agente). Aceite: SEC-ORG-01, SEC-OWN-01, SEC-OWN-02 ([org threat model](../security/org-module-threat-model.md), [owner auth](../security/owner-auth-idp-threat-model.md)). | Nenhuma: `routes/agents.rs:38-48`, `http_bridge/agents.rs:30-41`, `register_owner.rs:5-16, 32-48`, `bot_promotion.rs:5-25` ainda aceitam esses campos do cliente. | Não |
```

### Histórico: endereçado na revisão `f853319e`

Dual control com principais distintos e bloqueio quando o owner ocupa posição (§7.5 (a)); aprovação de agente rejeitada até P3 (§7.5); reconhecimento da janela TOCTOU (§7.5); tabela `org_revocation_epochs` listada (§11); filtro de `org_id` em traversal Neo4j + teste cross-org (§6, §10); auditoria append-only por privilégio com teste (§11); `VerifiedProductOwner` não autoriza `org` (§3 D-OWNER-BIND); remoção da projeção de grants para `agents` (§5); plano alinhado à errata. Em `03bd020a`: exceção do CEO explicitada em §7.5 (b), D-OWNER-POSITION formalizada em §3 e autoteste reproduzível do `check-import-direction.sh` em §9. Este último é controle de arquitetura (import direction), não de segurança, e não gera critério SEC-ORG.

## 8. Dependência de owner auth (P1)

**Modelo de ameaças de P1:** [owner-auth-idp-threat-model](./owner-auth-idp-threat-model.md) (F-OWN-*, SEC-OWN-*).

**O que o `org` assume de P1:** verificador que valida `iss`, `aud`, `exp`/`nbf`, assinatura e rotação de chaves e produz `HumanPrincipal(issuer, subject)`; enrollment/bootstrap do owner por organização sem endpoint público de autoelevação (D-OWNER-BIND); revogação de sessão/credencial por principal com checagem por request (F-ORG-02); `AuthContext` tipado até os controllers. Autenticação de **agentes** fica em P3 (§14) — até lá, aprovações (e, por este modelo, revogações) atribuídas a agentes são rejeitadas.

**O que quebra sem P1:** não há actor verificável; dual control, audit de ator, isolamento de tenant e não-repúdio viram declarações do cliente. O único seam disponível (`BOT_HTTP_ADMIN_TOKEN`) é compartilhado, pode estar ausente (fail-open, [F-ADM-01](./admin-http-auth-fail-open.md)) e não identifica quem age; o owner singleton atual é uma string de env.

**Bloquear até P1 aprovado e implementado (com Critic):** fatias S07, S10, S11, S12, S13, S14, S16 e S17 (plano, linhas 106 e 109-116; N6); todas as rotas HTTP mutantes do `org` e leituras privadas/histórico (SEC-ORG-20); approvals humanos e publicação de policy global; transferência de owner; cutover (§13); qualquer consumo de grants/epoch pelo runtime (também depende de P3).

**Pode prosseguir antes de P1:** tipos e invariantes puros (§14 "Não dependem"), schema aditivo sem writes (condicionado a P2) e revisão deste modelo.

## 9. Limitações

- SDD em draft, não rastreado e em edição ativa; quatro versões lidas no mesmo dia. Hash revisado registrado em §0 (`905ed5e7`).
- Não há código `org`; achados de código são dos seams existentes.
- Não verificados: roles/privilégios PG reais, logging do servidor PG, deploy/bind real, IdP candidato. Nenhum teste executado.

## 10. Próximos passos

1. Orquestrador atribui Critic de segurança independente (até 3 ciclos).
2. Architect incorpora F-ORG-18/19/20 (SEC-ORG-27…30), a extensão N2 de SEC-ORG-05, os itens de §6.3 e as divergências de §6.2; owner decide os parâmetros de §6.4; F-ADM-01 corrigido antes de qualquer rota `org`; dono do `agents` registra AGT-SEC-01.
3. Cada SEC-ORG vira teste red→green na fatia correspondente (AGENTS.md G3).

## 11. Registro do ciclo 2 do Critic (TM-ORG, follow-ups N1-N8)

| Item | O que mudou | Seção |
|---|---|---|
| N1 | Novo SEC-ORG-30 e achado F-ORG-20 (Médio): aprovação ligada à tupla (`JobTitleVersion`, `parent(T)`, versão da posição aprovadora), `reconciling` até o aprovador atual aprovar, aprovador distinto de quem mudou, regra do sucessor, leitura de "superior direto"; SEC-ORG-29 (d) corrigido; interpretação 2 confirmada com as duas condições | §6.1, §6.2, §6.3, §7, §7.1 |
| N2 | SEC-ORG-05 estendido: transfer com autoridade na origem e no destino, encerramento só vago e sem descendentes, re-parent de `OrgUnit`; casos (f)(g)(h) | §6.1, §6.3 |
| N3 | Mapa: 27 → S03 (implementa), S05/S13 (persiste); 28 → +S11; 29 → +S05/S13 (gatilhos) e S09 (property test); 30 → S03, S05, S09, S10, S13 | §6.2 |
| N4 | §6.3: 04, 07, 08, 09 e 20 na lista de cobertos; 20 parcial (falta startup falhar com feature ligada sem verificador); 04 parcial (falta a regra "sem `format!`") | §6.3 |
| N5 | SEC-ORG-28 (b) mantido e amarrado a D-OWNER-POSITION; novo (b′): owner agindo por `Position(parent(T))` → `ActiveOnCommit` auditado com `actor_is_owner` | §6.1 |
| N6 | Gate P1 inclui S11, S14, S16, S17 (plano, linhas 110, 113, 115, 116) | §7 veredito, §8 |
| N7 | `agents-module-sdd.md` não editado; texto do AGT-SEC-01 pronto para colar | §7.2 |
| N8 | Estado e Critic no cabeçalho atualizados | cabeçalho |
