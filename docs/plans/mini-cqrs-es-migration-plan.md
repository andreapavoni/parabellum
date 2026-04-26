# mini_cqrs_es Migration Plan (`parabellum_app` + `parabellum_db`)

## Goal
Move from the current command/query handler + repository style to a true CQRS/ES architecture using `mini_cqrs_es`, with incremental rollout and no service freeze.

## Why This Matters Here
Current architecture has CQRS-style naming but not event sourcing:
- command handlers directly mutate repositories and enqueue jobs;
- queries hit transactional repositories;
- job side effects are embedded in command/job handler flows.

You gain with `mini_cqrs_es`:
- event history/auditing;
- stronger modularity around aggregates and commands;
- optimistic concurrency at event stream level;
- explicit projection pipeline for read models and side effects.

## External Constraints (as of April 26, 2026)
- `mini_cqrs_es` crate latest: `0.9.0` (published April 4, 2026).
- Library README states active development and possible breaking changes.

Design implication: integrate behind local traits and keep a controlled migration path.

## Target Architecture
1. **Write side**
   - Domain aggregates implement `mini_cqrs_es::Aggregate`.
   - Commands implement `mini_cqrs_es::Command` and emit domain events.
   - Event store persists immutable streams with optimistic concurrency.

2. **Read side**
   - Projections/read models updated by event consumers.
   - Queries read from projection stores (not from write aggregates).

3. **Side effects**
   - Job scheduling and external actions become event consumers (or orchestrated process managers), not ad-hoc inline writes.

## Migration Strategy
Hybrid mode with vertical slices:
- keep existing API routes and payloads;
- migrate one bounded behavior at a time to event-sourced commands;
- run old and new command paths in parallel by feature flag/routing switch.

## Phased Plan

### Phase 0 — Domain/Event Modeling and Boundaries
Deliverables:
- Define aggregate boundaries and stream IDs (initial shortlist):
  - `VillageAggregate`
  - `ArmyMovementAggregate` (or substream strategy)
  - `MarketplaceAggregate`
  - `PlayerAggregate`
- Define canonical domain events per aggregate.
- Create event versioning and metadata conventions.

Exit criteria:
- Written event catalog with invariants and replay rules for each aggregate.

---

### Phase 1 — CQRS/ES Infrastructure in `parabellum_db`
Deliverables:
- Implement `mini_cqrs_es::EventStore` in `parabellum_db`.
- Optionally implement `SnapshotStore` for high-churn aggregates.
- Add projection storage schema/tables.
- Build `SimpleCqrs` composition root in `parabellum_app`.

Exit criteria:
- One toy aggregate can execute command -> persist events -> replay -> query projection.

---

### Phase 2 — First Real Vertical Slice (Recommended: Building Queue)
Scope:
- Migrate one command family end-to-end:
  - `AddBuilding` / `UpgradeBuilding` / `DowngradeBuilding`

Deliverables:
- New commands emit events (instead of direct repo writes).
- Consumers update read model tables and schedule jobs.
- Existing API endpoints call new CQRS engine for this slice only.

Exit criteria:
- Old and new flows are behaviorally equivalent for this slice.
- Replay from empty projections reconstructs correct read state.

---

### Phase 3 — Job System Refactor to Event-Driven Side Effects
Deliverables:
- Convert job creation from inline command-handler writes to event consumers.
- Define idempotency keys for consumer processing.
- Make worker-safe retry semantics explicit.

Exit criteria:
- Reprocessing the same event does not duplicate jobs or corrupt state.

---

### Phase 4 — Query Side Migration
Deliverables:
- Move query handlers to projection-backed reads.
- Remove dependence on write-side repositories for query operations.
- Add projection lag/consistency strategy (synchronous update or acceptable eventual consistency windows).

Exit criteria:
- Query handlers no longer need write-side UoW semantics.

---

### Phase 5 — Expand by Aggregate Family
Order suggestion:
1. Village build/research/train domain
2. Army movement and combat
3. Marketplace flows
4. Player progression/reporting

Per family:
- migrate commands and events;
- migrate projections;
- remove old command handlers and repo write paths.

Exit criteria:
- Old command handler modules for migrated family deleted.

---

### Phase 6 — Decommission Legacy CQRS Layer
Deliverables:
- Retire `AppBus` generic command/query path (or keep as thin facade over new CQRS engine).
- Remove legacy command handler trait machinery and direct write repositories where obsolete.
- Keep compatibility adapters only if needed for remaining modules.

Exit criteria:
- `mini_cqrs_es` is primary write path across core gameplay actions.

## Data and Consistency Rules to Lock Early
1. Event naming/versioning policy.
2. Per-aggregate concurrency policy (`expected_version` behavior).
3. Projection update guarantees (at-least-once consumer + idempotency).
4. Backfill/replay operational procedure.
5. Failure policy for consumer errors (retry, dead-letter, manual recovery).

## Testing Plan
1. Aggregate unit tests: command -> emitted events; apply/event replay.
2. Event store tests: optimistic concurrency conflict cases.
3. Projection tests: idempotent application and ordering.
4. End-to-end slice tests from API command to query read model.
5. Replay tests: rebuild projections from event history and compare with live tables.

## Key Risks and Mitigations
- Increased boilerplate:
  - Mitigation: shared crate modules for event metadata, consumer wiring, projection helpers.
- Dual-write/dual-path complexity during migration:
  - Mitigation: strict slice-by-slice ownership; no mixed old/new writes for same invariant.
- Event schema drift:
  - Mitigation: explicit versioning and upcaster strategy before production rollout.

## Recommended Sequencing with Toasty Plan
1. Build persistence abstraction + `toasty` base first.
2. Implement CQRS/ES infra on top of that persistence layer.
3. Migrate command families incrementally.

This avoids implementing event store/projections directly on legacy repository internals that will be removed soon after.
