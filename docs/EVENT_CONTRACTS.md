# Event Contracts

Last updated: 2026-05-19

This document tracks canonical workflow facts, their producers, and projector responsibilities.

Global invariant:
1. Scheduler progression is fact-only: due actions emit canonical workflow facts.

## Attack Workflow

Canonical facts:
1. `AttackSent` (source stream movement materialization)
2. `AttackArrivalScheduled` (source stream scheduling intent fact)
3. `AttackBattleResolved` (source village stream)
4. `BattleOutcomeAppliedToVillage` (target village stream)

Producer:
1. `parabellum_infra/es/village_service/scheduler.rs` via attack scheduled-action workflow

Projection responsibilities:
1. `VillageProjector` materializes village/army/movement state.
2. `ReportProjector` emits user-visible battle reports.
3. `VillageProjector` synchronizes reinforcement owners' deployed-army snapshots from post-battle stationed armies.
4. `ReportProjector` includes reinforcement owners in battle-report audiences.

Invariants:
1. Source and target facts are appended atomically in one workflow append.
2. Projectors do not recompute battle decisions.
3. Attack arrival scheduling is projected from explicit `AttackArrivalScheduled` fact, not inferred from `AttackSent`.

## Reinforcement Workflow

Canonical facts:
1. `ReinforcementSent` (source stream when dispatching reinforcement)
2. `ReinforcementArrived` (source stream arrival milestone/materialization for source-side views)
3. `ReinforcementAppliedToVillage` (target stream materialization fact)

Producer:
1. Scheduler workflow in `parabellum_infra/es/village_service/scheduler.rs`

Projection responsibilities:
1. `VillageProjector` applies source-side deployed-army and movement cleanup from `ReinforcementArrived`.
2. `VillageProjector` applies target-side stationed/home-hero materialization from `ReinforcementAppliedToVillage`.

Invariants:
1. `ReinforcementArrived` and `ReinforcementAppliedToVillage` are appended atomically.
2. Hero-only transfer decision is computed in scheduler workflow and carried in facts (`hero_alone_transfer`), not inferred in projector.

## Scout Workflow

Canonical facts:
1. `ScoutArrived` (arrival milestone)
2. `ScoutBattleResolved` (source village stream)

Producer:
1. `parabellum_infra/es/village_service/scheduler.rs` scout scheduled-action workflow

Projection responsibilities:
1. `VillageProjector` applies arrival/removal and return scheduling from fact payload.
2. `ReportProjector` builds scouting battle reports from `ScoutBattleResolved`.

Invariants:
1. Projectors do not execute scout battle simulation.
2. Scout return movement/scheduling derives from canonical resolved fact payload.

## Merchants Transfer Workflow

Canonical facts:
1. `MerchantsTripScheduled` (source stream when dispatching transfer)
2. `MerchantsArrived` (source stream on arrival milestone)
3. `MerchantTransferAppliedToVillage` (target stream materialization fact)
4. `MerchantsReturned` (source stream on return milestone)

Producer:
1. Scheduler workflow in `parabellum_infra/es/village_service/scheduler.rs`

Projection responsibilities:
1. `VillageProjector` schedules due actions from trip facts.
2. `VillageProjector` applies target stock snapshot from `MerchantTransferAppliedToVillage`.

Invariants:
1. Arrival milestone and target materialization are appended atomically.
2. Merchant arrival materialization is fact-driven (`MerchantsArrived` + `MerchantTransferAppliedToVillage`).

## Marketplace Workflow

Canonical facts:
1. Create:
   1. `MarketplaceOfferCreated`
   2. `MarketplaceOfferReservationAppliedToVillage`
2. Cancel:
   1. `MarketplaceOfferCanceled`
   2. `MarketplaceOfferReservationReleasedFromVillage`
3. Accept:
   1. `MarketplaceOfferAcceptanceAppliedToVillage`
   2. `MarketplaceOfferAccepted`
   3. owner `MerchantsTripScheduled`
   4. accepting `MerchantsTripScheduled`

Producer:
1. `parabellum_infra/es/village_service/mod.rs` (`create_marketplace_offer`, `cancel_marketplace_offer`, `accept_marketplace_offer`)

Projection responsibilities:
1. `VillageProjector` applies stocks/busy-merchants snapshots from explicit application facts.
2. `VillageProjector` stores/schedules merchant trips from `MerchantsTripScheduled`.
3. `MarketplaceRepository` status transitions are driven by `MarketplaceOffer*` facts only.
4. Cancel flow stock restoration must come from `MarketplaceOfferReservationReleasedFromVillage.owner_stocks` (never inferred).

Invariants:
1. Acceptance fact set is appended atomically across involved streams.
2. Accepting-village departure reservation is materialized by `MarketplaceOfferAcceptanceAppliedToVillage` and not inferred from offer status alone.
3. Cancel flow must restore both owner stocks and busy merchants from canonical release fact payload.

## Settlers Workflow

Canonical facts:
1. `SettlersSent` (source stream when dispatching settlers)
2. `SettlersArrived` (source stream on arrival milestone)
3. `VillageFounded` (target stream for the newly founded village)

Producer:
1. Scheduler workflow in `parabellum_infra/es/village_service/scheduler.rs`

Projection responsibilities:
1. `VillageProjector` stores/schedules settlers movement from `SettlersSent`.
2. `VillageProjector` removes moving settlers army on `SettlersArrived`.
3. `VillageProjector` materializes the newly founded village and map occupancy from `VillageFounded`.

Invariants:
1. `SettlersArrived` and `VillageFounded` are appended atomically in one workflow append.
2. Map occupancy assignment is derived from canonical `VillageFounded` projection, not direct scheduler-side map mutation.
