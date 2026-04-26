# Toasty Migration Plan (`parabellum_app` + `parabellum_db`)

## Goal
Replace the current `sqlx` + manual repository/UoW implementation with `toasty`-based persistence, while preserving behavior and enabling a later CQRS/ES migration.

## Why This Matters Here
- Today, `parabellum_app` depends on `UnitOfWork` and many repository traits.
- `parabellum_db` implements those traits via `Arc<Mutex<Transaction>>` and manual SQL/mapping.
- Complex read/write methods (example: village/job repositories) mix query shape, transaction concerns, and mapping logic.

`toasty` should reduce boilerplate and centralize persistence model definitions, but we need an adapter-first migration to avoid a big-bang rewrite.

## External Constraints (as of April 26, 2026)
- `toasty` crate latest: `0.4.0` (published April 13, 2026).
- `toasty` is explicitly marked preview / API not stable yet.

Design implication: isolate `toasty` behind a thin local persistence boundary so future `toasty` API changes are contained.

## Target State
1. `parabellum_db` owns `toasty` models + persistence adapters.
2. `parabellum_app` domain logic does not depend on `toasty` types.
3. Transaction handling is simplified and explicit in application service boundaries.
4. Existing API behavior remains unchanged.

## Migration Strategy
Use a strangler approach with compatibility layers:
- keep existing trait contracts initially;
- migrate one repository family at a time to `toasty`;
- remove UoW complexity only after enough repositories are migrated and tested.

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

### Phase 4 — Remove UoW Complexity (after most repos are migrated)
Deliverables:
- Replace `UnitOfWork` internals with simplified transactional/session boundary.
- Remove `Arc::try_unwrap` transaction ownership trap in commit/rollback path.
- Keep `AppBus` command/query APIs stable (initially), but simplify internals.

Exit criteria:
- Command/query execution still transactional where needed.
- Commit/rollback no longer depend on fragile multi-owner runtime checks.

---

### Phase 5 — Cleanup + Hardening
Deliverables:
- Delete obsolete `sqlx` query-heavy repository code.
- Reduce mapping duplication (`mapping.rs`) where `toasty` models can be mapped cleanly.
- Refresh `test_utils` mocks only where contracts changed.

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
