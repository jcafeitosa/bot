---
title: Plano de implementação do sistema organizacional org
description: Fatias verticais, dependências P1/P2/P3, seams públicos e critérios de aceite para org e governança de agentes
tags:
  - planning
  - org
  - agents
  - implementation
  - backend
status: draft
---
# Org Complete System Implementation Plan

> **For agentic workers:** execute only after the owner explicitly approves the SDD revision and this plan. Every slice has a Builder and a separate independent Critic; no gate in this document is marked passed.

**Goal:** implement the `org` domain (units, titles, positions, assignments, hierarchical policies, autonomous-agent governance) with PostgreSQL schema `org` as source of truth and Neo4j as a derived projection.

**Spec:** [[sdd/org-module-sdd]] (draft, revision 2026-09-27, Critic cycles 1–4 and the security verdict on SDD `905ed5e7` applied). The [threat model do `org`](../security/org-module-threat-model.md) (read at sha256 `c1d21216`) is **normative** for `org`: every SEC-ORG criterion is acceptance of the slice mapped in the traceability table below. The errata [[planning/org-plan-authority-errata]] is **incorporated** into this plan and the SDD; the errata file is kept for traceability only.

**Architecture:** `org` owns structure, assignments, policy versions, approvals, delegation, grants, `revocation_epoch` and audit in PG schema `org`, migrated by the central `core::database` runner (`backend/src/core/database/migrations/`; use the next free sequence number at the time of S04, since `0011` is already `0011_monitor_supervisor_snapshot.sql`). Neo4j receives an idempotent `graph_domain=org` structural projection through the core outbox/projection seams (SDD D-OUTBOX); it never authorizes. Owner authentication (P1), DB authority (P2) and the runtime/tool gateway including agent-principal authentication (P3) are separate prerequisites.

**Tech stack:** Rust backend, MVC module convention, SQLx/PostgreSQL (code minimum PG 18; CI currently `timescaledb-ha:pg16`, divergence tracked by task T-CI-02), Neo4j via `core::database`, HTTP/OpenAPI, OpenKnowledge, Graphify.

## Global constraints

- `OrgId` is 1:1 with legacy `AgencyId`; `legacy_agency_id NOT NULL UNIQUE` survives cutover.
- Hierarchy per root `AGENTS.md`: owner → CEO → Level B → Level A → specialists/workers. Level C is **not** approved (SDD §3 D-HIER). No slice before S09 encodes levels or tiers, under any D-HIER option, and no alias maps Level C to Level B.
- Exactly one root `OrgUnit` and one root `Position` per org; the root position's occupant is the CEO for global approval. One active occupant per position; one principal may hold many positions, subject to separation of duties. Published `JobTitleVersion`s are immutable.
- **Authority (errata):** PG schema `org` is the only authoritative store for `capability_grants` and `revocation_epoch`. Neo4j is a structural projection; there is **no grants projection into `agents`**. No projection or cache authorizes or keeps a revoked authorization. No cross-database grant and no authority transaction outside PG `org`. The runtime reads grant, assignment, policy ancestry and epoch from PG `org` before every effect; if it cannot, it denies/cancels.
- The outbox is written in the local PG `org` transaction and only propagates/rebuilds projections.
- **Approvals:** global policy needs owner + CEO, **distinct principals**, on the same hash/version. Subordinate policy needs the direct active superior, subset only. No approver may approve policy for a position they occupy or whose subtree contains a position they occupy. No self-approval, and interim delegation never approves. No eligible approver ⇒ blocked. Subordinate approval is bound to (policy version, superior position, approver assignment): reuse under another superior, re-parent, title-version change in T or a change/vacancy of the superior's occupant leaves the affected grants `reconciling`, with an epoch bump, until the current direct superior approves again (SDD §7.5; Critic F1, **proposal pending SEC-ORG-30**).
- **Agent approvers:** an agent approval counts only with P3-c2 agent-principal authentication bound to `AgentId`, assignment and lifecycle; an `AgentId` in a body/header is never proof. Interim rule: only humans approve; agent approvals are rejected with a stable error until P3-a is done and the P3-c2 credential exists (SDD §14).
- `task.start` is separate from `tool.invoke`; every effect rechecks the SoT. "Deny immediately" applies to checks after the revoke commit (TOCTOU window defined in P3-b, maximum `D-SEC-W`). Finance, payments, production and credential/permission changes stay denied until their own SDD, threat model and human approval.
- **Structural authorization (SDD §7.3 matrix, SEC-ORG-05):** act only within the strict subtree of the acting position; assign/transfer/unassign need the approver of the target level (root: owner only); no self-assignment; no re-parenting of one's own position or its ancestors; JobTitle versions are drafted/published only by owner or CEO; a transfer is unassign at the origin + assign at the destination in one transaction, each half authorized by its own row; closing a unit or retiring a position needs a vacant position and no active descendants (`409 position_occupied` / `409 active_descendants`); `OrgUnit` re-parent follows the position re-parent rules.
- **Activation (SEC-ORG-06):** an assignment made by anyone other than the eligible approver of the target level leaves its grants `reconciling` until that approver confirms.
- **Pending assignment (SEC-ORG-28, F-ORG-18):** a `pending_approver_confirmation` assignment is inert (no structural authority, approval or delegation → `403 assignment_pending`); the confirmer is the eligible approver and a principal distinct from the author (`403 confirmer_not_independent`); after `D-SEC-PENDING-TTL` (PG `now()`) it is closed as `expired`, with audit. SDD §7.5 rule (c) makes an approver's assignment to a position covered by its own policy pending (SEC-ORG-27).
- **Monotone authority (SEC-ORG-29, F-ORG-19):** `effective(T) = policy(T) ∩ effective(parent(T))`, evaluated at each decision from PG primary; re-parent, title-version change of T or an ancestor, new version/revocation of an ancestor policy and ancestor vacancy bump the epoch (or chain version) in the same transaction.
- **Security parameters:** `D-SEC-W`, `D-SEC-TTL`, `D-SEC-LEEWAY`, `D-SEC-AUTHZ-TIMEOUT`, `D-SEC-EPOCH-BUMP` and `D-SEC-PENDING-TTL` are owner decisions (threat model §6.4); this plan fixes no numbers.
- **Acting position (SEC-ORG-21):** every command/action names `acting_position_id`; no union of authority across a principal's positions.
- Existing `AgentCapabilities` stay authoritative in `agents` for current consumers; `org` grants never read or override them.
- No module-local pool, runner or `neo4rs` import. Docs/OpenKnowledge and Graphify updated per slice.

## Per-slice workflow (applies to every slice)

1. Owner agrees the slice's public seams (listed per slice) **before** any test is written (AGENTS.md G3).
2. Red: one test of one public behavior fails. Green: minimal implementation. Refactor during review, preserving tests.
3. The threat model exists and is normative. Slices marked **[SEGURANÇA]** must include the SEC-ORG criteria mapped to them (traceability table) as acceptance tests; a change to the threat model re-opens the affected slices.
4. Independent Critic reviews and records a verdict (template at the end). Author never approves own work.

## Review focus

1. Concurrent hierarchy edits that jointly create a cycle are serialized/denied (S06).
2. Stale or unavailable policy/epoch denies every check after the revoke commit; cache invalidation and in-flight revalidation (S11, S17).
3. Wrong-organization or spoofed actor/occupant never crosses tenants: domain, SQL constraints, HTTP auth and Neo4j traversal filtered by `org_id` (S02–S05, S07, S08, S13).
4. **(Replaces the former cross-database focus, per errata.)** Grants/epoch are authoritative only in PG `org`; a Neo4j projection or cache that is missing, lagging, duplicated or reordered never authorizes and never delays a revoke; outbox replay is ordered and idempotent (S08, S11).
5. Neo4j lag/down/rebuild never authorizes or loses PG state; rebuild only touches `graph_domain=org` (S08).
6. Separation of duties: the same principal never supplies both global approvals nor approves a policy it benefits from; agent approvals are verifiably attributed (S10).
7. Structural authorization: whoever assigns or re-parents cannot grant capability or pick their own approver outside the matrix; union across positions is impossible (S03, S06, S13, S11); a pending assignment confers nothing and an approval does not survive a change of superior (S03, S05, S09–S11, S13).
8. Authority never grows through structural change: epoch/chain bump in the same transaction and effective limit recomputed at each decision (S06, S11, S17).

## Decisions required from the owner (block only their dependents)

| ID | Decision (SDD §3) | Blocks |
|---|---|---|
| D-HIER | Level C: adopt (A), map to existing levels (B), level/tier as `org` data (C, added additively in S09) | S09, S10, S15, S16 |
| D-SEAM-AGENTS | `org::models` imports `AgentId`/`AgencyId`/`AgentLifecycleState` from `modules::agents` public re-exports vs. `application_contracts` | S01 |
| D-OUTBOX | Reuse core `graph_projection_outbox` (`graph_domain=org`) vs. dedicated `org` outbox | S05 (outbox write), S08, S11 |
| D-OWNER-BIND | Per-org owner vs. existing singleton `product_owner_bootstrap` | S12, S13 |
| D-OWNER-POSITION | May the owner occupy positions? Default: owner occupying any position blocks global policy | S10 |
| `D-SEC-*` | Security parameters of threat model §6.4: `D-SEC-W`, `D-SEC-TTL`, `D-SEC-LEEWAY`, `D-SEC-AUTHZ-TIMEOUT`, `D-SEC-EPOCH-BUMP`, `D-SEC-PENDING-TTL` | P1 (TTL, leeway), S05/S13 (pending TTL), S11 (authz timeout, epoch bump), S17 (W), S18 (epoch bump) |

## Prerequisite gates

P1/P2/P3 are identifiers, not priorities. None is passed. P3 is split as in [[planning/master-plan]] §3.19–§3.21 and §4.4 (P3-cat, P3-c1, P3-c2, P3-b, P3-a). An **approved** P1 is a prerequisite of S07, S10, S12 and S13 (security verdict, F-ORG-02).

### P1 — Owner authentication / IdP and bootstrap (human principals only)

- Provider, issuer/audience, signature and key rotation, expiry/revocation, mapping to `HumanPrincipal(issuer, subject)`; D-OWNER-BIND.
- Per-organization owner bootstrap via controlled operator setup or signed enrollment; no public self-appointment. `BOT_HTTP_ADMIN_TOKEN` and `VerifiedProductOwner` never substitute owner identity.
- Location of the auth seam is chosen in the P1 SDD: there is **no** `core/auth` today; the only HTTP seam is the admin bearer in `presentation/http/admin_auth.rs`.
- Agent principals are **not** in P1 (see P3).
- Per-principal `principal_auth_epoch`/session version checked on every request, short access token, logout-all and revocation on owner transfer (F-ORG-02, SEC-ORG-15); verified per-person identity or audited out-of-band approval for a human CEO distinct from the owner (F-ORG-06); owner binding only by P1 enrollment, never by env string (SEC-ORG-23).
- Acceptance: valid/invalid/expired/wrong-issuer/wrong-audience/replayed/org-mismatch credentials tested at a real HTTP entrypoint; actor always server-derived; SEC-ORG-15 tests (token rejected after logout-all/owner transfer, TTL ≤ `D-SEC-TTL`, leeway ≤ `D-SEC-LEEWAY` (owner decisions, threat model §6.4), `alg`/`none`/JWKS rotation).

### P2 — DB roles, migrations and isolated PostgreSQL

- Central DB/schema `org` ownership; runtime DML role vs. migration role; runtime role has only `INSERT`/`SELECT` on `org_audit_events`; core runner as sole executor.
- Disposable PG with the version the code requires (PG 18 vs. CI `pg16`: task T-CI-02); never the shared `trading_bot`. The isolation guard must not rely only on the DB name, because CI's ephemeral service is also named `trading_bot`.
- Acceptance: fresh/upgrade/rollback run isolated with cleanup proof; no pool or runner inside `org`.

### P3 — Runtime/tool-gateway package, capability catalog and agent-principal authentication

Split per [[planning/master-plan]] §3.19–§3.21 and §4.4. Wave-3 order: **P3-cat → P3-c1 → S09–S11 (human-only approval) → P3-c2 → P3-b → P3-a → S14, S17 + agent approval in S10**; P3-cat and P3-c1 may run in parallel.

- **P3-cat — capability catalog:** typed, closed capability/action/resource/limit schema, no enforcement; approval protocol; which changes are restrict-only vs. global-envelope. Needed by S09–S11 and P3-b.
- **P3-c1 — basic agent credential (after P1):** short-lived server-issued credential bound to `AgentId` + lifecycle, immediate revocation, an agent never authenticates as a human; no effect on approvals yet. Interim rule until agent approval is enabled: **only humans approve**. Needed by S09–S11. (Agent authentication sits in P3 because the runtime/tool gateway executes agent actions; there is no agent credential in the code today, and `promoted_by` in `/bots/runtime/promote` is an unauthenticated body field.)
- **P3-c2 — credential bound to assignment and epoch (after P3-c1, S05/S06 and S11):** the credential carries assignment + `revocation_epoch` and a change invalidates it at the next check; issuance, rotation and revocation; the only accepted proof for agent approvals (S10, after P3-a), agent revocations (SEC-ORG-26) and agent-initiated delegation (S14).
- **P3-b — tool gateway (after P3-c2):** reads grant/epoch on PG `org` before each effect; outbox owner, per-agent ordering, idempotency, retry/DLQ, registered consumer acks for `revocation_complete`; committed revoke increments the PG epoch and every later check denies, with no grace period; TOCTOU window ≤ `D-SEC-W` and audit of effects completed after the commit; high-impact deny.
- **P3-a — runtime/scheduler (after P3-c2 and P3-b):** `task.start` vs. `tool.invoke`, per-action authorization, cancellation/revalidation. Unlocks S14, S17 and agent approval in S10.
- Acceptance: separate SDDs reviewed by independent Critics with threat model; `org` implements no external effects.

## Dependency table

✔ = required; — = not required. "Other" lists slice and owner-decision dependencies.

| Slice | Former task | P1 | P2 | P3 | Other | [SEGURANÇA] |
|---|---|---|---|---|---|---|
| S01 Domain IDs, occupant reference, import rule | 1 | — | — | — | D-SEAM-AGENTS | — |
| S02 Structure invariants (units, titles, supervisor tree) | 1, 3 (pure) | — | — | — | S01 | — |
| S03 Position/assignment invariants + structural authorization matrix (pure) | 1, 4 (pure) | — | — | — | S01, S02 | — (implements SEC-ORG-05/06/21/27/28 as pure functions; the threat model is an existing input, not a gate) |
| S04 Schema + repository: org map, units, titles, positions | 2 | — | ✔ | — | S02 | tenant isolation, DB roles |
| S05 Schema + transactions: assignments and audit | 2, 4 | — | ✔ | — | S03, S04, D-OUTBOX, D-SEC-PENDING-TTL, F-ORG-18 incorporated | tenant isolation, pending assignment (SEC-ORG-28) |
| S06 Concurrent hierarchy edits and re-parent authorization (`SERIALIZABLE`) | 3 | — | ✔ | — | S03, S04, F-ORG-19 incorporated | privilege escalation (re-parent), monotone authority (SEC-ORG-29) |
| S07 Authenticated read-only API | 7 | ✔ (approved) | ✔ | — | S04, S05 | tenant isolation (incl. traversal), PII |
| S08 Neo4j structural projection | 6 | — | ✔ | — | S04, S05, D-OUTBOX | tenant isolation (traversal), PII |
| S09 Policy domain types (pure; tier if D-HIER=C) | 5 (pure) | — | — | P3-cat, P3-c1 | S01, D-HIER, SEC-ORG-30 confirmed (G1) | privilege escalation |
| S10 Policy approvals persistence + API | 5 | ✔ (approved) | ✔ | P3-cat, P3-c1 (human-only approval); agent approval after P3-a + P3-c2 | S09, S12, D-OWNER-POSITION, SEC-ORG-30 confirmed (G1) | privilege escalation, separation of duties, agent auth |
| S11 Grants and `revocation_epoch` | 5 | ✔ | ✔ | P3-cat, P3-c1 | S05, S10, D-OUTBOX, D-SEC-AUTHZ-TIMEOUT, D-SEC-EPOCH-BUMP, F-ORG-18/19 incorporated | revocation, TOCTOU, pending assignment, monotone authority |
| S12 Per-org owner bootstrap and transfer | 8 | ✔ (approved) | ✔ | — | S04, D-OWNER-BIND | owner bootstrap, actor derivation |
| S13 Mutating API: structure and assignments (no grants) | 8 | ✔ (approved) | ✔ | — | S03, S05, S06, S12, D-SEC-PENDING-TTL, F-ORG-18/19 incorporated | privilege escalation (matrix), tenant isolation, actor derivation |
| S14 Interim delegation | 5 | ✔ | ✔ | P3-a (after P3-c2, P3-b; agent-principal auth = P3-c2) | S10, S11 | authority delegation, agent auth |
| S15 Role serialization migration (only if D-HIER = A) | 9 (part) | — | ✔ | — | D-HIER | privilege escalation (role mapping) |
| S16 Hierarchy cutover from `agents.supervisor` | 9 | ✔ | ✔ | — | S05, S08, S12, S13, D-HIER, S15 if A | cutover authority |
| S17 Runtime/tool integration | 10 | ✔ | ✔ | P3-a, P3-b, P3-c2 (is P3) | S11, S14, D-SEC-W | revocation, high-impact deny |
| S18 Operations, docs, final verification | 11 | — | — | — | runs with each slice; release after all | — |

Changes relative to the previous draft: structure/assignment writes (S13) no longer wait for P3 because they emit no grants (consistent with SDD §19 step 4); structural Neo4j projection (S08) uses the existing core outbox and no longer waits for P3; policy/grant/delegation work (S09–S11, S14) and runtime (S17) keep P3. Critic cycle 1: agent-principal authentication assigned to P3 (S10, S14); separation of duties added to S10; S02 no longer carries a [SEGURANÇA] precondition; tier (D-HIER option C) moved to S09 as an additive change; traversal tenant filter added to S07/S08. Critic cycle 3 (threat model F-ORG-01): structural authorization matrix as pure function in S03, enforced in PG in S06 and at HTTP in S13; SEC-ORG-06 confirmation state in S05/S13 and grant effect in S11; acting position (SEC-ORG-21) from S01 to S17; SEC-ORG-03/10/14/19 mapped below. P1/P2/P3 columns unchanged. Critic cycle 4 + security verdict (threat model `c1d21216`): the P3 column now uses the master-plan split (S09–S11 on P3-cat + P3-c1 with human-only approval; S14, S17 and agent approval after P3-a); F-ORG-18/19 block S05, S06, S11, S13; approved P1 before S07, S10, S12, S13; the F1 approval binding (proposal pending SEC-ORG-30) conditions the G1 of S09/S10; `D-SEC-*` parameters added where used.

## Task 1 independence (evidence and slicing change)

**Verdict:** the original Task 1 was *almost* independent; it is now split into S01–S03, which depend on no DB, no auth, no runtime, no threat model and no D-HIER option.

Evidence from the code (2026-09-27):

- `backend/src/modules/org` does not exist; nothing to migrate or wire.
- The only external types S01–S03 need are `AgentId`, `AgencyId` (`modules/agents/models/identity.rs`, imports only `std::fmt` and `thiserror`) and `AgentLifecycleState` (`models/definition.rs`, pure enum). All are publicly re-exported by `modules/agents/mod.rs`. No SQLx, tokio, HTTP or config.
- Tests run as `cargo test --locked --bin bot modules::org::tests` without `DATABASE_URL`, Neo4j or IdP. The S01 import-direction rule is a shell-script change, also without external services.
- S01–S03 carry no [SEGURANÇA] precondition: same-org, acyclicity and the structural authorization matrix are pure functions over an in-memory snapshot. S03 implements SEC-ORG-05/06/21/27/28 from the already existing normative threat model; that document is an input, not a gate, and needs no DB, auth or runtime. The acting principal is a value (`OccupantRef` or owner `HumanPrincipal`), not an authenticated context.
- The only remaining pre-condition is **owner agreement** on D-SEAM-AGENTS and on the S01–S03 seams (process gate, not a technical dependency).

What was changed to make it true:

1. Removed policy and delegation state enums from Task 1 → S09 (they depend on the P3-cat capability catalog).
2. Removed any role/level/tier typing from Task 1 and from S04 → S09/S15 (depends on D-HIER).
3. `HumanPrincipal` in S01 is an opaque, bounded, normalized pair; IdP semantics and verification stay in P1.
4. "One active assignment per position" and "retired agent closes assignments" are pure rules over in-memory values in S03; the atomic transactional closure and DB uniqueness move to S05.
5. `OrgId↔AgencyId` is an in-memory mapping type in S01; the `UNIQUE` guarantee is S04.

## Slices

Builder/Critic are roles; concrete independent instances are assigned by the Orchestrator per AGENTS.md. Files are indicative.

### S01 — Domain IDs, occupant reference and import rule

- **Depends on:** none (D-SEAM-AGENTS agreement). **Builder:** backend domain. **Critic:** architecture/patterns.
- **Seams to agree:** `OrgId`, `OrgUnitId`, `JobTitleId`, `JobTitleVersionNo` (monotonic number of the `JobTitleVersion` entity), `PositionId`, `AssignmentId` constructors and normalization rules (proposal: same as `agents`: trimmed, non-empty, ≤64 chars, no control characters); `HumanPrincipal { issuer, subject }` as opaque validated pair; `OccupantRef::{Human, Agent(AgentId)}`; `ActingContext::{Owner, Position(PositionId)}` (SEC-ORG-21); `OrganizationAgencyMap(OrgId, AgencyId)`; `OrgDomainError` variants (including `ForbiddenScope`, `SelfAssignmentForbidden`, `AssignmentPending`, `ConfirmerNotIndependent`, `PositionOccupied`, `ActiveDescendants`).
- **Acceptance:** invalid IDs rejected with typed errors; `HumanPrincipal` `Debug`/`Display` never prints raw issuer/subject; no SQL/HTTP/tokio imports in `modules/org/models`; `check-import-direction.sh` gains a rule rejecting `modules::org::` in `presentation/http/routes` (today it only checks `modules::agents::`); the Builder proves it reproducibly with a script self-test: the script accepts an alternative `SRC` (e.g. `SRC=<fixture> bash scripts/check-import-direction.sh`), a committed fixture containing a forbidden `modules::org::` import in a routes path makes it exit non-zero with the `FAIL` line, and the real tree still passes; both outputs are recorded in the delivery log.
- **Files:** `modules/org/{mod.rs,models/ids.rs,models/occupant.rs,models/error.rs,tests.rs}`, `modules/mod.rs`, `scripts/check-import-direction.sh`.

### S02 — Structure invariants (pure)

- **Depends on:** S01. **Builder:** backend domain. **Critic:** architecture/patterns.
- **Seams to agree:** exactly one root `OrgUnit` and exactly one root `Position` per org (the only position with no supervisor; its occupant is the CEO); `JobTitleVersionState` transitions (`draft→published→retired`); pure validators `validate_unit_parent(...)`, `validate_supervisor(...)`, `is_in_strict_subtree(ancestor, target, snapshot)` over an in-memory snapshot, returning `OrgDomainError`.
- **Acceptance:** second root unit/position rejected; cross-org parent/supervisor rejected (SEC-ORG-07); unit and position cycles rejected; published version content immutable; a position is never in its own strict subtree.

### S03 — Position and assignment invariants (pure)

- **Depends on:** S01, S02. **Builder:** backend domain. **Critic:** QA.
- **Seams to agree:** `PositionState` (`filled` derived); `AssignmentState` (`pending_approver_confirmation | confirmed | expired | ended`; pending and expired never count as active); validators `can_assign(position, occupant, lifecycle)`, `can_freeze/retire(position, active_assignment)`, `assignments_to_close_on_retire(agent)`; lifecycle passed as `AgentLifecycleState` value; `StructuralOp` (create, close unit/retire position, open/freeze, assign/unassign, transfer O→T, root occupant change, re-parent position, re-parent `OrgUnit`, change title version, draft/publish/retire title version, bootstrap root); `authorize_structural(actor, acting: ActingContext, op, snapshot) -> Result<Activation, OrgDomainError>` with `Activation::{ActiveOnCommit, PendingApproverConfirmation, NoGrants}` (SDD §7.3 matrix).
- **Acceptance:** one active assignment per position; multiple positions per occupant accepted; frozen/retired positions refuse assignment; occupied position refuses freeze/retire; paused agent keeps assignment; retired agent cannot be assigned and yields the full close set. **Matrix (SEC-ORG-05): one positive and one negative test for each row of SDD §7.3**, through `authorize_structural`:
  1. Create unit/position under X — positive: A creates under its strict subtree → `NoGrants`, position vacant; negative: X outside A's scope → `ForbiddenScope`; second root → conflict.
  2. Close unit U / retire position T — positive: vacant T without active descendants in A's strict subtree → allowed; negative: occupied T (including pending) → `PositionOccupied`; active descendant position or unit → `ActiveDescendants`; T = A or an ancestor → `ForbiddenScope`.
  3. Open/freeze position T — positive: A freezes a vacant T in its strict subtree; negative: T = A or an ancestor → `ForbiddenScope`; freezing an occupied T → `PositionOccupied`.
  4. Assign/unassign non-root T — positive: direct superior assigns in its strict subtree → `ActiveOnCommit`; negative: intermediate occupant assigns to peer/superior/root → `ForbiddenScope`; assign self → `SelfAssignmentForbidden`.
  5. Transfer O→T — positive: actor with authority over both O and T → unassign + assign, activation as in row 4; negative: the approver of T pulls an occupant from an O outside its own strict subtree → `ForbiddenScope`.
  6. Root occupant change — positive: owner changes the root occupant → `ActiveOnCommit`; negative: any other actor → `ForbiddenScope`; owner assigns self to root → `SelfAssignmentForbidden`.
  7. Re-parent position T — positive: A re-parents T inside its strict subtree to N → allowed with T's grants `reconciling` and an epoch bump of T's subtree (SEC-ORG-29); negative: own position or an ancestor → `ForbiddenScope`; root → denied; cycle → hierarchy conflict.
  8. Re-parent `OrgUnit` U — positive: A re-parents U inside its unit scope → allowed with the same `reconciling` + epoch-bump effect on the affected positions; negative: U = `unit(A)` or an ancestor → `ForbiddenScope`; root unit → denied; cycle → hierarchy conflict.
  9. Change title version of T — positive: the approver of T's level changes the version → old grants `reconciling`, new grants active only with an approval of the pair (policy version, current `parent(T)`); negative: actor outside scope → `ForbiddenScope`, and an approval made under another superior does not activate. **The LGTM of this row depends on SEC-ORG-30** (SDD §20).
  10. Draft/publish/retire title version — positive: owner or root occupant; negative: any other position → `ForbiddenScope`.
  11. Bootstrap root — positive: owner at bootstrap; negative: a later or non-owner attempt → denied.
- **SEC-ORG-06:** owner assigning a non-root position → `PendingApproverConfirmation`. **SEC-ORG-27:** an approver assigned to a position covered by a policy it approved → `PendingApproverConfirmation`, and its grants stay withheld after confirmation until another eligible approver approves a new version. **SEC-ORG-28:** a pending or expired assignment used as acting position (assign, approve, delegate) → `AssignmentPending`; confirmation by the assignment's author, including the owner who also occupies `parent(T)` → `ConfirmerNotIndependent`; with `parent(T)` vacant the owner fills the chain from the root (`ActiveOnCommit`) and confirmation proceeds (no deadlock). **SEC-ORG-21:** an acting position the actor does not occupy → denied; authority from position X is never used for a target only reachable from position Y.

### S04 — Schema and repository: org map, units, titles, positions

- **Depends on:** P2, S02. **Builder:** data/backend. **Critic:** database. **[SEGURANÇA]** tenant isolation, DB roles.
- **Seams to agree:** migration at the next free sequence number (e.g. `00NN_org_structure.sql`) with table/constraint list and **no level/tier column**; `OrgRepository` port methods taking a `core` transaction and an `AuthorizedOrgScope` (SEC-ORG-03; production constructor only in the auth layer delivered with P1, test constructor behind `cfg(test)`); no own pool.
- **Acceptance:** SEC-ORG-03 (a) visibility test proves no public constructor of `AuthorizedOrgScope` outside the auth layer, (b) position with a unit of another org → FK violation, (c) every SELECT has a two-org test; isolated PG tests for cross-org FK rejection, `legacy_agency_id` uniqueness, one root unit/position per org, immutable published version guard, fresh + upgrade migration; runtime role has DML only.

### S05 — Assignments, audit and outbox write

- **Depends on:** P2, S03, S04, D-OUTBOX, D-SEC-PENDING-TTL; F-ORG-18 incorporated (SEC-ORG-28). **Builder:** data/backend. **Critic:** database. **[SEGURANÇA]** tenant isolation.
- **Seams to agree:** `assign_occupant`, `end_assignment`, `transfer_occupant`, `retire_agent_assignments`, `confirm_assignment` command shape (actor, `acting_position_id`, expected version, idempotency key, reason code); `assignments.confirmation_state` (`confirmed | pending_approver_confirmation`) and confirmer; audit event schema; outbox payload; `principal_ref = HMAC-SHA256(k, issuer‖subject)` with `k` from `core::config`, stored outside PG/Neo4j.
- **Acceptance:** partial unique index blocks a second active occupant; SEC-ORG-06 persistence: assignment recorded with the `Activation` from S03 and only an eligible approver's `confirm_assignment` sets `confirmed`; SEC-ORG-19: different keys yield different refs and audit/outbox payloads contain no `issuer`/`subject`; SEC-ORG-17: allowed and denied commands each produce exactly one audit event with `correlation_id`; SEC-ORG-24: `UNIQUE (org_id, idempotency_key)`, same key in two orgs gives independent commands; injected failure rolls back the whole transfer (assignment + audit + outbox); audit append-only enforced by privilege (runtime role has `INSERT`/`SELECT` only on `org_audit_events`) and proved by a test where `UPDATE` and `DELETE` with the runtime role fail; no grants written. **Transfer (F2):** unassign at O + assign at T in one transaction, each half authorized by its row; a failure in either half rolls back both. **SEC-ORG-28 (F-ORG-18, blocks the LGTM):** assignment rows carry author, `confirmation_state` (`pending_approver_confirmation | confirmed | expired`), confirmer and expiry; `confirm_assignment` by the author → `403 confirmer_not_independent`; after `D-SEC-PENDING-TTL` by PG `now()` the pending assignment reads as expired, is closed as `expired` with exactly one audit event, and can no longer be confirmed.

### S06 — Concurrent hierarchy edits

- **Depends on:** P2, S03, S04; F-ORG-19 incorporated (SEC-ORG-29). **Builder:** backend. **Critic:** database + security. **[SEGURANÇA]** privilege escalation (re-parent), monotone authority (SEC-ORG-29).
- **Seams to agree:** org-level lock + `SERIALIZABLE` + bounded retry on serialization failure only; re-parent command runs `authorize_structural` inside the same transaction after the lock, reading the epoch `FOR SHARE` (SEC-ORG-12).
- **Acceptance:** two concurrent edits that together form a cycle cannot both commit, the loser gets `409 hierarchy_conflict` (SEC-ORG-07); committed graph acyclic. Re-parent (SEC-ORG-05), isolated PG: positive, an occupant re-parents T inside its strict subtree to N, T's grants move to `reconciling` and the epoch of T's subtree is bumped in the same transaction, until the occupant of N approves T's policy for the pair with N (SDD §7.5; the actor may issue that approval when it occupies N and is eligible); negative, re-parent of own position, of an ancestor, of the root, or to an N outside the actor's scope → denied with zero rows changed and a deny audit event; a concurrent re-parent and assignment on the same subtree cannot both commit with a stale authorization. **`OrgUnit` re-parent (F3):** same positive/negative set as positions (own unit or ancestor, root unit, out-of-scope N, cycle → denied with zero rows changed and a deny audit event). **SEC-ORG-29 (F-ORG-19, blocks the LGTM):** re-parent of a position or unit, title-version change of T or an ancestor, and ancestor vacancy or occupant change each bump the affected epoch in the same transaction; injected failure → neither the change nor the bump persists.

### S07 — Authenticated read-only API

- **Depends on:** P1, P2, S04, S05. **Builder:** backend/HTTP. **Critic:** security. **[SEGURANÇA]** tenant isolation, PII.
- **Seams to agree:** OpenAPI for reads (org, tree, titles, positions, current occupant, history, chain); error contract; pagination/version token; every query (PG and Neo4j traversal) scoped by the actor's `org_id`.
- **Acceptance:** SEC-ORG-02 (member of A on `/orgs/{B}/…` → uniform `404 org_not_found`, zero B rows read), SEC-ORG-03 (repositories only via `AuthorizedOrgScope`), SEC-ORG-18 (PG failure → `503 org_store_unavailable` without driver text), SEC-ORG-20 (no private reads without the P1 verifier, regardless of `BOT_HTTP_ADMIN_TOKEN`); unauthenticated and wrong-org reads denied at the real route; a traversal request for another org's position returns nothing from that org (cross-org traversal test); DTOs redact principals; Neo4j-down traversal returns `degraded`, never empty.

### S08 — Neo4j structural projection

- **Depends on:** P2, S04, S05, D-OUTBOX. **Builder:** data/graph. **Critic:** database. **[SEGURANÇA]** tenant isolation (traversal), PII in projection.
- **Seams to agree:** projection payload (aggregate ID, `org_id`, version, event ID, redacted fields); new `GraphProjectionPort` method or generic event; traversal query API with mandatory `org_id` parameter; `graph_domain=org` added to the unified Neo4j strategy doc.
- **Acceptance:** SEC-ORG-04 (A/B fixture with a wrong `REPORTS_TO` A→B: A's traversal returns only A nodes; Cypher-payload ID → 400/empty and graph intact; no `format!`-built query); SEC-ORG-19 (projection carries `principal_ref`, never `issuer`/`subject`); every projected node carries `org_id`; traversal queries without `org_id` are not expressible through the port; cross-org traversal test with two orgs returns only the caller's org; idempotent `MERGE`; out-of-order version rejected; rebuild replaces only `graph_domain=org`; Neo4j outage leaves PG committed and outbox pending; no PII.

### S09 — Policy domain types (pure)

- **Depends on:** P3-cat, P3-c1 (interim: only humans approve), D-HIER, S01. **G1 condition:** Security confirms the F1 approval binding (SDD §7.5, proposal pending SEC-ORG-30). **Builder:** backend domain. **Critic:** security. **[SEGURANÇA]** privilege escalation.
- **Seams to agree:** `CapabilityPolicyVersion`, typed limits, `is_subset_of(ancestor)`, approval-requirement function per level (from D-HIER), hash definition, pure eligibility check `is_eligible_approver(principal, policy_scope, target_position, occupancy_snapshot)` (separation of duties): rejects a principal occupying the target position or any position in its subtree, with **one exception**: the CEO (root-position occupant) is eligible **only** for the global policy of its own org and **only** jointly with the distinct owner approval (SDD §7.5 rules a/b). If D-HIER = C: tier as a new nullable column on `job_title_versions` in its own migration plus a new published title version carrying the tier; published versions are never rewritten; a version without tier admits no subordinate policy. Approval key `(policy_version, superior_position, approver_assignment)` and pure `approval_is_current(approval, snapshot)` (F1, proposal pending SEC-ORG-30); `effective(T) = policy(T) ∩ effective(parent(T))`.
- **Acceptance:** expansion beyond ancestor rejected; effective limit is the chain intersection; hash changes invalidate approvals; eligibility rejects approver occupying the target position or its subtree; **positive test:** CEO is eligible for its own org's global policy when a distinct owner also approves; **negative tests:** CEO is not eligible for a subordinate policy of a position whose subtree contains another position the CEO occupies, CEO is not eligible for another org's global policy, and CEO approval alone (or with an owner who is the same principal) does not satisfy the global policy. **F1 (G1 condition of S09):** an approval recorded under superior P1 is not current for the same version under superior P2 (reuse or re-parent), nor after the occupant of `parent(T)` changes or it becomes vacant; rules (b) and (c) are evaluated against the current superior's position and assignment, never the original approver's. **SEC-ORG-27 (pure):** the approver of P assigned to a position covered by P → grants withheld until another eligible approver approves a new version; its own approval of the new version is ineligible.

### S10 — Policy approvals: persistence and API

- **Depends on:** approved P1, P2, P3-cat, P3-c1 (interim: only humans approve; agent approval enabled only after P3-a, with the P3-c2 credential), S09, S12, D-OWNER-POSITION. **G1 condition:** Security confirms the F1 approval binding (SEC-ORG-30). **Builder:** backend. **Critic:** security. **[SEGURANÇA]** privilege escalation, separation of duties, agent auth.
- **Seams to agree:** `approve_global_policy(actor, org, version, hash)`, `approve_subordinate_policy(superior_actor, …)`, `publish_policy`, `reject_policy`; actor from verified human auth (P1) or verified agent credential (P3) only; subordinate approvals act from `acting_position_id = parent(target)` (SEC-ORG-21) and are stored with `superior_position_id` and `approver_assignment_id` (F1, proposal pending SEC-ORG-30).
- **Acceptance:** single signature does not activate; mismatched hash/version, vacant owner/CEO, wrong-level approver and self-approval denied; **owner occupying the root (CEO) position ⇒ global policy blocked** (and, under the default of D-OWNER-POSITION, owner occupying any position); **a principal occupying the target position or any position in its subtree cannot approve**; approval re-checked at publish; no eligible approver ⇒ policy stays unpublished with an explicit blocked status; every agent approval rejected with a stable error until agent approval is enabled after P3-a (then only with a valid P3-c2 credential), and an `AgentId` only in body/header never counts; owner+CEO recorded as two distinct principals on the same hash (SEC-ORG-08, 09). SEC-ORG-21: approval declared from a position other than `parent(target)`, even if the principal also occupies `parent(target)`, → `403`; a principal occupying `parent(T)` and `parent(U)` cannot combine them in one approval. **F1 (G1 condition):** after reuse under another superior, a re-parent that changes `parent(T)`, or a change/vacancy of `parent(T)`'s occupant, the old approval activates nothing; only an audited `approve_subordinate_policy` by the current direct superior for that pair reactivates the grants, and approval by the previous superior → `403`. **SEC-ORG-27:** approval of the new version by the approver who now occupies the covered position → `403`, with audit. **SEC-ORG-28:** approval acting from a pending or expired assignment → `403 assignment_pending`. SEC-ORG-01: body with `actor`/`approved_by` → `400 unknown_field`. SEC-ORG-17: allow and deny of each approval audited. SEC-ORG-18: generic errors. SEC-ORG-20: no approval route without the P1 verifier.

### S11 — Grants and `revocation_epoch`

- **Depends on:** P1, P2, P3-cat, P3-c1, S05, S10, D-OUTBOX, D-SEC-AUTHZ-TIMEOUT, D-SEC-EPOCH-BUMP; F-ORG-18 and F-ORG-19 incorporated (SEC-ORG-28/29). No external effect consumes grants before P3-c2/P3-b. **Builder:** backend. **Critic:** security. **[SEGURANÇA]** revocation, TOCTOU, pending assignment, monotone authority.
- **Seams to agree:** grant states, origin/precedence, `acting_position_id` on each grant check, `org_revocation_epochs(org_id, scope_kind, scope_id, epoch)` with increment via `UPDATE … RETURNING` and anti-decrement trigger, read API the runtime calls (primary only).
- **Acceptance:** SEC-ORG-06/28: grants of an assignment in `pending_approver_confirmation` are `reconciling` and the read API denies; confirmation by the eligible approver, a principal distinct from the author → `active` with audit; an `expired` assignment's grants stay denied. SEC-ORG-21: principal with X (cap a) and Y (cap b): action needing a+b → deny; action with cap a declaring Y → deny. SEC-ORG-10: property test of strictly increasing epochs; runtime-role `UPDATE` to a lower value → error; injected failure after revoke → neither revoke nor increment persists. SEC-ORG-11/13: next check after commit with the cached epoch → `stale_epoch` even with the outbox worker stopped; PG unavailable → `authz_unavailable` within `D-SEC-AUTHZ-TIMEOUT`; replica pool not accepted by the authz seam; revoking an intermediate policy denies the rest of that chain. SEC-ORG-12: concurrent revoke × internal effect commits before or is denied, never after with the old epoch. SEC-ORG-14: migration down never decrements/removes epoch; restore bump ≥ `D-SEC-EPOCH-BUMP`. SEC-ORG-16: expiry uses PG `now()`; app clock ±10 min changes nothing. SEC-ORG-26: revoke attributed to an `AgentId` without a P3 credential → `403`. Assignment applies only approved, non-revoked policy grants whose approval is current for (policy version, current `parent(T)`, its current assignment); SEC-ORG-27: grants of a policy are withheld from an occupant who approved that policy until another eligible approver approves a version; committed revoke increments epoch and any check after the commit with the old epoch denies even while outbox is pending; revoking one origin keeps others; no projection or cache is consulted to allow; the TOCTOU contract from P3-b is referenced in the read API docs. **SEC-ORG-29 (F-ORG-19, blocks the LGTM):** (a) property test: for any tree and policies, `effective(T) ⊆ effective(A)` for every ancestor A; (b) re-parent of T to an N with a smaller envelope → the next decision for T and its descendants uses the new limit even with a warm cache, and a caller holding the old epoch gets `stale_epoch`; (c) narrowing a grandparent's policy → the grandchild's next action above the new limit is denied without re-approving the grandchild; (d) the same `JobTitleVersion`/policy under two superiors yields distinct effective limits, each equal to its own chain's intersection; (e) an old `active` grant never authorizes an action beyond the current intersection. **F1 (proposal pending SEC-ORG-30):** reuse under another superior, re-parent, title-version change in T and an occupant change or vacancy of `parent(T)` move the affected grants to `reconciling` with an epoch bump in the same transaction; the read API denies until the current direct superior approves the pair. SEC-ORG-17: allow and deny of each revoke and confirmation audited. SEC-ORG-18: generic errors. SEC-ORG-24: grant idempotency key `UNIQUE (org_id, key)`; the same key in two orgs gives independent commands.

### S12 — Per-organization owner bootstrap and transfer

- **Depends on:** approved P1, P2, S04, D-OWNER-BIND. **Builder:** backend/auth. **Critic:** security. **[SEGURANÇA]** owner bootstrap, actor derivation.
- **Seams to agree:** operator-only bootstrap command; `org_owner_bindings`; transfer requiring old+new owner approvals; relation to `product_owner_bootstrap`.
- **Acceptance:** no public self-appointment; one active owner per org; transfer audited; spoofed body owner rejected; SEC-ORG-23 (only `BOT_PRODUCT_OWNER_BOOTSTRAP_ID`/ACK → no row in `org_owner_bindings`; conflict logs pseudonymized). SEC-ORG-01: body with `owner_id`/`actor` → `400 unknown_field`. SEC-ORG-17: bootstrap and transfer, allowed or denied, each audited. SEC-ORG-20: no owner route without the P1 verifier, regardless of `BOT_HTTP_ADMIN_TOKEN`.

### S13 — Mutating API for structure and assignments (no grants)

- **Depends on:** approved P1, P2, S05, S06, S12, D-SEC-PENDING-TTL; F-ORG-18 and F-ORG-19 incorporated (SEC-ORG-28/29). **Builder:** backend/HTTP. **Critic:** security. **[SEGURANÇA]** tenant isolation, actor derivation.
- **Seams to agree:** OpenAPI commands for units/titles/positions/re-parent/assignments/`confirm_assignment`, each with `acting_position_id` (except owner); DTOs `deny_unknown_fields`; error codes `403 forbidden_scope`, `403 self_assignment_forbidden`, `403 assignment_pending`, `403 confirmer_not_independent`, `409 position_occupied`, `409 active_descendants`, `409 hierarchy_conflict`; feature gate name and default off. Human actors only until P3 provides agent credentials.
- **Acceptance:** no auth, forged actor, wrong org and version conflict tested at the real entrypoint against isolated PG; audit and outbox rows verified; routes fail closed when the gate is off (SEC-ORG-20). **Matrix at HTTP (SEC-ORG-05): one positive and one negative per row of SDD §7.3**; every negative asserts zero changed rows and exactly one deny audit event:
  1. Create unit/position — 2xx under own strict subtree / `403 forbidden_scope` outside it.
  2. Close unit / retire position — 2xx for a vacant T without active descendants / `409 position_occupied` for an occupied (or pending) T, `409 active_descendants` with an active descendant.
  3. Open/freeze position — 2xx freezing a vacant T in scope / `403 forbidden_scope` on own position or an ancestor, `409 position_occupied` freezing an occupied T.
  4. Assign/unassign non-root — direct superior 2xx, `confirmed` / intermediate occupant on peer/superior/root → `403 forbidden_scope`; self → `403 self_assignment_forbidden`.
  5. Transfer O→T — 2xx when the actor has authority over both ends (one transaction) / the approver of T pulling an occupant from outside its own strict subtree → `403 forbidden_scope`, O and T unchanged.
  6. Root occupant change — owner 2xx / anyone else → `403 forbidden_scope`; owner as occupant → `403 self_assignment_forbidden`.
  7. Re-parent position — 2xx inside the actor's strict subtree, with T's grants `reconciling` and the epoch bumped / own position or an ancestor → `403 forbidden_scope`.
  8. Re-parent `OrgUnit` — 2xx inside the actor's unit scope, with the same `reconciling` + epoch effect / own unit or an ancestor → `403 forbidden_scope`.
  9. Change title version — 2xx by the approver of T's level, old grants `reconciling` / outside scope → `403 forbidden_scope` (activation of the new version depends on SEC-ORG-30).
  10. Draft/publish/retire title version — owner or root occupant 2xx / other positions → `403 forbidden_scope`.
  11. Bootstrap root — positive covered by the S12 operator bootstrap test / an HTTP attempt to create a root → `403` (or `409` for a second root).
- **SEC-ORG-06:** owner assigns a non-root position → 2xx with `pending_approver_confirmation`; `confirm_assignment` by a non-approver → `403`, by the eligible approver → `confirmed`. **SEC-ORG-28:** the pending occupant acting from T (assign, approve, delegate) → `403 assignment_pending`; confirmation by the assignment's author, including the owner who also occupies `parent(T)` → `403 confirmer_not_independent`; after `D-SEC-PENDING-TTL` → `expired`, confirmation refused, audit event recorded. **SEC-ORG-21:** `acting_position_id` not occupied by the actor → `403`; body carrying `actor`/`owner_id`/`approved_by` → `400 unknown_field` (SEC-ORG-01). SEC-ORG-17/18/24 as in S05/S07.

### S14 — Interim delegation

- **Depends on:** P1, P2, P3-a (after P3-c2 and P3-b; agent-principal auth is P3-c2), S10, S11. **Builder:** backend. **Critic:** security. **[SEGURANÇA]** authority delegation, agent auth.
- **Seams to agree:** `create_interim_delegation`, `revoke_delegation`; scope, allowlist, start/end, policy version; delegator identity from P1 (human) or P3 (agent).
- **Acceptance:** SEC-ORG-22 (cross-org → 400; subdelegation → 403; after delegator revocation the delegate's next action → `stale_epoch`); cannot execute structural-matrix operations; expiry by PG `now()` (SEC-ORG-16); executes only pre-approved actions; cannot approve or subdelegate; delegation by an unauthenticated or body-declared agent rejected; invalid on expiry, delegator revocation, lifecycle/assignment change, stale epoch or revoked ancestor policy; delegation acting from a pending or expired assignment → `403 assignment_pending` (SEC-ORG-28/21). SEC-ORG-01: body with authority fields → `400 unknown_field`. SEC-ORG-20: no delegation route without the P1 verifier.

### S15 — Role serialization migration (only if D-HIER = A)

- **Depends on:** P2, D-HIER = A, accepted ADR. **Builder:** data/backend. **Critic:** database + security. **[SEGURANÇA]** role mapping.
- **Acceptance:** inventory of every serialized role value (Rust, HTTP `AgentRoleBody`, PG `CHECK` in `0002`, fixtures, Neo4j `a.role`, docs); explicit mapping; unknown/duplicate/unmappable value blocks; no alias Level C↔Level B; rehearsed in disposable DB with rollback; SEC-ORG-25 (no existing `level_b` gains approval over a new tier without an explicit entry in the owner-approved report hash).

### S16 — Hierarchy cutover from `agents.supervisor`

- **Depends on:** P1, P2, S05, S08, S12, S13, D-HIER (S15 if A), owner approval of the exact preflight hash. **Builder:** data/SRE. **Critic:** database + SRE. **[SEGURANÇA]** cutover authority.
- **Acceptance:** SEC-ORG-25 and F-ORG-13 (multiple CEOs, CEO reporting to an arbitrary `owner_id`) resolved in the divergence report; SDD §13 steps 1–9 evidenced in a disposable clone; zero unresolved mismatches; rollback restores legacy reads before accepting non-representable mutations; no dual writable hierarchy.

### S17 — Runtime and tool integration

- **Depends on:** P3-a, P3-b, P3-c2 (their own SDDs and threat model), S11, S14, D-SEC-W. Not implemented inside `org`.
- **Acceptance from `org`:** `task.start` independent of tool invocation; agent actions carry the P3-c2 agent credential and an `acting_position_id` (SEC-ORG-21); epoch checked on the primary before each effect (SEC-ORG-11); external check→effect window documented and measured (`authz_epoch_check_to_effect_ms` p99 ≤ `D-SEC-W`, owner decision, SEC-ORG-12); effective limit recomputed at each decision and a stale chain epoch denied (SEC-ORG-29); ancestry intersection before each effect; in-flight revalidation/cancel on policy/assignment/lifecycle change; stale/unavailable SoT denies; high-impact denied.

### S18 — Operations, docs and final verification

- Runs with every slice. SEC-ORG-14 rehearsal in isolated PG: restore an old snapshot + procedure (epoch bump ≥ `D-SEC-EPOCH-BUMP`, grants `reconciling`) → grants revoked after the snapshot stay denied; readiness green only after the audited bump. Runbook (backup/restore, projection lag/rebuild, owner bootstrap/recovery, approve/revoke, replay, cutover rollback); SLI/SLO and alerts before launch; `./scripts/verify-backend-gates.sh` (`cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, import direction, bin `bot` tests and listed integration tests), `./scripts/run-pg-integration-tests.sh` on isolated PG, Neo4j suite, OpenAPI; Graphify update; OpenKnowledge links. No deploy or cutover without explicit authorization.

## Traceability: threat model SEC-ORG → slice

Source: [threat model do `org`](../security/org-module-threat-model.md) §6.1 (normative, sha256 `c1d21216`). "Acceptance" points to the slice line that carries the criterion verbatim or by reference.

| SEC-ORG | Topic | Slice(s) | Acceptance line |
|---|---|---|---|
| 01 | Actor only from auth; `deny_unknown_fields` | S07 (read DTOs), S10, S12, S13, S14 | S10/S12/S14 SEC-ORG-01; S13 SEC-ORG-21 / `400 unknown_field` |
| 02 | Org from path only selects; uniform 404 | S07, S13 | S07 SEC-ORG-02 |
| 03 | `AuthorizedOrgScope` in every repository; composite FKs | S04, S07 | S04 SEC-ORG-03 (a)(b)(c); S07 |
| 04 | Parameterized Cypher with `org_id` on every node | S08 | S08 SEC-ORG-04 |
| 05 | Structural authorization matrix | S03 (pure), S06 (re-parent in PG), S13 (HTTP) | S03 Matrix; S06 Re-parent; S13 Matrix at HTTP |
| 06 | Non-approver assignment leaves grants `reconciling` | S03, S05, S06, S11, S13 | S03/S05/S11/S13 SEC-ORG-06; S06 Re-parent |
| 07 | Anti-cycle / same-org | S02, S06 | S02; S06 `409 hierarchy_conflict` |
| 08 | Separation of duties | S09, S10 | S09 eligibility; S10 |
| 09 | No self-approval; monotone subset | S09, S10, S14 | S09; S10; S14 (cannot approve) |
| 10 | Epoch table, `UPDATE … RETURNING`, anti-decrement | S11 | S11 SEC-ORG-10 |
| 11 | Epoch on primary, no permissive cache | S11, S17 | S11 SEC-ORG-11/13; S17 |
| 12 | Internal effects `FOR SHARE`; external window `D-SEC-W` | S06, S11, S17 | S06 seams; S11 SEC-ORG-12; S17 |
| 13 | Ancestor revocation invalidates descendants | S11 | S11 SEC-ORG-11/13 |
| 14 | Anti-rollback of epoch on restore/PITR/migration down | S11, S18 | S11 SEC-ORG-14; S18 rehearsal |
| 15 | Per-principal session epoch, short TTL, full claim validation | P1 | P1 acceptance |
| 16 | PG `now()` as authoritative clock | S11, S14 | S11 SEC-ORG-16; S14 |
| 17 | Audit of allow and deny | S05, S10, S11, S12, S13 | S05/S10/S11/S12 SEC-ORG-17; S13 (as in S05) |
| 18 | Generic errors, redacted `Debug` | S07, S10, S11, S13 | S07/S10/S11 SEC-ORG-18; S13 (as in S07) |
| 19 | `principal_ref` = HMAC with key outside the DBs | S05, S08 | S05 SEC-ORG-19; S08 |
| 20 | Fail-closed until P1, independent of admin token | S07, S10, S12, S13, S14 | S07/S10/S12/S14 SEC-ORG-20; S13 |
| 21 | Authorization by acting position, no union | S01 (type), S03, S10, S11, S13, S14, S17 | S03/S10/S11/S13/S17 SEC-ORG-21; S14 SEC-ORG-28/21 |
| 22 | Bounded interim delegation | S14 | S14 SEC-ORG-22 |
| 23 | Owner only by P1 enrollment | P1, S12 | S12 SEC-ORG-23 |
| 24 | `UNIQUE (org_id, idempotency_key)` | S05, S11, S13 | S05/S11 SEC-ORG-24 |
| 25 | Role mapping only by owner-approved report | S15, S16 | S15; S16 |
| 26 | Revocation by agent only with P3 credential | S11 | S11 SEC-ORG-26 |
| 27 | Rule (c): approver occupying a covered position → pending, grants withheld until another approver approves a new version | S03, S09, S10, S11 | S03/S09/S10/S11 SEC-ORG-27 |
| 28 | Pending assignment inert; distinct confirmer; `D-SEC-PENDING-TTL` → `expired` | S03, S05, S10, S11, S13, S14 | S03/S05/S13 SEC-ORG-28; S10/S14 `assignment_pending`; S11 SEC-ORG-06/28 |
| 29 | Monotone effective authority; epoch/chain bump on structural change | S06, S11, S17 | S06/S11 SEC-ORG-29; S17 |

SEC-ORG-30 does not exist yet. The F1 approval binding (SDD §7.5) is a **proposal pending SEC-ORG-30**; its acceptance lines are S03 row 9 (LGTM depends on it), S09/S10 F1 (G1 condition) and S11 F1. Add the SEC-ORG-30 row here once Security publishes it.

Threat-model code findings outside `org` (F-ORG-03, 10–13, 15, 16) are debts of existing seams (`agents`, admin HTTP, graph query); `org` does not reuse those seams.

## Per-slice review record (required before accepting any slice)

```markdown
### ENTREGA ORG-Sxx
- Builder:
- Resumo e artefato:
- Testes/evidências:
- Achados/revisão: <itens e evidências>
- Riscos, premissas e pendências:
- Veredito: PENDENTE | APROVADO | APROVADO COM FOLLOW-UP | REPROVADO — por <Critic independente>
```

Builder is never its own Critic. Critical/high findings block dependents until fixed or formally accepted by the authorized human. Independent review capacity must exist before a slice starts.

## Stop conditions

Any uncertainty about actor/tenant, agent-principal identity, role mapping, outbox/revocation, PG isolation or projection stops the dependent slices; pure, tested work that writes no authority (S01–S03, S09 once P3-cat, P3-c1 and D-HIER close) may continue. Do not substitute mocks for required real DB/HTTP behavior, and never proceed to grants or cutover on a failing tenant/policy/revocation/separation-of-duties test.

## Open design inputs

Resolved in P1/P2/P3 or owner decisions (SDD §3, §20): IdP and owner bootstrap/recovery, per-principal session epoch (SEC-ORG-15) and human-CEO identity distinct from owner (F-ORG-06); external check→effect window `D-SEC-W` (SEC-ORG-12); agent-principal credential (issuance, rotation, revocation); whether the owner may occupy positions; capability schema, quotas and finance/production rule; outbox storage, ordering, DLQ and consumer acks; TOCTOU window; DB roles and PG version (T-CI-02); Neo4j required-vs-degraded readiness; retention/pseudonymization/legal hold; SLI/SLO, RPO/RTO and cutover window; restrict-only vs. global-envelope policy changes; security parameters `D-SEC-*` (threat model §6.4); SEC-ORG-30 (F1 approval binding).

## Approval

This plan is **draft** and is not implementation authorization. The owner must approve the SDD revision, this plan, the decisions above and the seams of each slice before its tests are written.
