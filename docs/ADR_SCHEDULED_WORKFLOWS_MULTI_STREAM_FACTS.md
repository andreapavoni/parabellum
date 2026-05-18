# ADR: Scheduled Workflows and Multi-Stream Facts

Status: Accepted  
Date: 2026-05-17  
Owners: Parabellum backend

## Context

Parabellum has delayed/scheduled actions that often impact multiple aggregates and multiple players:

- attack/battle flows affect source village, target village, reinforcements, heroes, movements, reports
- merchants/marketplace flows affect source village stocks, target village stocks, merchants availability, reports
- some outcomes have domain milestones that matter independently (`VillageConquered`)

The domain logic is already centralized in `parabellum_game` (battle resolution, village/army/building rules, upkeep side effects).  
Replay determinism and strict consistency are primary goals.

## Decision

### 1) Scheduled actions are workflow triggers

A scheduled action execution is treated as workflow progression, not as "run one aggregate command later".

Workflow execution:

1. load needed domain state
2. resolve domain outcome once (in application/service layer, using domain models)
3. emit canonical workflow fact(s)
4. append facts across involved streams in one DB transaction
5. project read models from appended facts

### 2) Fact strategy: one canonical fact per resolved domain decision

For complex outcomes (example: battle), emit one canonical "big fact" that contains authoritative outcome data needed by projections and replay.

Secondary facts are allowed only when they have distinct semantic value/lifecycle:

- keep `VillageConquered` as explicit domain fact/milestone
- avoid splitting into many technical micro-facts unless they are independently consumed

### 3) Multi-stream append is mandatory for cross-aggregate workflow facts

If one workflow step writes facts for multiple aggregates, all those stream appends must be atomic:

- fail fast on expected-version conflict
- no partial cross-stream append
- consumer/projection processing runs only after successful append

### 4) Reports are projector side effects

Reports are communication read-model artifacts, not canonical domain events.

- canonical workflow facts remain the source of truth
- report projector derives user-facing reports from canonical facts

### 5) Production/upkeep policy

After workflow outcomes:

- persist recomputed production/stocks in read model immediately from outcome payload
- still apply lazy recompute on read for elapsed-time production since last write

This keeps UI responses correct both immediately and after long idle periods.

## Consequences

### Positive

- replay determinism improves because domain decision is computed once and replay re-applies facts
- lower risk of divergence between runtime and replay
- clearer boundary between domain facts and communication projections
- easier reasoning about consistency in cross-village workflows

### Tradeoffs

- canonical events become richer payloads
- projector logic remains non-trivial (many read models touched)
- requires discipline to avoid reintroducing domain recomputation in projectors

## Naming and Modeling Rules

1. Names must be domain facts (`AttackBattleResolved`, `BattleOutcomeAppliedToVillage`, `MerchantsArrived`), never technical process names.
2. Canonical facts may be large when they represent one resolved domain decision.
3. Distinct milestone facts (`VillageConquered`) are explicit when they drive independent behavior.
4. Projectors must not run battle/marketplace decision logic; they only apply facts.

## Scope of this ADR

Applies to:

- attacks/battles
- merchants/resource transfers
- marketplace accept/cancel/settlement
- reinforcements and other delayed cross-village actions

## Implementation Notes (Current State)

- transactional multi-stream append primitive exists in infra event store (`append_workflow_events`)
- attack workflow uses atomic source+target append
- canonical attack facts are projected to read models and reports

## Follow-up Work

1. migrate merchants/marketplace to same workflow boundary
2. define explicit canonical merchant workflow fact shape
3. add conflict/rollback tests for workflow append boundary
4. remove legacy single-stream assumptions where still present

## Applied Increment (2026-05-17)

1. Merchant arrival completion command path was retired in favor of scheduler-driven multi-stream facts (`MerchantsArrived` + `MerchantTransferAppliedToVillage`).
2. Marketplace acceptance now emits an explicit state-application fact:
   1. `MarketplaceOfferAcceptanceAppliedToVillage`
   2. projector applies stock + busy-merchant materialization from that payload
3. Workflow append preparation/building was extracted behind a dedicated service helper seam for future `mini_cqrs_es` extraction.

## Implemented in Code

1. Workflow append transaction primitive:
   - `parabellum_infra/es/stores.rs` (`PostgresEventStore::append_workflow_events`)
2. Village workflow orchestration seam:
   - `parabellum_infra/es/village_service/mod.rs`
   - `build_village_workflow_appends`
   - `append_village_workflow_events`
3. Canonical marketplace acceptance application fact:
   - `parabellum_app/villages/events.rs` (`MarketplaceOfferAcceptanceAppliedToVillage`)
   - projector application in `parabellum_infra/es/consumers/village_projector.rs`
