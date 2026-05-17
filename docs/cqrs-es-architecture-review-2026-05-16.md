# CQRS/ES Architecture Review (2026-05-16)

## Scope
- Reviewed: `README.md`, `docs/ARCHITECTURE.md`, and key CQRS/ES runtime/projector/repository code paths.
- Focus: consistency model, aggregate boundaries, read-model ownership, replay/scheduler behavior, and refactor opportunities.

## Executive Summary
The refactor has strong foundations: explicit layered crates, a clear single facade (`GameApplication`), event-driven scheduler flow, and replay tooling. The most important blind spots are around **projection mutability during reads**, **coupling of projector with cross-aggregate business policy**, and **error typing erosion at adapter boundaries**.

## What is structurally strong
1. **Clear layering and composition**
   - `GameApplication` is a focused app facade and keeps web layer orchestration clean.
2. **Operationally useful ES runtime**
   - Dedicated replay service with dry-run/full modes and sequence windows.
3. **Scheduler discipline**
   - Scheduler worker issues commands via service rather than mutating model state directly.
4. **Read-model ownership intent documented**
   - Good explicit contract in `docs/ARCHITECTURE.md` about `rm_armies` vs `rm_village_movements` vs `rm_village`.

## Blind spots and architectural risks

### 1) Read-side queries should compute live values, but must not persist them
**Observation**
`PostgresVillageRepository::refresh_for_read` currently recomputes **and persists** derived village state when reading (`list_by_player_id`, `list_by_village_ids`, etc.). The recomputation itself is useful (resources/CP are time-based and need “as-of-now” values), but persisting during read turns query paths into writers.

**Why it matters**
- Violates CQRS read/write separation in practice.
- Makes read throughput dependent on write IO.
- Risks subtle non-determinism in replay validation because projection correctness is partly “healed” by reads.

**Refactor direction**
- Keep the **read-time recomputation**, but make it **read-only** (derive an ephemeral view returned to clients without `UPDATE rm_village`).
- Explicitly model this as “live projection view hydration” for ticking resources and culture points.
- Keep persisted read models as last-materialized snapshots; never heal them from read endpoints.
- If needed, add periodic/background materialization for operational reporting, but not in request path.

### 2) Projector contains game-policy decisions beyond pure projection
**Observation**
`VillageProjector` includes logic like conquest eligibility checks and culture-point update checks (`can_attempt_conquer`), and resource deduction utility.

**Why it matters**
- Projectors should ideally be deterministic materializers of already-decided events.
- Domain-policy in projector creates dual decision centers (aggregate + projector).
- Increases replay fragility if behavior of helper methods changes over time.

**Refactor direction**
- Treat projector as a “dumb” event applier.
- Move policy to command/aggregate phase and emit richer events with already-decided facts.

### 3) Error typing is flattened to `ApplicationError::Unknown` in adapter edges
**Observation**
`VillageEsAdapter` maps many failures via `map_err(|e| ApplicationError::Unknown(e.to_string()))`, with selective string matching for hero cases.

**Why it matters**
- Loses machine-actionable semantics at API boundary.
- Encourages brittle message-based mapping.
- Makes contract matrix less enforceable as domain grows.

**Refactor direction**
- Introduce typed domain/command failure enum(s) from ES service.
- Perform exhaustive typed mapping to `ApplicationError::Game`/`Conflict`/`NotFound` variants.

### 4) Dual army sources increase drift risk (even if documented)
**Observation**
Architecture docs declare `rm_armies` canonical for troop state while `rm_village` still stores compatibility army snapshots (`army`, `reinforcements`, `deployed_armies`).

**Why it matters**
- Any bug or missed projector branch can desynchronize the snapshots.
- Query authors may accidentally use stale compatibility fields.

**Refactor direction**
- Create an explicit deprecation path for compatibility fields.
- Add repository-level lint/tests that forbid new query paths from using `rm_village.army*` as authority.

### 5) Scheduler tick model can become hotspot under load
**Observation**
Worker polls every second with fixed batch limit and logs only when processed > 0.

**Why it matters**
- Potential fairness issues when due queue spikes.
- Harder to observe lag without explicit metrics (queue depth, oldest due age).

**Refactor direction**
- Add metrics: due count, max lateness, per-action throughput/failure counters.
- Consider adaptive batch size or bounded “drain-until-empty with time budget”.

### 6) Replay safety is good, but replay determinism checks are missing
**Observation**
Replay can fully rebuild projections and uses advisory lock, but no explicit post-replay consistency validation appears in the reviewed path.

**Why it matters**
- Large ES systems benefit from invariant checks after replay (counts, null constraints, cross-table integrity).

**Refactor direction**
- Add replay verification mode that computes and reports invariant violations (e.g., movement rows linked to armies, single home army per village, no negative stocks, etc.).

## High-value refactoring opportunities (prioritized)

### Priority A (next iteration)
1. Replace write-on-read with read-only live hydration in `PostgresVillageRepository` (for ticking resources/CP), while keeping projector as the only persistent writer.
2. Introduce typed adapter error mapping end-to-end.
3. Add architectural tests guarding read-model authority boundaries.

### Priority B
4. Simplify `VillageProjector` responsibilities to pure projection; move policy upstream.
5. Add scheduler observability and lag dashboards.
6. Add replay verification/invariant pass.

### Priority C
7. Gradually phase out compatibility army snapshots in `rm_village` from query consumers.
8. Introduce explicit versioned event upcasters strategy before event schema complexity grows further.

## Suggested architecture fitness tests
- **Projection purity + live-view test**: ensure query repositories do not call mutating SQL in reads, while still returning up-to-date resource/CP values at time *t*.
- **Authority test**: troop availability endpoints must source from `rm_armies` only.
- **Replay idempotency test**: replaying same range twice yields stable table checksums.
- **Scheduler determinism test**: same due action payloads produce same emitted events in controlled clock harness.

## Open questions for you
1. Do you want strict-event-sourcing purity (no write-on-read anywhere), or is pragmatic materialization healing acceptable?
2. Are compatibility `rm_village.army*` fields temporary for migration, or intended long-term?
3. Is eventual support for async projectors planned, or do you want to keep strict in-transaction projection only?
4. Do you want a follow-up patch that implements Priority A.1 + A.2 with minimal API impact?
