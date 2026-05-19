# CQRS/ES Architecture Status

Last updated: 2026-05-19

This is the current implementation snapshot of the CQRS/ES migration.

## Completed

1. Transactional workflow append:
   1. multi-stream events append atomically
   2. fail-fast conflict behavior
   3. projector application in same SQL transaction
2. Live projector runtime is tx-only:
   1. `VillageProjector` uses `process_in_tx`
   2. `ReportProjector` uses `process_in_tx`
   3. runtime uses tx-only projector execution path
3. Cross-stream canonical workflow facts:
   1. attack: `AttackBattleResolved` + `BattleOutcomeAppliedToVillage`
   2. reinforcement: `ReinforcementArrived` + `ReinforcementAppliedToVillage`
   3. settlers: `SettlersArrived` + `VillageFounded`
   4. merchants: `MerchantsArrived` + `MerchantTransferAppliedToVillage`
   5. marketplace create/cancel/accept materialization facts
4. Replay safety and determinism coverage:
   1. replay does not pollute `rm_scheduled_actions`
   2. deterministic replay checks for attack and marketplace windows
5. Scheduler recovery behavior:
   1. stale `processing` actions are requeued to `pending`
   2. batch failures do not leave actions stuck in `processing`
6. Village read paths:
   1. village repository read refresh no longer writes back to `rm_village`
   2. derived refresh on queries is in-memory only
7. Scheduler delayed-workflow progression uses canonical facts (no completion-command dispatch in runtime path).
8. Reinforcement scheduled-action IDs are deterministic (`movement_id`) and no longer generated inside projector.

## Remaining Architecture Focus

1. Keep scheduler branches thin and route deterministic shaping through workflow fact builders.
2. Keep source/target stream ownership explicit in event contracts for every cross-village workflow.
3. Keep replay strictly projection-only and operational queue repair strictly maintenance-only.

## Operational Notes

1. `rm_scheduled_actions` is operational queue state, not canonical domain history.
2. Canonical truth is `es_events`; read models and reports are projections.
3. Replay is a projection rebuild operation, not a scheduler rebuild operation.
