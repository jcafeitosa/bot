---
title: Plano de implementação do sistema organizacional org
description: Entregas, gates, testes e integrações PostgreSQL/Neo4j para org e governança de agentes
tags:
  - planning
  - org
  - agents
  - implementation
  - backend
status: draft
---
# Org Complete System Implementation Plan

> **For agentic workers:** Execute this plan task-by-task only after explicit user approval. Each implementation task requires a Builder and a separate independent Critic. Steps use checkbox tracking.

**Goal:** Implement the complete `org` organizational domain for units, titles, positions, assignments, hierarchical policies and autonomous-agent governance, integrated with PostgreSQL as the source of truth and Neo4j as a derived graph projection.

**Architecture:** `org` owns organizational structure, assignments, policy versions, approvals, delegation and audit. PostgreSQL schema `org` is the transactional source of truth through the central `core::database` runner. Neo4j receives an idempotent `graph_domain=org` projection through core-owned seams/outbox; it is never used as an authorization or write source. Owner authentication/bootstrap and the runtime/tool gateway are separate prerequisite deliveries; mutating endpoints and task/action execution remain fail-closed until their gates pass.

**Tech Stack:** Backend Rust, existing MVC module conventions, SQLx/PostgreSQL 18+ central database, Neo4j through `core::database` graph projection/query seams, existing HTTP/OpenAPI and test gates, OpenKnowledge and Graphify.

**Spec:** [[sdd/org-module-sdd]] — SDD draft awaiting final user approval.

## Global Constraints

- `OrgId` is 1:1 with legacy `AgencyId`; the legacy ID mapping remains `NOT NULL UNIQUE` through cutover.
- Owner → CEO → Level C → Level B → Level A → Specialist/Worker. Level C is new relative to current enums and requires an explicit serialized-data migration.
- One active occupant per position; one principal may occupy multiple positions.
- JobTitle published versions are immutable.
- Owner and CEO independently approve the same global-policy hash/version; subordinate approval is by the direct active superior and policies are subsets only.
- A position assignment never grants unapproved capabilities; pending or revoked grants fail closed.
- `task.start` is separate from tool/action capabilities; every action revalidates current policy ancestry/epoch; unknown/stale/unavailable state denies and cancels.
- Finance, payment, production and credential/permission mutation remain denied until their own SDD, threat model and human approval gate.
- PostgreSQL `org` schema is SoT; Neo4j `graph_domain=org` is derived, idempotent, rebuildable and read/query-only.
- All DB/bootstrap/projection code uses `core`; no module-local pool or `neo4rs` import.
- Mutations, approval and cutover stay disabled until the prerequisite auth, owner bootstrap, DB-role and isolated-test gates pass.
- Every task includes docs/OpenKnowledge updates proportional to its interface changes, Graphify update after code changes and independent Critic review before acceptance.

## Review Focus

1. Concurrent hierarchy edits that could jointly create a cycle must be serialized/denied; test concurrent write skew at the PostgreSQL transaction boundary.
2. A stale or unavailable policy/revocation epoch must deny actions immediately; test cache invalidation, in-flight task revalidation and outbox lag.
3. Wrong-organization or spoofed actor/occupant IDs must never cross tenant boundaries; test domain, SQL constraints and HTTP auth.
4. Cross-database grants must remain non-effective until confirmed in `agents`; test outbox retries, order, duplicates, partial failures and compensation.
5. Neo4j lag/down/rebuild must never authorize or lose PostgreSQL state; test PG SoT, projection idempotency, namespace-scoped rebuild and degraded reads.

---

## File map

| Area | Planned files | Responsibility |
|---|---|---|
| Domain | `backend/src/modules/org/models/{ids,organization,unit,job_title,position,assignment,policy,delegation,error}.rs` | Typed entities, states, invariants and errors without SQL/HTTP. |
| Use cases | `backend/src/modules/org/controllers/{organization,structure,assignment,policy,delegation,query}.rs` | Validation, transactions and orchestration. |
| PostgreSQL | `backend/src/modules/org/adapters/postgres.rs`; SQL in `backend/src/core/database/migrations/00xx_org_*.sql` | Repository operations and migrations registered by central runner. |
| Neo4j | Existing `core::database::GraphProjectionPort`, add `backend/src/core/database/neo4j_org_projection.rs`; projection records/outbox adapters within `org` | Idempotent graph MERGE and projection health/rebuild seams. |
| HTTP | `backend/src/presentation/http/routes/org.rs`, module bridge/API state/OpenAPI | Authenticated DTOs/queries/commands; no domain logic in handlers. |
| Auth | Existing `core` provider/auth seam selected by prerequisite SDD | Verify issuer/audience/expiry/signature and derive actor/owner binding. |
| Runtime | Separate runtime/task/tool-gateway SDD and modules | Proactive tasks, per-action policy enforcement, tool execution, cancellation/revalidation. |
| Tests | `backend/src/modules/org/tests.rs`, adapter integration tests, HTTP integration tests, PG+Neo4j integration tests | Domain, SQL, auth, concurrency, outbox, projection and real request paths. |
| Docs | `backend/docs/sdd/org-module-sdd.md`, index/catalog/strategy and runtime SDDs via OpenKnowledge | Contracts, decisions, operations, status. |

## Prerequisite gates (each gate blocks only the tasks listed)

The gate IDs P1/P2/P3 are identifiers, not priority rankings. **Pure domain work and tests may proceed before P1/P2/P3**; authentication, persistent schema, API writes, grants, projection, runtime effects and cutover are individually gated below.

### Gate P1 — Owner authentication and bootstrap

**Files:** new/selected auth-provider adapter under `backend/src/core/auth/` only after provider choice; `ApiState` composition and auth tests.

- Define provider, issuer/audience, signature/key rotation, expiry/revocation and mapping to `HumanPrincipal(issuer,subject)`.
- Bootstrap one owner principal per org through controlled operator setup or signed enrollment; public API cannot self-appoint owner.
- Test valid, invalid, expired, wrong-issuer, wrong-audience, replayed and organization-mismatched credentials at a real HTTP entrypoint.
- `BOT_HTTP_ADMIN_TOKEN` never substitutes for owner identity.

**Blocks:** Task 5 policy approvals/grants; Task 7 reads if private data; Task 8 all mutation routes; Task 9 cutover; runtime SDD. It does not block Task 1 pure models/tests.

**Acceptance:** actor is server-derived; every mutation fails closed without verified principal/owner binding; owner transfer requires old+new principal approvals and audit.

### Gate P2 — Database/migration authority and isolated tests

**Files:** `core::database` config/bundle, PG integration harness and CI workflow only as needed.

- Confirm central PG database/schema ownership and least-privilege runtime DML vs migration role; core runner is sole migration executor.
- Provide a disposable PostgreSQL database (with required version/extensions) for fresh/upgrade/rollback tests; shared `trading_bot` is never the test target.
- Add URL/database-name validation and fail before migrations when target is not the explicit isolated DB in integration mode.
- Add migration ownership/rehearsal and cleanup proof.

**Blocks:** Task 2 schema/repository, Task 4 transactional assignment tests, Task 5 policy/grant integration tests, Task 6 projection/outbox integration, Task 8 end-to-end writes and Task 9 migration rehearsal. Task 1 pure domain tests and non-mutating controller design can proceed without it.

**Acceptance:** fresh/upgrade/rollback run isolated; no connection pool or second migration runner inside `org`.

### Gate P3 — SDD closure for capability catalog, delegation and runtime gateway

- Fix typed capability/action/resource/limit schema and approval protocol owner+CEO plus direct-superior approvals.
- Define task/action/tool categories, per-action authorization, cancellation/revalidation, idempotency and high-impact deny rules.
- Define outbox owner, per-agent ordering, idempotency key, retry/backoff, DLQ/manual reconciliation and registered consumer acknowledgements for `revocation_complete`.
- A committed revoke increments the authoritative PG epoch and denies all actions immediately; missing/stale consumer ack keeps status incomplete but never leaves a grace period. Do not enable grants/runtime if the authorization gateway cannot read the authoritative epoch or fail closed on outage.
- Produce separate reviewed SDD for task scheduler/runtime/tool gateway. `org` does not implement external effects.

**Blocks:** Task 5 grant approval/application, all autonomous task/tool effects in Task 10 and any high-impact domain. Tasks 1–4 may implement structure/assignment without capability grants; Task 7 can expose authenticated read-only views after P1.

**Acceptance:** independent Critics approve auth, policy and runtime contracts; no grant/task writes before gates pass.

## Task 1: Domain type and invariant foundation

**Files:** create `backend/src/modules/org/mod.rs`, `models/*.rs`, `tests.rs`; modify module registry/import-direction as needed.

**Interfaces:**
- `OrgId`, `OrgUnitId`, `JobTitleId`, `JobTitleVersion`, `PositionId`, `AssignmentId` newtypes.
- `HumanPrincipal { issuer: IssuerId, subject: SubjectId }`; agent occupant references existing `AgentId`.
- `OccupantRef::{Human(HumanPrincipal), Agent(AgentId)}`.
- State enums from SDD: org unit/title version/position/assignment/policy/delegation.
- Domain validators return `OrgDomainError`, no SQL/HTTP.

**Steps:**
- [ ] Write tests for `OrgId↔AgencyId`, lifecycle transitions, title immutable publication, occupied-position freeze/retire rejection, one active assignment per position, occupant multi-position acceptance, paused/retired agent semantics.
- [ ] Run `cargo test --locked --bin bot modules::org::tests::domain` and observe red.
- [ ] Implement minimal typed model and pure validators.
- [ ] Re-run focused domain tests and formatting.
- [ ] Critic reviews Task 1 before accepting.

## Task 2: Central PostgreSQL schema and repositories

**Files:** add ordered `00xx_org_*.sql` migrations; create `modules/org/adapters/postgres.rs`; modify only central migration registration if the current runner requires it; integration harness dedicated DB.

**Interfaces:**
- Tables: `organization_agency_map`, `org_units`, `job_titles`, `job_title_versions`, `positions`, `assignments`, `capability_policy_versions`, `capability_policy_approvals`, `scoped_delegations`, `org_audit_events`, `org_outbox`, projection checkpoint.
- Unique mapping `legacy_agency_id NOT NULL UNIQUE`; one owner active per org; composite tenant-scoped FKs; partial unique active assignment per position; immutable published version trigger/guard; temporal/state checks.
- `OrgRepository` methods accept a transaction/session from `core`; no own Pool/bootstrap.

**Steps:**
- [ ] Write isolated PostgreSQL tests for migration shape and constraints (cross-org FK, duplicate active assignment, immutable title version, owner uniqueness).
- [ ] Run focused PG test; observe red in disposable DB only.
- [ ] Add minimal schema and repository read/write primitives.
- [ ] Test fresh migration, upgrade from current schema, constraints and rollback; capture DB state before/after cleanup.
- [ ] Critic reviews SQL, tenant scoping and least privilege.

## Task 3: Org units, job titles and positions

**Files:** modify `models/{unit,job_title,position}.rs`, `controllers/structure.rs`, `adapters/postgres.rs`; domain/PG tests.

**Interfaces:** `create_unit`, `create_job_title_draft`, `publish_job_title_version`, `create_position`, `set_position_supervisor`, `freeze_position`, `retire_position`, query/read models.

**Steps:**
- [ ] Add red tests for same-org parent/supervisor and cross-org rejection, unit/position cycle rejection, title version semantics and occupied-position state constraints.
- [ ] Implement controllers with transaction-scoped org lock and `SERIALIZABLE` hierarchy writes; bounded retry only on serialization failures without external effects.
- [ ] Run domain+PG tests including two concurrent edits that together would form a cycle; assert one denies/retries and committed graph remains acyclic.
- [ ] Critic reviews graph concurrency and transition semantics.

## Task 4: Human/agent assignments and audit

**Files:** add/modify `models/assignment.rs`, `controllers/assignment.rs`, PG adapter/migrations and tests.

**Interfaces:** `assign_occupant`, `end_assignment`, `transfer_occupant`, `list_assignments`, `get_position_history`; every command carries authenticated actor, idempotency key if retryable, expected version and reason code.

**Steps:**
- [ ] Add red tests for human/agent assignment, paused-agent retention, retired-agent assignment closure, one active occupant/position, multiple positions/occupant, transfer rollback and append-only audit.
- [ ] Implement assignment/transfer/retirement in a single PG transaction with audit+outbox.
- [ ] Run real PG isolated tests including injected constraint failure proving whole transfer rolls back.
- [ ] Critic reviews transaction/error/audit behavior.

## Task 5: Owner/CEO approval, policies and delegation

**Files:** `models/policy.rs`, `models/delegation.rs`, `controllers/policy.rs`, `controllers/delegation.rs`, PG adapter/schema, auth integration.

**Interfaces:** `draft_policy`, `approve_global_policy(actor, hash, version)`, `approve_subordinate_policy(superior_actor, ...)`, `publish_policy`, `revoke_policy`, `create_interim_delegation`, `revoke_delegation`; approval derives actor from verified auth, never request body.

**Steps:**
- [ ] Add red tests for one signature only, mismatched hash/version, vacant owner/CEO, wrong-level approver, policy expansion, descendant subset validation, self-approval, interim delegation trying to approve/subdelegate, and revocation epoch.
- [ ] Implement versioned typed policy with two independent owner+CEO approvals for global; direct-superior approvals for subordinate versions; monotone-subset validation.
- [ ] Implement bounded interim delegation (execution only; expiry; no approval/subdelegation).
- [ ] Run auth+PG tests at real request entrypoint; assert all missing/stale states deny.
- [ ] Critic reviews escalation resistance and policy conflicts.

## Task 6: Neo4j `graph_domain=org` projection

**Files:** extend `core::database::GraphProjectionPort` and `AppDatabases` composition if necessary; add `core/database/neo4j_org_projection.rs`; `org` projection/outbox adapter; integration tests.

**Interfaces:** projection events include stable PG aggregate ID, org ID, aggregate version, event ID and redacted projection fields. Graph node/edges: Organization, OrgUnit, Position, JobTitleVersion, Agent/HumanPrincipal pseudonymous reference, `CONTAINS`, `REPORTS_TO`, `HAS_TITLE`, `OCCUPIES`; omit PII and approval evidence.

**Steps:**
- [ ] Add fake-projector tests for idempotent event delivery, ordering/version rejection and no secrets/PII in projection DTO.
- [ ] Add Neo4j integration test (enabled target only) for MERGE and namespace isolation; Neo4j outage leaves PG commit and outbox pending.
- [ ] Add rebuild test that drops/rebuilds only `graph_domain=org` from PG snapshot and leaves `agents`, `bots`, `code` namespaces intact.
- [ ] Add readiness/lag/error reporting and degraded traversal response; auth continues to query PG/current revocation epoch, never Neo4j.
- [ ] Critic reviews projection, replay and rebuild safety.

## Task 7: Read-only organization API

**Files:** `presentation/http/routes/org.rs`, DTO module, router/OpenAPI, `ApiState` query composition, HTTP tests.

**Interfaces:** read routes for org, units/tree, job titles/versions, positions/current assignments/history, subordinate traversal. Every query requires verified actor and organization scoping unless a separate public-read policy is approved; API gateway/core auth adapter owns actor resolution.

**Steps:**
- [ ] Add route-level tests for tenant isolation, unauthenticated read denial, DTO redaction, pagination/version consistency, Neo4j down with PG fallback only for required transactional detail.
- [ ] Implement handlers that call query controllers; keep SQL/domain rules outside handlers.
- [ ] Run real HTTP integration tests and OpenAPI checks.
- [ ] Critic reviews route/auth/read privacy.

## Task 8: Mutating API and owner bootstrap

**Files:** auth/bootstrap provider seam, `routes/org.rs`, `ApiState`, bootstrap command or operator-only setup path chosen in P1, HTTP tests.

**Interfaces:** commands for unit/title/position/assignment/policy/delegation; current actor comes from signed auth middleware. No public self-assign-owner endpoint.

**Steps:**
- [ ] Do not begin until P1/P2/P3 approved and implemented.
- [ ] Add red HTTP tests for no auth, forged actor in JSON, owner/CEO single approval, wrong organization, optimistic version conflicts, unsafe high-impact capability.
- [ ] Implement bootstrap and routes fail-closed by default, then enable only with explicit runtime config.
- [ ] Run end-to-end request against isolated PostgreSQL and test IdP fixture; verify audit and outbox records.
- [ ] Critic reviews authz, bootstrap, redaction and error handling.

## Task 9: Hierarchy cutover from `agents.supervisor`

**Files:** data-migration command/SQL, `agents` model/serialization, `org` import/cutover controller, rollback artifact and reports.

**Steps:**
- [ ] Inventory every serialized role/supervisor value in Rust, PostgreSQL, fixtures, JSON APIs, Neo4j labels/properties and docs; explicitly migrate legacy LevelB/LevelA to the approved Owner/CEO/LevelC/B/A/Specialist/Worker mapping. Unknown, duplicate or unmappable value blocks; no aliases silently map Level C to Level B.
- [ ] Require a user-approved final SDD plus migration-authority approval before cutover code or data mutation; current `draft` status blocks this task.
- [ ] In a disposable clone, backfill OrgId map, positions and supervisors; inventory serialized role compatibility; report cycles/orphans/multiple roots, assignment/policy mappings and counts/hash differences; no automatic winner selection.
- [ ] Reconcile each difference to an owner-approved resolution; produce signed/hash-identified preflight report proving zero unresolved hierarchy/tenant/assignment mismatches.
- [ ] Rehearse rollback from a snapshot: restore legacy source reads, remove/mark only `graph_domain=org` projection, retain audit history, and revoke only grants whose source is the reverted org policy while preserving grants from other sources.
- [ ] Verify rollback restores the old hierarchy before accepting any mutation not representable by it; assert no dual writable hierarchy and replay audit/outbox consistently.
- [ ] Only after isolated rehearsal passes, the authenticated owner explicitly approves that exact migration/preflight hash and the authorized operations window; switch one canonical read source and keep legacy tree as immutable read-only rollback snapshot.
- [ ] Verify no hierarchy mismatch, auth remains PG/policy-based, projection rebuilds; Critic reviews evidence before production cutover.

## Task 10: Runtime and tools integration (separate SDD required)

**Files:** new runtime/task/tool modules and separate SDD/Threat model, not hidden inside `org`.

**Acceptance from `org` side:** runtime proves `task.start` independently of tool invocation, performs server-side ancestry intersection before each effect, cancels/revalidates in-flight tasks on policy/assignment/lifecycle changes, denies stale/unavailable policy/revocation, logs decision correlation and idempotency. High-impact finance/production remains disabled until a specific domain SDD and human approval gate.

**Implementation begins only after a reviewed runtime SDD defines task persistence, scheduler, action catalog, resource scoping, budgets, human escalation, retry, cancellation and recovery.**

## Task 11: Operations, docs and final verification

**Files:** module catalog/status, integrations, Neo4j strategy, runbook, test matrix, OpenKnowledge index; Graphify generated output.

**Steps:**
- [ ] Add runbook for backup/restore, projection lag/rebuild, owner bootstrap/recovery, approval/revoke, outbox replay, cutover/rollback.
- [ ] Define SLI/SLO/readiness and alerts for PG SoT, Neo4j lag, outbox oldest age, revocation propagation and runtime denial/error rates before launch.
- [ ] Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `./scripts/verify-backend-gates.sh`, isolated PG suite, Neo4j integration suite, import-direction, OpenAPI checks.
- [ ] Capture full logs in `{SCRATCH}`; update Graphify and run OpenKnowledge links/audit (and markdownlint if enabled).
- [ ] Independent Critic reviews entire branch; no deploy/cutover without explicit authorization.

## Per-task review record (required before accepting any task)

Record each delivery in this shape before accepting the task:

```markdown
### DELIVERY ORG-Txx
- Builder: <independent implementation instance>
- Summary/artifacts: <files/interfaces, exact scope>
- Tests/evidence: <commands, captured output, real path exercised>
- Critic: <separate instance>
- Findings and fixes: <ranked, concrete, with evidence>
- Risks/follow-ups: <remaining blockers and dependencies>
- Verdict: PENDING | APPROVED | APPROVED WITH FOLLOW-UP | REJECTED — by <Critic>
```

Builder cannot be its own Critic. Critical/high findings block dependent tasks until fixed or explicitly accepted by the authorized human. Independent review capacity must be available before that task starts; no role simulation or self-approval.

## Dependency graph and stop conditions

**Gates P1/P2/P3 são gates, não prioridades globais:**

- Task 1 (pure domain types/invariants) pode começar sem P1/P2/P3.
- Task 2 (schema/repository) exige P2; seu desenho/teste SQL isolado pode ser revisado antes de provisionar o ambiente, mas migration/write não roda fora dele.
- Task 3 (units/titles/positions) depende de Task 1; unit tests podem correr antes de P2, persistência/ciclo concorrente PG exige Task 2 + P2.
- Task 4 (assignments/audit) depende de Tasks 1–3 e P2; comandos HTTP mutantes aguardam P1.
- Task 5 (policy/delegation) depende de Tasks 1–4, P1 e P3. Grants efetivos aguardam policy integration tests e outbox aprovado.
- Task 6 (Neo4j projection) depende de Task 2, outbox contract em P3 e teste PG; Neo4j é derivado, nunca bloqueia commit SoT.
- Task 7 read API depende de Tasks 1–3; P1 é obrigatório se dados forem privados. Task 8 writes depende de P1/P2/P3 e Tasks 1–6.
- Task 9 cutover depende de Tasks 1–8, migration authority, owner approval, isolated rehearsal e rollback aceito.
- Task 10 depende de auth, Task 5, Task 6, runtime SDD independente aprovado e gateway capaz de consultar revocation epoch fail-closed.
- Task 11 acompanha cada entrega; release/readiness final só fecha depois dos gates, críticos e integrações correspondentes.

Stop conditions: qualquer incerteza de actor/tenant, serializer mapping, outbox/revocation, PG isolation ou Graph projection deixa a dependência downstream parada; trabalho puro/testado que não escreve autoridade pode continuar.

A missing owner auth provider, isolated PostgreSQL/Neo4j test environment, unanswered serialized role migration, missing owner+CEO approvals, stale projection contract or failure of any tenant/policy/revocation test blocks the dependent task; do not substitute mocks for required real DB/HTTP behavior and do not continue to grant/cutover.

## Open design inputs (must be resolved in prerequisite SDDs)

- Authentication provider, issuer/audience/key rotation/revocation and secure owner bootstrap/recovery.
- Exact policy action/tool/resource schema, quotas/budgets and rule for finance/production approvals.
- Concrete outbox storage/worker, idempotency, ordering, retry/DLQ and `revocation_complete` consumer acknowledgement.
- PostgreSQL migration and runtime roles; isolated PG+Neo4j test environment, versions/extensions and cleanup procedure.
- Neo4j required-vs-degraded readiness per deployment; traversal API's stale projection response contract.
- Retention/PII deletion/subject pseudonymization; audit access and legal hold.
- SLI/SLO, backup/restore, RPO/RTO and cutover maintenance window.
- Which policy changes only restrict and can receive single superior approval versus any global envelope change that requires owner+CEO coapproval; prevent lateral policy changes from evading dual approval.

## Verification / documentation lock

Use separate OpenKnowledge calls to review the SDD and plan (`exec`/`search` read), then check their backlinks and audit scope. Never use a combined `cat` call to stand in for separate review records: SDD review confirms domain/security decisions; plan review confirms task dependencies/evidence only. Before each implementation task, load the approved SDD revision and its specific task section; implementation must stop if that revision/hash is not the owner-approved one.

## Self-review

- **Coverage:** SDD sections 1–18 are covered by P1–P3 and Tasks 1–11; runtime/tools have their own required SDD before work. Auth, migration, PG, Neo4j, role mapping, policies, revocation, audit, operations and rollback are represented.
- **No placeholders:** all external decisions are named in prerequisite gates/open inputs; implementation tasks do not use “TBD” or silently assume providers/DB credentials.
- **Type consistency:** identifiers and policy/assignment states are defined in Task 1; later tasks consume those names; `AgentId` is referenced from agents, not duplicated.
- **Review Focus:** all five risks map to concrete Task 3, 5/10, 4/7/8, 5/6, and 6 tests respectively.

## Approval

This plan is **draft**. It is not implementation authorization. Owner must approve the SDD version and this plan before code; then user must choose execution method if not already established.
