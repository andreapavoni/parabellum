# Toasty Migration Plan (`parabellum_app` + `parabellum_db`)

## Goal
Hard-switch from `sqlx` + transaction-heavy UoW internals to `toasty` session-based persistence, preserving behavior while removing UoW transaction complexity and unlocking CQRS/ES-first command flows.

## Why This Matters Here
- Today, `parabellum_app` depends on `UnitOfWork` and many repository traits.
- `parabellum_db` implements those traits via `Arc<Mutex<Transaction>>` and manual SQL/mapping.
- Complex read/write methods (example: village/job repositories) mix query shape, transaction concerns, and mapping logic.

`toasty` gives better model ergonomics (including embedded/json-like shapes) and reduces UoW-specific ceremony. This plan now assumes a hard switch on a dedicated branch.

## External Constraints (as of April 26, 2026)
- `toasty` crate latest: `0.4.0` (published April 13, 2026).
- `toasty` is explicitly marked preview / API not stable yet.

Design implication: isolate `toasty` behind a thin local persistence boundary so future `toasty` API changes are contained.

## Target State
1. `parabellum_db` owns `toasty` models + persistence adapters.
2. `parabellum_app` domain logic does not depend on `toasty` types.
3. UoW commit/rollback choreography is minimized (or no-op for toasty sessions).
4. Existing API behavior remains unchanged.
5. Toasty embedded types are preferred over ad-hoc JSON plumbing where it improves model clarity.

## Migration Strategy
Hard switch with behavior parity:
- prioritize toasty-backed runtime paths;
- keep compatibility shims only where required by test/runtime constraints;
- remove UoW transactional complexity early and avoid reintroducing it.

## Phased Plan

### Phase 0 — Discovery Spike (1 short PR)
Deliverables:
- Add `toasty` dependency in `parabellum_db` only.
- Build one disposable POC model + query against a non-critical table (or test table) to validate:
  - connection lifecycle;
  - transaction API;
  - async ergonomics with your runtime;
  - compatibility with current migration flow.

Exit criteria:
- We can execute one read + one write transactionally with clear error mapping to `ApplicationError`.

---

### Phase 1 — Persistence Core Abstraction
Deliverables:
- Introduce `parabellum_db::persistence` module:
  - `DbSession`/`TxSession` wrapper abstraction (local to this repo);
  - conversion utilities between domain models and persistence models;
  - centralized DB error mapping.
- Keep current repository traits unchanged in `parabellum_app`.

Exit criteria:
- No behavior changes; compile-only refactor with tests green.

---

### Phase 2 — Pilot Repository Migration
Recommended pilot: `JobRepository` (high impact, bounded surface, strongly testable).

Deliverables:
- Reimplement `PostgresJobRepository` on `toasty` via the new persistence abstraction.
- Preserve existing method signatures and semantics (`find_and_lock_due_jobs`, `reschedule`, status transitions).
- Add regression tests for locking and status transitions.

Exit criteria:
- Job worker behavior unchanged.
- No measurable query regression on common worker loop paths.

---

### Phase 3 — Repository Family Migration by Domain Slice
Migrate in this order (write-risk first, then read-heavy):
1. `users`, `players`
2. `villages`, `armies`, `heroes`
3. `marketplace`, `reports`, `map`

Per slice:
- migrate repository implementation;
- keep app-level trait unchanged;
- add/refresh integration tests for that slice.

Exit criteria (per slice):
- API and job flows for that slice pass existing tests.
- SQLx implementation for that slice can be deleted.

---

### Phase 4 — Remove UoW Complexity (hard-switch priority)
Deliverables:
- Replace UoW internals with toasty session boundary (no transaction lifetime coupling).
- Remove fragile transaction-ownership patterns (`Arc::try_unwrap`, explicit rollback-on-read paths).
- Keep `AppBus` API stable while removing mandatory commit/rollback choreography for non-transactional sessions.

Exit criteria:
- Runtime path no longer depends on explicit SQL transaction ownership checks.
- UoW semantics are transitional and lightweight, ready to be replaced by CQRS/ES orchestration.

---

### Phase 5 — Cleanup + Hardening
Deliverables:
- Delete obsolete `sqlx` query-heavy repository code.
- Reduce mapping duplication (`mapping.rs`) where `toasty` models can be mapped cleanly.
- Refresh `test_utils` mocks only where contracts changed.
- Replace raw JSON blobs with toasty embedded types where domain/value-object structure is known and stable.

Exit criteria:
- `parabellum_db` no longer exposes old repository internals.
- Persistence layer is ready to support CQRS/ES event store + projections.

## Risks and Mitigations
- `toasty` API churn:
  - Mitigation: local `persistence` adapter boundary; pin crate version.
- Hidden query behavior changes:
  - Mitigation: per-repository integration tests + query performance snapshots.
- Transaction semantics drift:
  - Mitigation: explicit transaction-focused tests for command handlers with side effects.

## Suggested PR Breakdown
1. `db: add toasty + persistence adapter skeleton`
2. `db: migrate job repository to toasty`
3. `db: migrate user/player repositories`
4. `db: migrate village/army/hero repositories`
5. `db: migrate marketplace/report/map repositories`
6. `app+db: simplify uow lifecycle`
7. `cleanup: remove old sqlx repository code`

## Dependencies with CQRS/ES Plan
- This migration should happen **before or alongside** CQRS/ES infra work, because CQRS/ES introduces new persistence needs (event store + projections).
- Avoid deep refactors in `AppBus` until CQRS/ES integration design is finalized.
