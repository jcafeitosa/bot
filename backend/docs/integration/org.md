---
title: Integração — módulo org (seams, contratos, D-OUTBOX e D-SEAM-AGENTS)
description: Spec de integração do módulo org com HTTP, ApiErrorBody, auth W0-01 classe Org, extractor unknown_field, outbox e agents
tags:
  - integration
  - backend
  - org
  - agents
  - outbox
  - http
status: draft
---

# Integração — módulo `org`

- **Papel:** Integrador. **Estado:** Pendente de revisão do Critic.
- **Natureza:** especificação. Não aprova SDD, plano, threat model nem gate. Não decide schema de tabela, migration, índice ou transação (DBA), nem achado ou severidade de segurança (Segurança), nem conteúdo de SDD/ADR/plano (Arquiteto). Onde esses temas aparecem, eles aparecem como observação com evidência.
- **Regra de leitura:** quando doc e código divergem, o código é o fato; a divergência fica registrada com `arquivo:linha`.

## 1. Escopo e fontes

### 1.1 Entradas lidas

| Entrada | Caminho | sha256 lido | Esperado | Situação |
|---|---|---|---|---|
| SDD `org` | [org-module-sdd](../sdd/org-module-sdd.md) | `d42291d751360d937bb0e6f526f91666bf511b1ce1164de6c2aa16d4bd4a00bd` | `d42291d7` | confere |
| Plano `org` | [org-complete-implementation-plan](../planning/org-complete-implementation-plan.md) | `a7ca91fcc92311d77a2d949ba34c46d1d773b3eda701892cb3b02fe26416b5cd` | `a7ca91fc` | confere |
| Threat model `org` (só contexto) | [org-module-threat-model](../security/org-module-threat-model.md) | `50140e33f8b6760a26543fe9e1883958853544b705543d2f6dac320612e2163a` | `276a98f6` | **não confere** (ver §8, item 10) |
| SDD W0-01 (admin auth) | [http-admin-auth-seam-sdd](../sdd/http-admin-auth-seam-sdd.md) | `e14a8d028e7e28c41fa50c8b411f0596e3d2eb87654bda1f216d625b23ac5628` | — | lido |
| SDD outbox F2.1 | [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md) | não medido | — | lido |
| SDD convenção MVC | [modules-mvc-convention-sdd](../sdd/modules-mvc-convention-sdd.md) | não medido | — | §2.4 lido |

Checkout: `HEAD d42b71a5`. O SDD, o plano e o diretório `backend/docs/security/` estão não rastreados no git no momento da leitura; o conteúdo pode mudar sem commit.

### 1.2 Código lido (fatos)

`backend/src/presentation/http/{error.rs,admin_auth.rs,server.rs,state.rs,routes/mod.rs,routes/agents.rs}`, `backend/src/modules/{mod.rs,application_contracts.rs,agents/mod.rs}`, `backend/src/modules/agents/adapters/pg_registry.rs`, `backend/src/core/database/{graph_projection_outbox.rs,graph_projection_outbox_worker.rs,graph_projection.rs}`, `backend/src/core/database/migrations/0009_graph_projection_outbox.sql` (só leitura, DBA é dono), `backend/scripts/check-import-direction.sh`, `backend/Cargo.toml`, `backend/Cargo.lock`.

**Fato central:** não existe módulo `org` no código. `backend/src/modules/mod.rs:1-13` não declara `org`; `rg` por `modules::org`, `/org` e `deny_unknown_fields|JsonRejection|WithRejection|unknown_field` em `backend/src` não retorna nada. Todo seam de `org` abaixo é **planejado**; os seams vizinhos (HTTP, erro, auth admin, outbox, agents) **existem**.

### 1.3 Graphify (rodado em `backend/`)

| Comando | Resultado usado como evidência |
|---|---|
| `graphify query "who calls the org module and what does org depend on"` | 4 nós, todos docs (`org-module-sdd.md`, `org-module-threat-model.md`, `org-module-capability-analysis.md`, `modules-completeness-audit.md`); nenhum nó de código |
| `graphify explain "org"` | ambíguo entre 4 nós, todos em `docs/` |
| `graphify path "org" "agents"` e `graphify path "org" "ApiErrorBody"` | nenhum caminho dirigido |
| `graphify query "ApiErrorBody error mapping http"` | `ApiErrorBody` em `src/presentation/http/error.rs` L12, `ApiError` L19, consumidores em `routes/*.rs` |
| `graphify path "src/presentation/http/error.rs::ApiError" "src/presentation/http/error.rs::ApiErrorBody" --undirected` | 1 salto: `ApiError --references--> ApiErrorBody` |
| `graphify query "admin auth extractor seam RequireAdmin"` | `HttpAdminAuth` em `admin_auth.rs` L8, `ApiStateInner` em `state.rs` L124, nó doc "Seams públicos para acordo antes do TDD" do W0-01 |
| `graphify query "outbox event enqueue"` | `enqueue_graph_projection_outbox_tx()` em `graph_projection_outbox.rs` L117, chamado por `pg_registry.rs` L26, `pg_idempotency.rs` L127/L84, `pg_catalog.rs` L44/L58/L70 |
| `graphify explain ".../pg_registry.rs::persist_identity_and_enqueue_graph_projection"` | chama `enqueue_graph_projection_outbox_tx()` (L71, INFERRED); testado por `pg_agent_identity_and_graph_projection_same_transaction()` (L378) |
| `graphify path "register_agent" "enqueue_graph_projection_outbox_tx" --undirected` | 4 saltos: `register_agent() → AgentRegistry → AgentDefinition ← persist_identity_and_enqueue_graph_projection() → enqueue_graph_projection_outbox_tx()` |
| `graphify path "retire_agent" "persist_identity_and_enqueue_graph_projection" --undirected` | 3 saltos via `AgentRegistry → AgentDefinition` |
| `graphify explain ".../supervisor_hook.rs::MonitorAgentHook"` | trait de `agents` implementada por `RegistryMonitorAgentHook` e consumida por `monitor/controllers/supervisor.rs` (L344, L866, L874, L1140): precedente de hook em que o módulo chamado é dono da trait |
| `graphify query "org module integration seams D-OUTBOX D-SEAM-AGENTS" --graph docs/graphify-out/graph.json` | `org-module-sdd.md` na comunidade `http-admin-auth-seam-sdd` |

### 1.4 OpenKnowledge e Mantis

- OpenKnowledge MCP unavailable: a descoberta de ferramentas não encontrou servidor `open-knowledge` registrado nesta sessão e o servidor OK não está rodando (`ok status`: `server not running`). Este documento foi lido e escrito com ferramentas nativas, e `ok start` não foi executado por regra da tarefa.
- Mantis: `mantis-advise` exige `$MANTIS_HOME/reference/scripts/advise.py` e `knowledge.db`; `MANTIS_HOME` não está definido e não há `knowledge.db`. `mantis-architecture` escreve em `workspace/kb/`, fora do escopo de escrita desta tarefa. **Nenhum Mantis foi rodado**; nenhum resultado Mantis é citado.

## 2. Mapa de seams do `org`

Legenda: **existe** = presente no código hoje; **planejado** = só no SDD/plano.

| # | Chamador → chamado | Seam | Estado | Evidência |
|---|---|---|---|---|
| S-1 | cliente HTTP → `org` | rotas `/api/v1/org*` em `presentation/http/routes/org.rs` | planejado | SDD §9 L265; hoje `routes/mod.rs:25-93` não tem rota `org`; prefixo `/api/v1` em `server.rs:86` (existe) |
| S-2 | router → auth | layer de auth W0-01 com tabela única e classe `Org` | planejado (W0-01 também é `draft`) | SDD L240; W0-01 L53-54; hoje cada handler chama `state.require_http_admin` à mão (`state.rs:800-802`, `routes/agents.rs:43`) e o seam é fail-open sem token (`admin_auth.rs:91-94`) |
| S-3 | auth → verificador P1 | bearer P1 validado; produz `AuthorizedOrgScope` | planejado (P1 não existe) | SDD §8 L236-240; não há verificador P1 em `backend/src` |
| S-4 | handler → extractor JSON | extractor próprio `400 unknown_field` + `deny_unknown_fields` | planejado | SDD L269; `rg` vazio; axum `0.8` (`Cargo.toml:34`, `0.8.9` em `Cargo.lock:394-395`) |
| S-5 | handler/bridge → erro HTTP | `ApiError::with_code` → `ApiErrorBody { error, code }` | existe (tipo); mapeamento `org` planejado | `error.rs:11-16`, `error.rs:35-47`, `error.rs:303-307`; graph: `ApiError → ApiErrorBody` (1 salto) |
| S-6 | rotas → `org` | acesso via `ApiState`/`http_bridge`; rota não importa `modules::org::` | planejado; padrão existe para `agents` | SDD L265; `check-import-direction.sh:25` só cobre `modules::agents::` |
| S-7 | boot → `org` | `ApiState::build_api_state_for_http_serve` e tarefa periódica de TTL no `serve` | planejado; padrão existe | `server.rs:21-25` (bootstrap e estado), `server.rs:30-53` (poll de reconciliação citado no SDD L217) |
| S-8 | `org` adapters → `core::database` | pool, transação, runner de migrations | planejado; seam existe | SDD §9 L266; `AppDatabases` em `server.rs:21` |
| S-9 | `org` adapters → outbox | `enqueue_graph_projection_outbox_tx` com `graph_domain=org` (D-OUTBOX) | planejado; seam existe para agents/orders/bots | SDD L79; `graph_projection_outbox.rs:117-141`; graph: callers `pg_registry.rs`, `pg_idempotency.rs`, `pg_catalog.rs` |
| S-10 | outbox → Neo4j | drain + `GraphProjectionPort` (novo método ou evento para `org`) | planejado; seam existe | `graph_projection_outbox.rs:180-275`, `graph_projection.rs:79`; plano S08 L264 |
| S-11 | `org::models` → `agents` | tipos `AgentId`, `AgencyId`, `AgentLifecycleState` (D-SEAM-AGENTS) | planejado | SDD L83; re-exports em `agents/mod.rs:16-20` |
| S-12 | `agents` (retire) → `org` (encerrar assignments) | mesma transação auditada (SDD §4) | planejado; **mecanismo não definido** | SDD L93; retire hoje: `lifecycle.rs:53`, `state.rs:1080`, persistência em TX própria `pg_registry.rs:26-34` |
| S-13 | runtime/tool gateway (P3) → `org` | leitura de grant/epoch no PG `org`; `authz_unavailable` | planejado (P3) | SDD §8 L245-252, §10 L295 |
| S-14 | `org` → consumidores de revogação | `revocation_complete` com ack de consumidores | planejado (P3-b) | SDD L250; plano L103 |

## 3. Contratos por seam

Regras comuns a todos os seams de `org`:

- **Timeout:** PG indisponível em autorização → deny em até `D-SEC-AUTHZ-TIMEOUT` (valor do owner, SDD L84, L226). Nenhum outro timeout numérico está definido no SDD; este documento não fixa números.
- **Retry interno:** só para serialization failure em `SERIALIZABLE`, com limite (SDD L158, L216). Qualquer outro erro sobe como está.
- **Retry do cliente:** comandos repetíveis levam idempotency key com escopo `UNIQUE (org_id, key)` (SDD L224, SEC-ORG-24; plano S05 L247). A semântica de replay não está definida no SDD (ver §9, pergunta 5).
- **Ordenação:** mutações da árvore e aprovações da mesma organização são serializadas por lock transacional da organização + `SERIALIZABLE` (SDD L158, L216). Não há ordenação entre organizações.

### 3a. Mapeamento de erros do `org` para `ApiErrorBody`

**Forma do corpo (existe):** `ApiErrorBody { error: String, code: Option<String> }`, com `code` omitido do JSON quando `None` (`error.rs:11-16`); construído por `ApiError::with_code(status, code, error)` (`error.rs:35-47`); serializado como JSON por `IntoResponse` (`error.rs:303-307`).

**Contrato proposto para `org`** (derivado do SDD §10 L286-297 e dos códigos das fatias do plano; nenhum código novo é criado aqui):

| Erro de domínio (nome indicativo, plano S01 L209) | HTTP | `code` | Campos do corpo | Fonte |
|---|---|---|---|---|
| credencial P1 ausente, inválida, expirada; token admin em rota `Org` | 401 | `unauthorized` | `error` genérico, `code` | SDD L290 |
| sem verificador P1 | 503 | `owner_auth_required` | idem | SDD L240, L295 |
| org alheia ou inexistente | 404 | `org_not_found` | idem; sem ler linhas da outra org | SDD L239, L292 |
| campo desconhecido no corpo | 400 | `unknown_field` | idem; **o nome do campo não é exigido pelo SDD** | SDD L269, L294 |
| `RootPositionImmutable` | 409 | `root_position_immutable` | idem | SDD L50, L157 |
| `ForbiddenScope` | 403 | `forbidden_scope` | idem | SDD L291 |
| `SelfAssignmentForbidden` | 403 | `self_assignment_forbidden` | idem | SDD L291 |
| `AssignmentPending` | 403 | `assignment_pending` | idem | SDD L291 |
| `ConfirmerNotIndependent` | 403 | `confirmer_not_independent` | idem | SDD L291 |
| `ApproverNotIndependent` | 403 | `approver_not_independent` | idem | SDD L54, L291 |
| `PositionNotVacant` | 409 | `position_not_vacant` | idem | SDD L293 |
| `HasDescendants` | 409 | `has_descendants` | idem | SDD L293 |
| `HierarchyConflict` | 409 | `hierarchy_conflict` | idem | SDD L293 |
| `ApprovalChainMismatch` | 409 | `approval_chain_mismatch` | idem | SDD L293 |
| `PolicyHashMismatch` | 409 | `policy_hash_mismatch` | idem | SDD L293 |
| `PolicyExceedsAncestor` | 422 | `policy_exceeds_ancestor` | idem | SDD L294 |
| falha de PG no store `org` | 503 | `org_store_unavailable` | `error` sem texto do driver | SDD L241, L295 |
| PG indisponível na read API do runtime | 503 | `authz_unavailable` | idem | SDD L226, L295 |
| pai de outra org | 400 ou 404 uniforme | não fixado no SDD | idem | SDD L184, L278 |

**Precedência (SDD L297):** (1) autenticação, verificador, tenant (`401`, `503 owner_auth_required`, `404 org_not_found`); (2) corpo (`400 unknown_field`); (3) `409 root_position_immutable`; (4) autorização, sempre `403`; (5) conflito de estado `409` e limite `422`.

**Garantias de estabilidade exigidas:**

1. `code` é sempre `Some` em erro de `org` (SDD L286). O cliente decide por `status` + `code`; o texto de `error` pode mudar sem aviso.
2. Um `code` publicado não muda de status nem de significado. Renomear ou remover um `code` é mudança incompatível e exige nova versão de contrato no OpenAPI (SDD L275 exige OpenAPI antes de cada fatia HTTP).
3. Erros de domínio distintos nunca colapsam no mesmo `code`. Precedente a **não** copiar: `from_agents_error` junta `InvalidId`, `Hierarchy` e `Lifecycle` em `400 "agents"` (`error.rs:98-102`), e `from_bots_error` junta sete variantes em `"bots"` (`error.rs:159-185`).
4. O mapeamento `OrgDomainError → (status, code)` é uma função total (um braço por variante, sem `_ =>`), para que variante nova quebre a compilação em vez de cair num padrão genérico.

**O que nunca pode vazar no corpo:** texto do driver PG/Neo4j, `issuer`/`subject` brutos, `principal_ref`, token ou bearer, nome de outra organização ou prova da existência dela, IDs de outra org, detalhes da matriz (qual regra negou além do `code`). Precedentes a **não** copiar: `AgentsError::Persistence(message)` vai direto para o corpo (`error.rs:93-97`); `OrdersError::StoreUnavailable(message)` idem (`error.rs:211-215`); o mesmo em `error.rs:233-238` e `error.rs:258-262` (o threat model registra isso como F-ORG-16; a análise é do Segurança).

**Lacunas SDD × código neste seam:**

- `ApiError::unauthorized()` tem a mensagem fixa `"missing or invalid admin bearer token"` (`error.rs:130-136`). Se `org` reutilizar esse construtor para o bearer P1, o `error` fala de token admin numa rota que não aceita token admin. O contrato de `org` precisa de `401 unauthorized` com mensagem genérica, sem citar o tipo de credencial esperado.
- O SDD não define `code` para JSON malformado, tipo errado, campo obrigatório ausente nem `Content-Type` ausente (só `unknown_field`). O `Json` do axum 0.8 responde esses casos com 4xx em texto puro, fora de `ApiErrorBody` (SDD L269; comportamento do axum não testado aqui). Ver §9, pergunta 3.
- O SDD não define o `code` do "pai de outra org" (`400`/`404` uniforme, L184, L278).
- `status_code()` e `error_code()` de `ApiError` só existem em `#[cfg(test)]` (`error.rs:292-301`); servem para testes unitários do mapeamento, não para lógica de produção.

### 3b. Seam de admin auth W0-01 com a classe `Org`

**Estado do código:** não há tabela de rotas nem layer de auth. `v1_routes()` registra rota a rota (`routes/mod.rs:25-93`); `build_router` aplica só `TraceLayer` (`server.rs:81-94`); cada handler protegido chama `require_http_admin` (`state.rs:800-802`), que devolve `Ok(())` quando não há token (`admin_auth.rs:91-94`). O W0-01 que cria a tabela está `w0_01_status: draft` no próprio SDD W0-01.

**O que as rotas `org` exigem do seam (contrato):**

| Item | Contrato | Fonte |
|---|---|---|
| Registro | toda rota `/api/v1/org*` está na tabela única do W0-01 com classe `Org` e política | SDD L240; W0-01 L53 |
| Credencial aceita | só o bearer P1 validado pelo verificador P1 (agente só depois de P3) | SDD L240 |
| Sem fallback | token admin numa rota `Org` → `401 unauthorized`; bearer P1 numa rota `Protected` → `401 unauthorized` | SDD L240; TM SEC-ORG-33 (contexto) |
| Sem verificador P1 | `503 owner_auth_required`; e feature `org` ligada sem verificador faz o startup falhar | SDD L240, SEC-ORG-20 |
| Fora da tabela | rota `org` fora da tabela ou sem política → nega | SDD L240; W0-01 L54 (`route_not_in_table`) |
| Tenant | binding Organization/Agency por request; org alheia e inexistente → `404 org_not_found`, sem ler linhas da outra org | SDD L239 |
| Saída para o handler | `AuthorizedOrgScope`, único construível fora de testes pela camada de auth, exigido por todo repositório | SDD L239, L304 |
| 401 | `ApiErrorBody { error: <genérico>, code: "unauthorized" }` | SDD L290 |
| 403 | só depois de autenticação, tenant, corpo e INV-ROOT; `code` da tabela de §3a | SDD L297 |

**Ordem em relação ao parse do corpo (contrato):** a verificação de credencial e de tenant roda **antes** de qualquer leitura do corpo. Concretamente: ou numa layer do router (como o W0-01 propõe para `Protected`, W0-01 L54), ou num extractor `FromRequestParts` que aparece antes do extractor de corpo na assinatura do handler. Nunca como chamada dentro do corpo do handler depois de `Json(...)`.

**Evidência do bug de ordem no código atual (não copiar):** `register_agent` recebe `Json(body): Json<RegisterAgentRequest>` como argumento e só depois chama `state.require_http_admin(&headers)` (`routes/agents.rs:38-43`). O axum extrai o corpo antes de o handler rodar; um chamador sem credencial com corpo inválido recebe a rejeição do axum em vez de `401`.

**Lacunas SDD × W0-01 neste seam:**

- A precedência do SDD (L297) começa em autenticação. O W0-01 põe antes dela a layer de `Host` (`421 host_not_allowed`, W0-01 L155, L175) e o limitador de falhas (`429`, W0-01 L162, L176). O SDD `org` não diz se essas duas camadas valem para rotas `Org`, nem se falhas do bearer P1 contam no limitador global. Proposta de contrato: `421` e `429` vêm antes da precedência (1) de `org`; a decisão é do Arquiteto com o Segurança.
- O W0-01 responde `401` para path inexistente sem credencial (W0-01 L54). Para path `/api/v1/org*` inexistente, não está definido qual verificador julga a credencial (admin ou P1).
- A tabela do W0-01 hoje tem só `Protected | PublicCompute | PublicRead` (W0-01 L53). A classe `Org` é follow-up cross-SDD que o SDD `org` registra e não edita (SDD L240).

### 3c. Extractor de `unknown_field`

**Estado do código:** não há `JsonRejection`, `WithRejection` nem `deny_unknown_fields` em `backend/src` (`rg` vazio, confirmado nesta leitura). Hoje campo desconhecido é ignorado pelo serde (SDD L269).

**Contrato:**

| Item | Contrato |
|---|---|
| DTOs de entrada de `org` | todos com `#[serde(deny_unknown_fields)]`, inclusive objetos aninhados (o atributo não se propaga sozinho para structs internas) |
| Campos proibidos | nenhum DTO tem `actor`, `owner_id`, `approved_by`, `org_id` de autoridade; se o cliente mandar, é campo desconhecido (SDD L284; TM SEC-ORG-01, contexto) |
| Resposta | `400` + `ApiErrorBody { error: <genérico>, code: "unknown_field" }`, em JSON, nunca `422` em texto puro do axum |
| Onde fica | extractor próprio definido em S07 e reutilizado em S13 (SDD L269; plano S07 L258, S13 L294) |
| Ordem | roda depois de auth e tenant (precedência 2 de SDD L297); antes de qualquer regra de domínio, inclusive `409 root_position_immutable` |
| Efeito | zero linhas alteradas; o SDD exige audit de deny para negações de mutação (SDD L189, SEC-ORG-17); se `400 unknown_field` gera audit não está explícito (observação para o Arquiteto) |
| Eco do campo | o corpo não precisa ecoar o nome do campo; se ecoar, nunca ecoa o valor |

**Interação com `ApiErrorBody`:** o extractor converte a rejeição em `ApiError::with_code(StatusCode::BAD_REQUEST, "unknown_field", …)`; assim a resposta passa pelo mesmo `IntoResponse` (`error.rs:303-307`).

**Interação com auth:** com credencial ausente e corpo com campo desconhecido, a resposta é `401`, não `400`. Com bearer P1 de outra org e campo desconhecido, a resposta é `404 org_not_found`, não `400`.

### 3d. Config/boot wiring

**Existe hoje:** `run` faz `AppDatabases::bootstrap_runtime()` (`server.rs:21`), sobe o worker do outbox (`server.rs:22`), monta `ApiState::build_api_state_for_http_serve` (`server.rs:24-25`), opcionalmente sobe o poll de reconciliação com `tokio::spawn` e `warn` em erro (`server.rs:30-53`) e monta o router (`server.rs:60`, `server.rs:81-94`). `ApiStateInner` guarda `databases`, `agents` e `http_admin_auth` (`state.rs:124-131`).

**Contrato para `org`:**

| Item | Contrato | Fonte |
|---|---|---|
| Pool e transação | só `core::database`; sem pool, runner, dotenv ou bootstrap próprios | SDD L266 |
| Config | chave HMAC de `principal_ref`, TTLs e intervalo da varredura vêm de `core::config` validado | SDD L135, L217 |
| Estado HTTP | `org` entra no `ApiState` (ou bridge); rotas não importam `modules::org::` | SDD L265 |
| Feature gate | `org` desligado por padrão; ligado sem verificador P1 → startup falha antes de abrir socket | SDD L240; plano S13 L294-295 |
| Varredura de TTL | tarefa periódica no `serve`, no padrão de `server.rs:30-53`, com lock + `SERIALIZABLE`; a decisão de autorização nunca depende dela (lê `now()` do PG) | SDD L217 |
| Router | rotas `org` entram pela tabela do W0-01, não por `.route(` avulso | SDD L240; W0-01 L53, L56 |

**Lacuna:** o padrão de tarefa periódica existente (`server.rs:33-46`) descarta o `JoinHandle` e só registra `warn`; se a tarefa entrar em pânico, ninguém fica sabendo. Para `org`, a correção não depende da varredura (SDD L217), mas a materialização de `expired` e o bump de epoch param em silêncio (ver §6, M-3).

## 4. D-OUTBOX

**Pergunta (SDD L79; plano L69, dependentes S05, S08, S11):** como `org` publica mudanças para fora do PG. A tabela de outbox, índices e transações são do DBA; aqui só entra o **contrato de evento**.

**Fato do seam central (código):**

- Mensagem = `graph_domain`, `event_kind`, `idempotency_key`, `payload` (`graph_projection_outbox.rs:37-43`); `payload` é enum com `#[serde(tag = "kind")]` e uma variante por domínio (`graph_projection_outbox.rs:27-35`); domínios `agents`, `bots`, `trading` (`graph_projection.rs:6-12`).
- Enqueue faz `ON CONFLICT (graph_domain, idempotency_key) DO UPDATE` do payload e reabre `pending` (`graph_projection_outbox.rs:123-131`). Com chave por agregado (ex.: `agent:{agency}:{agent}`, `graph_projection_outbox.rs:47`), a outbox guarda o **último estado** do agregado, não cada evento.
- Drain: um por vez, `ORDER BY id … FOR UPDATE SKIP LOCKED` (`graph_projection_outbox.rs:206-211`), marca `processing` e comita (`:230-235`), aplica, marca `done` ou `retry` com `attempt_count + 1` (`:244-270`). Entrega **pelo menos uma vez**; sem DLQ, sem limite de tentativas, sem backoff.

| | A — outbox central, `graph_domain=org`, mesma TX | B — outbox própria `org` (log de eventos) | C — sem eventos nas fatias S05–S07; projeção começa no S08 por rebuild + A | D — chamada direta pós-commit (`graph_projection_best_effort`) |
|---|---|---|---|---|
| Onde está hoje | seam existe (`graph_projection_outbox.rs:117-141`); usado por agents/orders/bots | não existe | não existe; rebuild PG→Neo4j por domínio é exigência do SDD (L98) | existe (`graph_projection_outbox.rs:276-294`) |
| Nome do evento | `event_kind` por agregado, no padrão existente: `org_unit`, `org_position`, `org_job_title_version`, `org_assignment` (proposta) | nome versionado por evento, ex. `org.assignment.ended.v1` (proposta) | nenhum até S08; depois como A | como A |
| Versão | campo `aggregate_version` monotônico no payload; mudança incompatível = novo `event_kind` (ex. `org_position_v2`); campos novos só aditivos com default | versão no nome + `schema_version` | como A | como A |
| Payload | IDs estáveis, `org_id`, versão, estado, timestamps; `principal_ref`, nunca `issuer`/`subject`, nome ou email (SDD L135, L321) | idem + `event_id`, `seq` | como A | como A |
| Chave de idempotência | `org:{org_id}:{kind}:{aggregate_id}` (proposta); estável por agregado | `event_id` único | como A | como A |
| Ordenação | por agregado: uma linha por chave; consumidor rejeita `aggregate_version` menor que o projetado (plano S08 L265, "out-of-order version rejected"); entre agregados: sem garantia | por `seq` da organização ou por agente (P3-b, plano L103) | como A | nenhuma |
| Entrega | pelo menos uma vez; consumidor = `MERGE` idempotente com guarda de versão | pelo menos uma vez + ack por consumidor registrado | como A | no máximo uma vez; falha vira `warn` |
| Dedupe no consumidor | guarda de versão por agregado | por `event_id` | como A | não há |
| Modos de falha | herda os defeitos do drain central (§6, M-1, M-2, M-6); estado final de agregado encerrado precisa ser representado como estado (ex. assignment `ended`), nunca como remoção de linha | complexidade nova, tabela e worker novos; consumidores de ack ainda não existem (P3-b) | S05–S07 sem projeção: leituras de traversal dependem do S08 de qualquer forma (plano S07 L259 usa a porta do S08) | perda silenciosa se o enqueue falhar depois do commit (`graph_projection_outbox.rs:287-293`); contradiz SDD L98 |
| Custo | baixo: nova variante do enum em `core::database`, novo método em `GraphProjectionPort` (`graph_projection.rs:79`), novo `graph_domain` | alto: tabela (DBA), worker, protocolo de ack, observabilidade | baixo no S05; no S08 igual a A + backfill | mínimo, mas não atende o SDD |
| Serve para `revocation_complete` | não (sem ack de consumidor) | sim | não | não |

**Observações para o DBA (não decididas aqui):** o formato da tabela, o índice `status,id` e a transação de enqueue são do DBA (`0009_graph_projection_outbox.sql:2-19`). Os defeitos de drain de §6 (M-1, M-2, M-6) estão no código de `core::database`, não na tabela; quem corrige é decisão do Orquestrador.

**Recomendação para o Julio:** **A**, com três condições de contrato: (1) chave por agregado + `aggregate_version` no payload + guarda de versão no consumidor; (2) a variante do payload, o método da porta e os testes T-12 a T-17 (§7) entram na mesma fatia que fizer o primeiro enqueue de `org`, porque o `match` do drain é exaustivo (`graph_projection_outbox.rs:327-338`) e uma linha de `org` sem tratador não pode ser gravada; (3) os defeitos M-1, M-2 e M-6 do drain central são corrigidos antes do primeiro enqueue de `org`. **B** fica reservada para quando o SDD de P3-b exigir ack de consumidor para `revocation_complete`; não antes. A outbox nunca autoriza nada: revogação vale pelo epoch no PG (SDD L224).

## 5. D-SEAM-AGENTS

**Pergunta (SDD L83; plano L68, dependente S01):** como `org` usa tipos de `agents`, e como os dois se chamam sem ciclo.

**Fatos:**

- `agents` reexporta tipos puros: `AgencyId`, `AgentId`, `AgentLifecycleState` e outros (`agents/mod.rs:16-20`).
- `agents` não importa nenhum outro módulo de domínio (só `agents` e uma menção a `bots` em comentário, `agents/mod.rs:4`); quem importa `agents` é `http_bridge/agents.rs:8-11`, `presentation/http/state.rs:11-12`, `server.rs:10` e `register_owner.rs:1`.
- Graph: `graphify path "org" "agents"` sem caminho (não há código `org`). Hoje não há ciclo.
- O SDD exige que aposentar um agente encerre os assignments ativos dele **na mesma transação auditada**, exceto o da raiz (SDD L93). A aposentadoria mora em `agents` (`lifecycle.rs:53`, `state.rs:1080`) e persiste numa transação aberta pelo próprio adapter (`pg_registry.rs:26-34`; graph: `retire_agent → AgentRegistry → AgentDefinition ← persist_identity_and_enqueue_graph_projection`, 3 saltos).
- **Risco de ciclo:** se `agents` chamar `org` para encerrar assignments e `org::models` importar tipos de `agents`, forma-se `org → agents → org`.
- Regra da convenção MVC: sem ciclos entre módulos; dependência cruzada só por `application_contracts` ou API pública estreita ([modules-mvc-convention-sdd](../sdd/modules-mvc-convention-sdd.md) §2.4 e regra 6). `application_contracts.rs` hoje só tem `Signal` e `BotSignal` (`application_contracts.rs:1-30`).
- Precedente de hook: `MonitorAgentHook` é trait de `agents` consumida por `monitor` (graph: `supervisor.rs` L344, L866, L874, L1140).

| | A — `org` importa tipos puros de `agents`; retire orquestrado fora dos dois | B — tipos movidos para `application_contracts` | C — `org` tem ref opaca própria + porta `AgentDirectory` de `org` | D — `agents` publica evento de lifecycle; `org` consome |
|---|---|---|---|---|
| Direção | `org → agents` (só tipos); `agents ↛ org` | `org → contracts ← agents` | `org` sem dependência de compilação em `agents`; composição liga | `agents → outbox → org` |
| Dono da trait | não há trait nova para tipos; a orquestração do retire fica no composition root (`http_bridge`/`ApiState`), que chama os dois stores com a **mesma** `Transaction` | não há | `org` é dono de `AgentDirectory` (lookup de existência/lifecycle) | não há |
| Mesma transação do retire (SDD L93) | sim, desde que o adapter de `agents` ganhe método que aceita `&mut Transaction` (hoje abre a própria, `pg_registry.rs:34`) | igual a A (o problema da TX é o mesmo) | igual a A | **não**: assíncrono, contradiz SDD L93 |
| Risco de ciclo | baixo, se um guard proibir `modules::org` em `src/modules/agents` | nenhum | nenhum | nenhum de compilação; ciclo lógico de eventos possível |
| Custo | baixo em `org`; mudança pequena no adapter de `agents` (fora do escopo de `org`, precisa de SDD próprio ou item em `agents`) | médio/alto: mexe em `agents` e em todos os usuários de `AgentId` | médio: duplica validação de ID; lifecycle chega como valor (compatível com SDD §18 L398) | alto e não atende o SDD |
| Modos de falha | `agents` passa a importar `org` por atalho (detectado pelo guard) | refactor largo com risco de regressão em `agents` | divergência de regras de ID entre `org` e `agents` | agente aposentado mantém assignment durante a janela |

**Recomendação para o Julio:** **A**. `org::models` usa `AgentId`, `AgencyId` e `AgentLifecycleState` pelas reexportações públicas de `agents`; nenhuma dependência de `agents` para `org`; a aposentadoria que encerra assignments é orquestrada no composition root numa única transação PG que chama o store de `agents` e o store de `org`; `check-import-direction.sh` ganha a regra "`src/modules/agents` não importa `modules::org`" junto com a regra de rotas do S01 (plano S01 L210). A mudança no adapter de `agents` para aceitar transação externa é pré-requisito do teste T-20 e pertence ao dono de `agents`.

## 6. Modos de falha entre módulos

| ID | Modo de falha | Onde (evidência) | Detecção | Teste que precisa provar (§7) |
|---|---|---|---|---|
| M-1 | **Perda silenciosa de snapshot na outbox:** enqueue durante `processing` reabre a linha como `pending` com payload novo, mas o drain depois grava `done` só por `id` e apaga a reabertura | `graph_projection_outbox.rs:123-131` (reabre) e `:244-253` (`UPDATE … SET status='done' WHERE id = $1`) | drift PG × projeção (`aggregate_version` projetado < PG) | T-16 |
| M-2 | **Linha presa em `processing`:** crash entre o commit de `processing` e o `done`/`retry`, ou payload que não desserializa (o `?` sai da função depois do commit) | `graph_projection_outbox.rs:230-235`, `:240-241`; health não conta `processing` (`graph_projection_outbox_worker.rs:28-39`) | contagem de `processing` com idade (hoje invisível) | T-17 |
| M-3 | **Tarefa de fundo morta:** worker do outbox e varredura de TTL rodam em `tokio::spawn` sem supervisão; intervalo ausente desliga o worker sem erro | `graph_projection_outbox_worker.rs:58-95` (retorna em silêncio em `:59-61`); padrão em `server.rs:33-46` | métrica/heartbeat por tarefa; backlog e idade no `/healthz` (SDD outbox §6) | T-21 |
| M-4 | **Retry que responde sucesso:** replay de comando com a mesma idempotency key devolve `2xx` para um comando cuja primeira tentativa foi negada, ou reaplica efeito | SDD L224 exige a chave, mas não define replay | comparação do resultado gravado × resposta do replay | T-18 |
| M-5 | **Mapeamento de erro que colapsa erros distintos** ou vaza texto do driver | precedentes `error.rs:98-102`, `:159-185`, `:93-97`, `:211-215` | teste de tabela total; captura de corpo | T-01, T-02, T-03 |
| M-6 | **Bloqueio na cabeça da fila:** linha `retry` de menor `id` volta a ser escolhida a cada iteração do mesmo lote; uma linha envenenada consome o lote inteiro e não tem limite de tentativas | `graph_projection_outbox.rs:201-211` (`for _ in 0..cap` + `ORDER BY id`), `:256-270` | `attempt_count` alto na mesma linha; `succeeded = 0` com `processed = cap` | T-17 |
| M-7 | **Auth depois do parse:** corpo é lido antes de validar credencial/tenant; chamador sem credencial aprende sobre o esquema ou recebe `400/422` em vez de `401` | `routes/agents.rs:38-43` | teste com corpo inválido e sem credencial | T-06 |
| M-8 | **Fallback entre bearers:** token admin aceito em rota `Org` ou bearer P1 aceito em `Protected` | SDD L240; seam atual aceita qualquer coisa sem token (`admin_auth.rs:91-94`) | teste de matriz de credenciais por classe | T-05 |
| M-9 | **Ciclo de dependência `org ↔ agents`** criado para o retire | SDD L93; §5 | guard de import | T-19, T-20 |
| M-10 | **Enqueue fora da transação:** mutação comita e o enqueue falha só com `warn` | padrão `graph_projection_outbox.rs:276-294` | teste de rollback injetado | T-12 |
| M-11 | **Projeção atrasada usada como verdade:** Neo4j fora ou atrasado e a leitura devolve vazio em vez de `degraded` | SDD L99; plano S07 L259 | teste com Neo4j fora | T-22 |
| M-12 | **Precedência errada entre camadas:** `403` antes de `409 root_position_immutable`, `409` antes de `403 approver_not_independent`, ou `401` antes de `421 host_not_allowed` | SDD L297; W0-01 L155 | testes de precedência | T-08, T-09, T-23 |
| M-13 | **Resposta de erro fora do contrato:** rejeição padrão do axum (texto puro) sai por rota `org` | SDD L269 | teste que confere `Content-Type: application/json` e `code` em toda rejeição de corpo | T-07 |

## 7. Testes de contrato/integração que devem existir

O Builder Rust implementa; aqui só a especificação. "Existe" cita o teste de hoje que serve de padrão; nenhum teste de `org` existe.

| ID | Nome proposto | Seam | O que prova | Existe hoje |
|---|---|---|---|---|
| T-01 | `org_error_mapping_is_total_and_stable` | S-5 | cada variante de `OrgDomainError` → (status, `code`) da tabela §3a; tabela congelada em snapshot | não; padrão: `store_unavailable_maps_to_service_unavailable` (`error.rs:273-280`) |
| T-02 | `org_error_body_never_leaks_driver_or_principal` | S-5 | falha de PG → `503 org_store_unavailable` sem texto do driver; nenhum corpo contém `issuer`, `subject`, token | não |
| T-03 | `org_errors_always_carry_code` | S-5 | toda resposta de erro de rota `org` tem `code` não nulo | não |
| T-04 | `org_routes_inventory_all_class_org` | S-2 | toda rota `/api/v1/org*` da tabela tem classe `Org` e política; rota fora da tabela reprova | não; depende do W0-01 F3 (W0-01 L186) |
| T-05 | `org_route_rejects_admin_token_and_p1_on_protected` | S-2, S-3 | token admin em `Org` → `401`; bearer P1 em `Protected` → `401` | não; padrão: `agents_register_requires_admin_bearer_when_enabled` (`http_integration_tests.rs:48`) |
| T-06 | `org_auth_and_tenant_run_before_body_parse` | S-2, S-4 | sem credencial + corpo malformado ou com campo extra → `401`; bearer de outra org + campo extra → `404 org_not_found` | não |
| T-07 | `org_unknown_field_returns_400_json_body` | S-4 | campo extra no topo e em objeto aninhado → `400`, `Content-Type: application/json`, `code = "unknown_field"`; nunca `422` texto | não |
| T-08 | `org_root_immutable_precedes_authorization` | S-5 | operação na raiz por actor sem escopo → `409 root_position_immutable`, não `403` | não |
| T-09 | `org_independence_precedes_chain_mismatch` | S-5 | ocupante anterior de `parent(T)` aprovando → `403 approver_not_independent`, não `409` (fatia S10) | não |
| T-10 | `org_feature_enabled_without_p1_fails_startup` | S-7 | exit ≠ 0 e socket não aberto | não; padrão de CLI do W0-01 F1 (W0-01 L184) |
| T-11 | `org_feature_disabled_routes_answer_503_owner_auth_required` | S-2, S-7 | comportamento com feature desligada (depende da pergunta 4 de §9) | não |
| T-12 | `org_mutation_and_outbox_share_transaction` | S-9 | falha injetada depois do enqueue → nem mutação, nem audit, nem outbox persistem | não; padrão: `pg_agent_identity_and_graph_projection_same_transaction` (`pg_registry.rs:378`, via graph) |
| T-13 | `org_outbox_payload_has_no_raw_principal` | S-9 | payload só com `principal_ref`; sem `issuer`/`subject`/nome/email | não |
| T-14 | `org_outbox_idempotency_keys_are_stable` | S-9 | mesma entidade → mesma chave; entidades de orgs diferentes → chaves diferentes | não; padrão: `outbox_message_idempotency_keys_are_stable` (`graph_projection_outbox.rs:461`, via graph) |
| T-15 | `org_projection_rejects_out_of_order_version` | S-10 | payload com `aggregate_version` menor que o projetado não altera o grafo | não (plano S08 L265) |
| T-16 | `outbox_enqueue_during_processing_is_not_lost` | S-9, S-10 | enqueue com a linha em `processing` → depois do drain, o payload novo continua `pending` ou foi aplicado | não (M-1; `core::database`) |
| T-17 | `outbox_poison_row_does_not_block_batch_or_stick_processing` | S-10 | payload inválido e falha de porta não deixam linha em `processing` nem consomem o lote inteiro | parcial: `pg_graph_projection_outbox_drain_marks_retry_on_port_failure` (`graph_projection_outbox.rs:529`, via graph) cobre só o `retry` |
| T-18 | `org_command_replay_returns_original_outcome` | S-1, S-9 | mesma chave + mesmo corpo → mesma resposta, inclusive negação; nenhum efeito duplicado; mesma chave em outra org → comando independente | não |
| T-19 | `import_direction_guards_org` | S-6, S-11 | fixture com `modules::org::` em rota e com `modules::org` em `src/modules/agents` → `FAIL`; árvore real passa (hoje `SRC` é fixo, `check-import-direction.sh:4`) | não |
| T-20 | `agent_retire_ends_org_assignments_in_same_transaction_except_root` | S-12 | retire encerra assignments não raiz na mesma TX; falha injetada → nada persiste; assignment da raiz fica | não |
| T-21 | `org_ttl_decision_independent_of_sweeper` | S-7 | com a varredura parada, leitura de autorização de pendente vencido já nega; tarefa morta aparece na observabilidade | não |
| T-22 | `org_traversal_with_neo4j_down_returns_degraded` | S-10 | leitura de traversal com Neo4j fora → `degraded`, nunca lista vazia | não (plano S07 L259) |
| T-23 | `org_host_and_rate_limit_precede_org_auth` | S-2 | `Host` fora da allowlist em rota `Org` → `421` sem chamar o verificador P1 | não; depende da resposta do Arquiteto em §3b |
| T-24 | `org_unauthorized_message_is_generic` | S-5 | `401` de rota `org` não cita "admin bearer" | não (`error.rs:130-136`) |

## 8. Acoplamento não declarado, ciclos e seams usados diferente do doc

1. **Outbox SDD desatualizado sobre transação:** [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md) §2 diz que o enqueue "não está na mesma transação que o domínio hoje"; o código de agents já grava identidade, audit e outbox numa TX (`pg_registry.rs:26-34`, graph `persist_identity_and_enqueue_graph_projection → enqueue_graph_projection_outbox_tx`). O §8 do mesmo SDD já registra isso; o §2 contradiz o §8.
2. **Outbox é snapshot, não log de eventos:** o SDD `org` fala em "eventos" e em "`MERGE` idempotente com aggregate/version" (SDD L98); o seam central coalesce por chave com `ON CONFLICT DO UPDATE` (`graph_projection_outbox.rs:123-131`). É compatível só se o payload carregar versão; sem versão, ordem e perda (M-1) ficam invisíveis.
3. **Payload de `org` vai morar em `core`:** `core` não pode importar `modules` (`check-import-direction.sh:20`), então o DTO de projeção de `org` fica em `core::database::graph_projection`, como `AgentHierarchyProjection` (`graph_projection.rs:22`). Cada domínio novo aumenta o enum central (`graph_projection_outbox.rs:27-35`). O plano cita "new `GraphProjectionPort` method or generic event" (S08 L264) sem decidir.
4. **Auth por handler, não por layer:** o SDD `org` depende da layer deny-by-default do W0-01 (SDD L240); o código continua com chamadas manuais (`routes/mod.rs:25-93`, `state.rs:800-802`) e fail-open (`admin_auth.rs:91-94`). S07/S13 não podem começar antes do W0-01 com a classe `Org` (plano L119, L125).
5. **Parse antes de auth nos handlers atuais** (`routes/agents.rs:38-43`). Se `org` copiar o padrão, viola a precedência do SDD (L297).
6. **Health não vê `processing` e não separa por domínio:** `fetch_graph_projection_outbox_stats` conta só `pending` e `retry` na tabela inteira (`graph_projection_outbox_worker.rs:28-39`); o lag da outbox de `org` exigido no SDD §17 (L392) não é observável com o seam atual.
7. **`ApiState` como concentrador:** `state.rs` tem 2781 linhas e já importa `modules::agents` direto (`state.rs:11-12`). A regra do SDD é só para rotas (SDD L265); se `org` entrar pelo `ApiState`, o arquivo cresce. Observação, sem proposta de refactor aqui.
8. **Retire sem ponto de extensão transacional:** o adapter de `agents` abre a própria transação (`pg_registry.rs:34`); o requisito de mesma transação do SDD (L93) não tem seam no código (§5).
9. **Guard de import sem `SRC` alternativo:** `check-import-direction.sh:4` fixa `SRC="$ROOT/src"`; a prova por fixture exigida no S01 (plano L210) depende de mudar o script.
10. **Versão do threat model:** o SDD declara normativa a versão `276a98f6` (SDD L353); o arquivo no checkout tem sha256 `50140e33…`. O próprio threat model cita o SDD `f11d9966` e o plano `fc85fc93` (threat model L28), não as versões lidas aqui (`d42291d7`, `a7ca91fc`). Divergência para o Arquiteto e o Segurança; este documento não interpreta os achados.
11. **SDD L217 cita `presentation/http/server.rs:30-49`** para o padrão de poll; o bloco vai até `server.rs:53` no código atual. Diferença de linha, sem mudança de sentido.

## 9. Perguntas abertas

1. **D-OUTBOX:** aprovar a opção A (§4) com chave por agregado, `aggregate_version` e guarda de versão, e com a correção de M-1, M-2 e M-6 antes do primeiro enqueue de `org`? **Recomendação:** sim; B só quando o SDD de P3-b pedir ack de `revocation_complete`.
2. **D-SEAM-AGENTS:** aprovar a opção A (§5), com retire orquestrado no composition root numa única transação e guard `agents ↛ org`? **Recomendação:** sim; o ajuste no adapter de `agents` para aceitar transação externa vai para o dono de `agents`.
3. **Erros de corpo além de `unknown_field`** (JSON malformado, tipo errado, campo ausente, `Content-Type` ausente): qual `status`/`code`? **Recomendação:** `400` com `ApiErrorBody` em JSON e um `code` estável próprio (ex. `invalid_body`), incluído pelo Arquiteto no SDD §10; nunca a rejeição em texto puro do axum.
4. **Feature `org` desligada:** as rotas não existem (o W0-01 responde `401` a path desconhecido) ou existem e respondem `503 owner_auth_required`? E onde ficam `421`/`429` do W0-01 na precedência? **Recomendação:** rotas sempre na tabela com classe `Org`; feature desligada → `503 owner_auth_required` antes de olhar credencial; `421` e `429` antes da precedência (1) do SDD.
5. **Replay de idempotency key:** mesma chave com corpo diferente, e replay de comando negado? **Recomendação:** guardar resultado e hash do corpo por `(org_id, key)`; mesmo hash → mesma resposta, inclusive a negação original; hash diferente → `409` com `code` próprio definido pelo Arquiteto no SDD §10.

## 10. Status

Pendente de revisão do Critic.
