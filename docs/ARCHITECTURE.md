# Parabellum Architecture

Parabellum is a multiplayer strategy game backend organized as a layered CQRS/ES system with strict consistency for game actions.

This document is prescriptive. When adding a feature, use it to decide where
logic belongs before adding new abstractions or persistence queries.

## Crates

- `parabellum_types`
  - Shared types and error enums.
  - No infrastructure dependencies.

- `parabellum_game`
  - Core game domain models and rules (`Village`, `Army`, `Hero`, `Battle`, map models, buildings, smithy, etc.).
  - Pure domain behavior.

- `parabellum_app`
  - Application layer.
  - Defines use-case ports (`ports/*`) and orchestrator (`GameApplication`).
  - Contains village aggregate commands/events/state for CQRS/ES.
  - Must not depend on SQLx or persistence details.

- `parabellum_infra`
  - Infrastructure adapters and persistence.
  - Implements app ports using Postgres/SQLx.
  - Hosts CQRS runtime wiring, event store, projectors, scheduler worker, and read-model repositories.

- `parabellum_web`
  - HTTP API and session/auth token handling.
  - Calls `GameApplication` only.

- `parabellum_server`
  - Runtime composition and startup.
  - Wires `GameApplication` with DB adapters, starts HTTP server and scheduler.

## Responsibility Map

| Concern | Owner | Rule of thumb |
| --- | --- | --- |
| Shared ids, value types, error enums | `parabellum_types` | Keep dependency-free and behavior-light. |
| Pure game mechanics | `parabellum_game` | If the rule can be evaluated from domain state alone, put it here. |
| Command intent, aggregate state, app policies | `parabellum_app` | Use when a rule combines domain mechanics with ownership, pending actions, read-model commitments, or workflow choices. |
| SQL, transactions, event store, read models | `parabellum_infra` | Implement ports and persistence details only; do not redefine game rules here. |
| HTTP/session/API mapping | `parabellum_web` | Validate transport payloads and call `GameApplication`; do not reach into infra. |
| Runtime composition | `parabellum_server` | Wire concrete adapters and start processes. |

Placement checklist:
1. Can this be decided from a `Village`, `Army`, `Hero`, battle result, or
   static game data without persistence? Put it in `parabellum_game`.
2. Does it combine domain mechanics with command intent, ownership, queued
   actions, read-model commitments, or workflow-time choices? Put it in
   `parabellum_app::villages::policies` or aggregate command/state code.
3. Is it a SQL-backed set/count/existence/snapshot question? Put it behind an
   app repository/read-model port and implement it in `parabellum_infra`.
4. Is it converting a scheduled operational payload into canonical facts? Put
   it in `parabellum_infra/es/workflows`.
5. Is it materializing facts into read models or operational queues? Put it in a
   projector module and keep writes in repositories/transaction helpers.
6. If a new abstraction only forwards to one domain method, delete the
   abstraction and call the domain model directly.

## Runtime Flow

1. API endpoint receives request (`parabellum_web/api/*`).
2. Endpoint validates payload/auth context and calls `GameApplication`.
3. `GameApplication` delegates to command/query/scheduler ports.
4. `parabellum_infra` adapter executes CQRS command:
   - append events
   - project read models
   - commit atomically (strict consistency)
5. API returns typed response or mapped error envelope.

## CQRS/ES Boundaries

- Aggregate granularity: one village aggregate per village id (`u32`).
- The live aggregate runtime uses `SnapshotAggregateManager` with
  `es_snapshots` for aggregate loading. Normal commands save snapshots through
  the CQRS runtime after events are appended and projected.
- Scheduled workflow facts are appended outside `SimpleCqrs::execute`, so the
  workflow append boundary refreshes snapshots for every affected aggregate
  stream after the workflow events are committed.
- Projectors run synchronously in the command/workflow transaction. There is no
  projector-offset table because offsets are only needed for asynchronous
  catch-up consumers.
- Scheduling:
  - validations at scheduling time
  - deterministic canonical fact production
  - scheduler reads due actions from `rm_scheduled_actions`
  - scheduler does not mutate state directly; it appends canonical workflow facts only.
- Read models are query sources (`rm_village`, `rm_armies`, `rm_village_movements`, `rm_reports`, `rm_map_fields`, etc.).

## Scheduler Operational Contract

`rm_scheduled_actions` is an operational queue (not canonical domain history).
Scheduled payloads use a strict variant shape with workflow data under
`workflow`; old top-level payload compatibility is intentionally not preserved:

```json
{
  "type": "Building",
  "workflow": {
    "village_id": 123
  }
}
```

Execution model:
1. scheduler claims due `pending` actions into `processing`,
2. executes deterministic workflow fact production,
3. terminally marks each action as `completed` or `failed`.

Recovery model:
1. at tick start, stale `processing` rows (older than recovery threshold) are requeued to `pending`,
2. batch failures do not leave actions indefinitely in `processing`.

Replay model:
1. replay rebuilds read models from event facts only,
2. replay does not recreate or mutate operational queue rows (`rm_scheduled_actions`).

Workflow module responsibilities:
- `parabellum_app` owns scheduled workflow payload contracts and aggregate fact
  shapes.
- `parabellum_game` owns game rules, combat math, building data, and domain
  state transformations.
- `parabellum_infra/es/workflows/*` owns scheduler orchestration only:
  repository-backed read-model lookups, conversion from operational payloads to
  app command outcomes, and grouping facts for append.
- `parabellum_infra/es/repositories/*` owns read-model and operational queue
  SQL. Scheduler/replay services may coordinate process-level execution through
  infrastructure helpers such as advisory locks, but should not inline
  read-model or queue mutation SQL.

Command outcome pattern:
- Immediate commands validate against the loaded aggregate/read-model context
  and emit canonical `VillageEvent` facts directly.
- Scheduled commands validate at scheduling time and emit facts that reserve the
  required state plus a typed workflow payload for `rm_scheduled_actions`.
- Workflow handlers convert due operational payloads into canonical command
  outcomes. They may load read models and call application commands, but they do
  not own domain mechanics.
- Commands that exist only to materialize workflow outcomes expose conversion as
  `into_outcome_event`; this names the behavior as producing a canonical fact
  from a command outcome, not as a generic event helper.

Current workflow modules:
- `buildings`, `training`, `research`: deterministic completion facts.
- `heroes`: hero revival validation and completion facts.
- `movements`: army/reinforcement/scout/attack movement arrival and return
  facts.
- `foundation`: settlers arrival, map-field claimability checks, default
  founded-village building creation, and settlers return scheduling when the
  target exists but is no longer available.
- `battles`: attack/scout orchestration around `parabellum_game::battle`.
- `merchants`: merchant arrival/return and marketplace acceptance workflow
  facts.

Adding a new command or workflow:
1. Define the command intent and canonical facts in `parabellum_app`.
2. Keep pure validation/mechanics in `parabellum_game`; add an app policy only
   when the command needs application context.
3. For immediate commands, emit canonical facts directly from the aggregate.
4. For delayed commands, emit a scheduling fact with all reserved/canonical
   state required by the future workflow.
5. Projectors turn scheduling facts into `rm_scheduled_actions`; workflow
   modules later turn due payloads into canonical outcome facts.
6. Add repository methods for read-model questions instead of mixing SQL and
   Rust filtering in services or workflows.

## Workflow Transaction Boundary

For cross-village facts (multi-stream workflows), infrastructure uses a dedicated
transactional append boundary:

1. collect workflow domain events grouped by target aggregate stream,
2. load each stream expected version,
3. append all grouped streams in one DB transaction (`es_events`),
4. project resulting stored events in `global_seq` order.

Current usage:
- attack battle resolution appends:
  - `AttackBattleResolved` on source village stream
  - `BattleOutcomeAppliedToVillage` on target village stream
- merchants arrival appends:
  - `MerchantsArrived` on source village stream
  - `MerchantTransferAppliedToVillage` on target village stream
- marketplace offer acceptance appends:
  - `MarketplaceOfferAcceptanceAppliedToVillage` on accepting village stream
  - `MarketplaceOfferAccepted` on accepting village stream
  - owner `MerchantsTripScheduled` on owner village stream
  - accepting `MerchantsTripScheduled` on accepting village stream
- marketplace create/cancel reservation effects:
  - `MarketplaceOfferReservationAppliedToVillage` carries owner stocks/merchant reservation state
  - `MarketplaceOfferReservationReleasedFromVillage` carries owner refund/merchant release state

Failure semantics:
- fail fast on any stream conflict (`CqrsError::Conflict`)
- no partial append across streams
- projector processing runs only after successful append
- live command runtime executes projector updates in the same SQL transaction boundary (`*_in_tx` path only)

## Projector Runtime Mode

`VillageProjector` and `ReportProjector` run through tx wrappers in live command handling:
1. begin SQL transaction
2. call `process_in_tx`
3. commit transaction

Legacy non-transactional projector execution path has been removed from the live runtime.

Projector module pattern:
- Parent projector files are dispatchers and shared infrastructure helpers.
- Feature modules own one read-model concern such as armies, battle, merchants,
  training, reports, or lifecycle.
- Projection methods should load required context, call small private helpers to
  shape the next read-model state, and keep persistence writes inside
  infrastructure transaction methods.
- Helpers should prefer taking `&VillageEvent` and destructuring the expected
  variant internally over accepting long argument lists. The event variant is
  the projection input contract.
- Pure shaping helpers stay module-local unless another projector/API path
  actually needs the same behavior.

Report projection:
- Reports are read-side notifications. They do not affect domain state, command
  validity, aggregate invariants, or replay facts.
- `parabellum_game` produces canonical outcomes such as battle reports,
  resource movements, and army survivors.
- `parabellum_infra` turns canonical events plus read-model context into
  `rm_reports` rows and audience rows.
- Missing player or village context is not expected during normal operation. A
  missing player is an identity/data error. A missing village is tolerated only
  as projector/replay resilience for non-authoritative notifications.

## Read-Model Ownership Contract

To avoid drift, each gameplay concern has exactly one canonical read-model owner:

- `rm_armies`:
  - canonical source for troop state (`home`, `stationed`, `moving`)
  - canonical source for UI troop availability and army cards
  - canonical source for troop crop upkeep by current location
  - stored as queryable columns (`army_id`, home/current village ids,
    `player_id`, `tribe`, `state`, `units`, `smithy_upgrades`, optional
    `hero_id`), not as an opaque domain payload
- `rm_village_movements`:
  - canonical source for movement timeline (outgoing/incoming/return)
- `rm_village`:
  - canonical source for village economy/buildings/production/research
  - canonical source for village CPP/day (`culture_points_production`)
  - village cumulative CP is not authoritative
  - does not contain army snapshots; troop state must be loaded from
    `rm_armies`

- `players`:
  - canonical source for cumulative culture points (`culture_points`)
  - `culture_points` advances from elapsed time using summed village CPP/day

Command-side rule:
- `VillageAggregate` remains fully aware of army data for invariants and domain behavior.
- Projectors materialize aggregate army facts into `rm_armies`. When a village
  read model must be hydrated for production or command context, current army
  context is loaded from `rm_armies`.
- `VillageModel` is the village economy/read-model shape. Converting it with
  `From<VillageModel>` creates an economy-only domain village. Code that needs
  troop-aware domain methods must load `VillageArmyContext` from `rm_armies`
  and call `hydrate_village(model, context)`.
- Report projectors follow the same read-model rule: reports are
  notifications, but scout/battle report payloads still load troop rows from
  `rm_armies` when visibility requires them.

Domain mechanics and application policies:
- Pure game mechanics live on `parabellum_game` domain models. Examples:
  `Army::split_units`, `Army::has_units`, `Village::required_merchants`,
  `Village::validate_merchant_transfer`, `Village::reserve_merchant_transfer`,
  and expansion slot math on `Village`.
- `parabellum_app::villages::policies` exists only when a rule combines domain
  mechanics with application context such as ownership, command intent, pending
  aggregate actions, read-model commitments, or workflow choices.
- Current app policies:
  - `ExpansionSlotUsage`: chief/settler slot availability across domain slots,
    queued training, founded child villages, and moving expansion units.
  - `ConquestAttempt`: battle-time conquest eligibility. Infrastructure loads
    current source/target/read-model context, but capital protection, chief
    presence, source slots, and culture point decisions stay in the policy.
  - `ArmyDispatch`: outbound dispatch across ownership, target, rally point,
    scout-only command intent, and domain army splitting.
  - `ReinforcementControl`: recall/release control over stationed
    reinforcements plus domain army splitting.
  - `MarketplaceOfferCreation`: marketplace offer terms such as non-zero sides,
    different resources, and exchange-ratio limits.
  - `MarketplaceAcceptance`: acceptance ownership/self-target checks. It does
    not revalidate offer terms; accepted offers are expected to have been
    created through `MarketplaceOfferCreation`.
- Merchant capacity, resource reservation, and resource refunds remain domain
  `Village` mechanics.
- Building schedule preparation follows the same rule: pending queue timing and
  queue capacity live in `VillageState`, while validated mechanics for adding,
  upgrading, downgrading, costs, and durations live on domain `Village` init
  helpers.
- Do not keep app policies that only delegate to a domain method. Call the
  domain model directly from aggregate state or command handling instead.
- Infrastructure may load policy inputs from read models, but it must not
  duplicate domain mechanics or app-policy counting rules in SQL/Rust mixtures.
- Workflow modules own conversion into operational `ScheduledAction` payloads.
  For mechanical scheduling facts, expose `*_from_event` helpers. When a
  projector must first assemble read-model context (for example battle or
  reinforcement returns), build a typed workflow object and pass that to the
  workflow module for scheduling.
- Projectors persist scheduled rows and apply immediate read-model effects such
  as resource deductions, busy-merchants updates, movement rows, and moving
  army rows.
- Projectors write army effects only through `rm_armies`; they must not keep
  duplicate troop snapshots in `rm_village`.
- Projectors apply canonical fact values directly when the fact carries the
  target state (for example projected stocks or busy merchants). When a
  projector must derive state from an operational event, it should instantiate
  the domain model and call domain helpers instead of reimplementing resource,
  merchant, or army arithmetic.
- `parabellum_infra::es::consumers::village_projector::*` modules apply this
  split per read-model concern: schedule operational actions from scheduling
  facts, apply fact-carried state directly, use domain helpers only for derived
  read-model state, and leave persistence writes and transaction boundaries in
  infrastructure.

Projector rules:
- **Scheduled actions**: projectors call workflow helpers and then persist the
  returned operational action. Examples: building/training/research use
  `scheduled_action_from_event`, merchant trips use `scheduled_trip_from_event`,
  and battle/reinforcement returns build an `ArmyReturnWorkflow` before
  scheduling.
- **Fact-carried state**: projectors write the state carried by the canonical
  fact without recalculating it. Examples: battle target state, marketplace
  offer status changes, marketplace stock snapshots, and marketplace
  busy-merchant snapshots.
- **Derived state**: projectors may instantiate `Village`, `Army`, or other
  domain models only when a read model must be derived from a fact. Examples:
  merchant transfer departure/return effects, foundation resource withdrawal,
  research completion, trained-unit home-army updates, and returning-army
  merges.
- **Infrastructure only**: projectors own persistence calls, movement rows,
  scheduled queue rows, and transaction boundaries. They must not encode domain
  mechanics or app-policy counting rules.

## Game World Data

- `rm_map_fields` is the canonical world map table.
- Each map field id is deterministic (`position.to_id(world_size)`), so `map_field_id`, `village_id`, and `oasis_id` remain aligned when applicable.
- Field occupancy is updated by domain actions (for example, village foundation on settlers arrival).
- Foundation lookup by id must find an `rm_map_fields` row. A missing row is a
  data/infrastructure error, not a default valley fallback.
- Existing map rows are claimable for foundation only when they are unoccupied
  valley fields owned by nobody or by the founding player.

## Key Design Rules

- `parabellum_app` and domain crates do not depend on SQLx.
- Infrastructure-specific mapping stays in `parabellum_infra`.
- Domain/game rules live in `parabellum_game` whenever possible.
- ES runtime path is the only command execution path.
- Query services and workflow modules should use repository/read-model methods
  for SQL-backed set, count, existence, and snapshot questions. Prefer named
  snapshot structs for multi-value read-model answers. Direct SQL in services or
  workflows is a smell unless no repository boundary exists yet.
- Runtime services should keep process coordination separate from persistence
  details: repositories own read-model/queue SQL, while small infrastructure
  helpers own database primitives such as advisory locks.
- Prefer SQL/read-model queries that answer the precise question directly.
  Rust filtering is reserved for domain behavior after a narrow read, not for
  broad read-model filtering that the database can express.

## API Surface

- HTTP API is served under `/api/v1`.
- Workflow fact contracts are documented in [`docs/EVENT_CONTRACTS.md`](docs/EVENT_CONTRACTS.md).
- Machine-readable contract entrypoint: `GET /api/v1/openapi.json`.
