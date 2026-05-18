# CQRS/ES Migration Plan

Last updated: 2026-05-18
Status: Draft, ready for execution
Owner: Core backend architecture

## Progress Update (2026-05-18)

1. Legacy `CompleteMerchantsArrival` command path removed.
2. Marketplace acceptance now has explicit application fact payload (`MarketplaceOfferAcceptanceAppliedToVillage`) used by projectors for stock/merchant materialization.
3. Village ES service gained a dedicated workflow-append preparation seam (`build_village_workflow_appends`) to ease future extraction into `mini_cqrs_es`.
4. Marketplace acceptance now uses explicit accepting-village application fact (`MarketplaceOfferAcceptanceAppliedToVillage`) and no longer relies on projector inference.
5. Scout battle simulation moved out of projectors into scheduler workflow fact production (`ScoutBattleResolved`).

## Goals

1. Enforce strict consistency for command handling and projection, preferably in one DB transaction.
2. Keep `VillageAggregate` as a main aggregate while supporting cross-village workflows correctly.
3. Treat passive resource production as deterministic derived state (materialized at write boundaries, derived at read/command boundaries).
4. Ensure replay rebuilds read models from happened facts only, without re-enqueuing operational delayed work.

## Non-Goals

1. Rewriting all gameplay mechanics in one pass.
2. Introducing eventual consistency between event store and read models.
3. Changing API contracts unless necessary for correctness.

## Key Architectural Decisions (Target State)

1. Events are immutable facts, not execution instructions.
2. Projectors are pure reducers: no battle calculations, no UUID generation, no command decisions.
3. Delayed execution queue is operational state, separate from replay-only read models.
4. Cross-village flows are coordinated by an application workflow/process manager that appends facts to affected aggregate streams atomically.
5. Replay only rebuilds projections; scheduler queue rebuild is an explicit, separate maintenance operation if needed.

## Current Blind Spots to Fix

1. `VillageProjector` includes domain decisions and side effects.
2. Some target-village mutations happen only in read models instead of event streams.
3. Runtime and replay apply projectors in different order.
4. Scheduler failure handling can leave actions stranded in `processing`.
5. Read paths mutate read models (`refresh_for_read`) and blur CQRS boundaries.

## Workstreams

## WS1 - Transactional CQRS Pipeline

Objective: append events + project read models + enqueue delayed commands atomically.

Tasks:
1. Introduce a transaction-scoped infrastructure boundary (`DbSession` or equivalent) passed to event store, projector, and queue writer.
2. Refactor command execution path to run inside one transaction:
   1. Load aggregate state.
   2. Execute command logic.
   3. Append events to one or more streams.
   4. Project read models from appended events.
   5. Write operational delayed commands (if any).
   6. Commit.
3. Add rollback tests for projector/queue failure after event append attempt.

Exit criteria:
1. No committed events without corresponding read-model updates in successful command paths.
2. Integration test proves rollback on injected projection failure.

Status checklist:
- [ ] Design transaction-scoped interfaces
- [ ] Refactor event store to tx-scoped API
- [ ] Refactor projector writes to tx-scoped API
- [ ] Refactor delayed-command writes to tx-scoped API
- [ ] Add atomicity tests

## WS2 - Projector Purification

Objective: turn projectors into pure, idempotent reducers.

Tasks:
1. Enumerate projector side effects currently doing domain work:
   1. Battle calculation.
   2. Return scheduling.
   3. Map occupancy claim logic with runtime decisions.
   4. Any `Uuid::new_v4()` in projection path.
2. Move side effects into command/workflow layer and emit explicit result events.
3. Keep projector logic limited to deterministic state materialization from event payloads.
4. Enforce deterministic event payloads (all IDs/timestamps resolved before append).

Exit criteria:
1. Replaying same event window twice produces identical read models.
2. Projector code has no command orchestration logic.

Status checklist:
- [x] Catalog projector side effects
- [x] Introduce explicit result events where needed
- [ ] Move battle/result logic out of projector
- [ ] Remove ID generation from projector
- [ ] Add replay determinism tests

## WS3 - Cross-Village Consistency Model

Objective: preserve `VillageAggregate` while handling multi-village invariants correctly.

Tasks:
1. Define workflow boundaries for:
   1. Attack/raid/scout.
   2. Reinforcement send/arrive/recall/release.
   3. Merchants transfer/offer accept.
   4. Settlers/founding/conquest.
2. Implement workflow-level multi-stream append with per-stream expected versions.
3. Choose conflict behavior for stale versions:
   1. Fail-fast to caller.
   2. Retry with bounded attempts.
4. Ensure target-village material state changes are emitted on target stream facts.

Exit criteria:
1. No target-village mutation exists only in projector state.
2. Multi-village commands are conflict-safe and test-covered.

Status checklist:
- [x] Define workflow contracts and ownership
- [x] Add multi-stream append primitive
- [x] Implement conflict policy
- [x] Migrate attack flow first
- [ ] Migrate merchants/reinforcement/settlers flows

## WS4 - Scheduler Domain Boundary

Objective: separate replayable facts from operational delayed command queue.

Tasks:
1. Split concepts:
   1. Replayable movement/action timeline read models.
   2. Non-replay operational queue (`scheduled_commands`).
2. Ensure replay does not recreate pending operational commands.
3. Fix scheduler batch failure semantics so no action remains indefinitely `processing`.
4. Add maintenance endpoint/tooling for explicit queue repair if required.

Exit criteria:
1. Replay is side-effect free with respect to operational queue.
2. Scheduler has bounded, recoverable failure states.

Status checklist:
- [ ] Define schema split
- [ ] Refactor scheduler read/write paths
- [ ] Prevent replay queue pollution
- [ ] Add stranded-action recovery strategy
- [ ] Add scheduler resilience tests

## WS5 - Passive Resource Derivation

Objective: keep resource production deterministic without write-on-read side effects.

Tasks:
1. Decide read policy:
   1. Return derived current stocks in query response without persisting.
   2. Persist only on command boundary or explicit maintenance compaction.
2. Remove repository auto-write behavior during read (`refresh_for_read` pattern).
3. Ensure command handlers materialize current resources at transaction start for validation.

Exit criteria:
1. Read methods are side-effect free.
2. Resource checks remain correct under elapsed time.

Status checklist:
- [ ] Define canonical materialization boundary
- [ ] Remove write-on-read from village repository
- [ ] Add command-time resource materialization helper
- [ ] Validate with integration tests

## WS6 - Replay Contract Hardening

Objective: make replay deterministic, safe, and operationally predictable.

Tasks:
1. Align live and replay projector execution order.
2. Ensure replay consumes facts only and never re-runs domain decisions.
3. Add replay invariants:
   1. Repeat replay idempotence.
   2. Bounded replay window consistency.
   3. Cross-projector ordering consistency.
4. Optionally track and validate projector offsets for incremental replay modes.

Exit criteria:
1. Same event range replay yields byte-equivalent read-model state (or deterministic equivalence).
2. Replay does not enqueue operational delayed commands.

Status checklist:
- [ ] Align projector order
- [ ] Add idempotent replay tests
- [ ] Add bounded-window replay tests
- [ ] Add operational safeguards/documentation

## Execution Sequence (Recommended)

1. WS4 quick safety patch (scheduler stranded `processing` + replay queue separation guard).
2. WS6 order alignment and replay test harness.
3. WS2 projector purification for one vertical slice (attack flow).
4. WS3 multi-stream workflow append for attack flow.
5. WS1 full transaction-scoped command pipeline.
6. WS5 resource derivation cleanup.
7. Expand WS2+WS3 migration to reinforcement, merchants, settlers/founding, conquest.

## M2 Attack Flow Design (Outcome-Driven)

Objective: remove domain decisions from projector for attack resolution and conquest, and emit explicit replay-safe facts.

### Current flow to replace

1. Scheduled action dispatches `CompleteAttackArrival`.
2. Command emits `AttackArrived`.
3. `VillageProjector` computes battle outcome and mutates source/target read models.
4. Scheduler may call `ConquerVillage` command based on post-projection state.

Problems:
1. Conquest is inferred via side effects.
2. Target village material state is not represented as target-stream facts.
3. Projector executes domain logic.

### Target flow

1. Scheduled action dispatches attack resolution workflow.
2. Workflow loads required village aggregate/read states.
3. Workflow runs battle algorithm once.
4. Workflow appends explicit outcome facts atomically.
5. Projectors only materialize those facts.
6. No `ConquerVillage` command.

### Event Contract (proposed)

1. `BattleResolved`
   1. Scope: attack movement lifecycle outcome.
   2. Contains:
      1. `action_id: Uuid`
      2. `movement_id: Uuid`
      3. `source_village_id: u32`
      4. `target_village_id: u32`
      5. `attacker_player_id: Uuid`
      6. `defender_player_id: Uuid`
      7. `attack_type: AttackType`
      8. `report: BattleReportSnapshot`
      9. `returns_at: DateTime<Utc>`

2. `TargetVillageStateApplied`
   1. Scope: authoritative target-side post-battle state.
   2. Contains:
      1. `action_id: Uuid`
      2. `target_village_id: u32`
      3. `player_id_after: Uuid`
      4. `loyalty_after: u8`
      5. `buildings_after: Vec<VillageBuilding>`
      6. `stocks_after: ResourceGroup`
      7. `home_army_after: Option<Army>`
      8. `reinforcements_after: Vec<Army>`

3. `AttackReturnScheduled`
   1. Scope: delayed return trip fact.
   2. Contains:
      1. `action_id: Uuid`
      2. `movement_id: Uuid`
      3. `army_id: Uuid`
      4. `source_village_id: u32`
      5. `target_village_id: u32`
      6. `player_id: Uuid`
      7. `returning_army: Army`
      8. `bounty: Option<ResourceGroup>`
      9. `returns_at: DateTime<Utc>`

4. `VillageConquestResolved` (optional, emitted only when conquer conditions are met)
   1. Scope: ownership transition caused by battle.
   2. Contains:
      1. `action_id: Uuid`
      2. `target_village_id: u32`
      3. `new_owner_player_id: Uuid`
      4. `owner_village_id: u32`

### Conquest rule

Conquest is not a command. It is a battle outcome.

1. Attack resolution workflow decides conquest from battle result and prerequisites.
2. If true:
   1. emit `VillageConquestResolved` in the same append transaction as other battle outcomes.
3. If false:
   1. no conquest event emitted.

### Projection responsibilities after migration

1. `VillageProjector`
   1. `BattleResolved`: update movement timeline/read-side combat metadata.
   2. `TargetVillageStateApplied`: write target village material state exactly as provided.
   3. `AttackReturnScheduled`: project movement and, only in live mode, enqueue operational delayed command row.
   4. `VillageConquestResolved`: project ownership/map occupancy updates.
2. `ReportProjector`
   1. Builds reports from `BattleResolved` snapshot only (no battle recalculation).

### Migration steps (attack vertical slice)

1. Add new event variants and serialization coverage.
2. Introduce attack resolution workflow service:
   1. input: existing attack arrival payload.
   2. output: ordered outcome event list.
3. Update scheduler attack branch:
   1. stop calling `ConquerVillage`.
   2. call attack resolution workflow once.
4. Projector update:
   1. remove battle calculation from `AttackArrived` branch.
   2. implement handlers for new outcome events.
5. Replay tests:
   1. repeat replay determinism for battle window.
   2. verify conquest appears only when explicit conquest event exists.
6. Compatibility cleanup:
   1. deprecate and later remove legacy `ConquerVillage` command path.

### Acceptance criteria for M2

1. No battle algorithm execution in `VillageProjector`.
2. No `ConquerVillage` command invocation in scheduler attack flow.
3. Target-side post-battle state is materialized from explicit events.
4. Replay of attack windows is deterministic and does not depend on live read-model lookups for battle logic.

## Milestone Gates

### M1 - Safety Baseline
- Scheduler cannot strand claimed actions.
- Replay does not mutate operational queue.
- Runtime/replay projector order aligned.

M1 status:
- [x] Scheduler cannot strand claimed actions.
- [x] Replay does not mutate operational queue.
- [x] Runtime/replay projector order aligned.

### M2 - Attack Flow Correctness
- Attack outcomes represented by explicit facts.
- Target village mutations occur via target stream events.
- Projector no longer runs battle domain logic.

M2 status (in progress):
- [x] Added explicit attack outcome fact (`AttackBattleResolved`).
- [x] Scheduler now resolves battle once and records explicit outcome.
- [x] `ConquerVillage` command removed from scheduler path; conquest decided by battle outcome.
- [x] `VillageProjector` no longer computes attack battle logic in `AttackArrived`.
- [x] `ReportProjector` now reads attack outcomes from explicit facts (no attack battle recomputation).
- [x] Replay report classification switched from `AttackArrived` to `AttackBattleResolved`.
- [x] Remove/retire legacy `ConquerVillage` command and event path from app layer.
- [x] Introduce target-stream authored battle outcome events (multi-stream append) instead of source-stream-only outcome.
- [x] Add deterministic replay assertions for attack windows at broader suite level.

### M3 - Full Transactional Integrity
- Command path event append + projection + queue write is atomic.
- Rollback tests pass for injected failures.

M3 status (in progress):
- [x] Introduced transactional multi-stream append primitive (`append_workflow_events`) in event store.
- [x] Switched attack battle workflow to append source+target facts in one DB transaction.
- [x] Added service-level workflow helper that projects stored events in global-sequence order.
- [x] Switched merchants arrival workflow to append source+target facts in one DB transaction.
- [x] Switched marketplace offer acceptance to emit stream-owned merchant trip facts through workflow append.
- [x] Switched marketplace create/cancel reservation effects to explicit application facts.
- [x] Narrowed `AcceptMarketplaceOffer` command to acceptance fact only; cross-stream trip facts are service-orchestrated.
- [x] Add conflict/rollback-focused tests for workflow append boundary.

### M4 - Full Workflow Coverage
- Reinforcement/merchant/settler/conquest flows migrated.
- Replay deterministic across all migrated flows.

## Risks and Mitigations

1. Risk: migration complexity on large event enum.
   Mitigation: migrate by vertical slice (attack first), keep compatibility events temporarily.
2. Risk: performance regression from broader transactions.
   Mitigation: measure lock duration and optimize query count inside tx.
3. Risk: backfill/replay duration grows.
   Mitigation: optimize projector batch writes and add replay window controls.
4. Risk: API behavior drift.
   Mitigation: contract tests against `docs/api-contract-matrix.md` expectations.

## Deliverables Checklist

- [x] ADR documenting transactional CQRS boundary and scheduler separation
- [ ] Schema migration(s) for operational queue split
- [ ] Refactored projector modules (pure reducers)
- [ ] Workflow/process-manager layer for cross-village commands
- [ ] Replay invariants test suite
- [ ] Scheduler resilience test suite
- [ ] Updated architecture docs

References:
- ADR: [`docs/ADR_SCHEDULED_WORKFLOWS_MULTI_STREAM_FACTS.md`](docs/ADR_SCHEDULED_WORKFLOWS_MULTI_STREAM_FACTS.md)
- mini_cqrs_es proposal: [`docs/MINI_CQRS_ES_WORKFLOW_APPEND_PROPOSAL.md`](docs/MINI_CQRS_ES_WORKFLOW_APPEND_PROPOSAL.md)

## Decisions (Resolved 2026-05-17)

1. Multi-stream append conflicts:
   1. Chosen: fail-fast.
   2. Rationale: conflicts indicate stale command context and should surface immediately.
2. Delayed command reliability:
   1. Chosen: exactly-once best effort.
   2. Implementation note: still add idempotency guards on completion commands as a safety net (crash/retry boundaries can otherwise duplicate effects).
3. Passive resource derivation responses:
   1. Chosen: return derived values only.
   2. Policy: persist stock snapshot at write boundaries; derive current resources from snapshot + elapsed time on reads.
4. Event versioning/upcasters:
   1. Chosen: defer for now.
   2. Trigger point: revisit after M2 or when event compatibility across releases becomes a requirement.

## Progress Log

2026-05-17:
1. Plan created.
2. Architectural decisions recorded:
   1. Conflict policy: fail-fast.
   2. Scheduler reliability target: exactly-once best effort (+ idempotency guards).
   3. Resource query policy: derived values only.
   4. Event evolution: upcasters/versioning deferred.
3. M1 implementation completed:
   1. Replay no longer deletes or rebuilds operational scheduled actions.
   2. Replay projector order aligned with runtime (`ReportProjector` then `VillageProjector`).
   3. Scheduler loop no longer aborts remaining claimed actions after first failure.
   4. Regression tests added for replay queue preservation and no stranded `processing` actions.
4. M2 partial implementation completed:
   1. Introduced explicit attack outcome recording command/event.
   2. Battle resolution moved out of `VillageProjector` attack-arrival branch into scheduler workflow.
   3. Conquest now decided by battle outcome workflow, not by explicit scheduler-issued conquer command.
   4. Report projection for attack now consumes outcome facts.
   5. Replay report-event classification updated to outcome facts.
