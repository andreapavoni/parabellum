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
  - Defines use-case services, app ports, command intent, aggregate facts, and
    orchestration contracts.
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
3. `GameApplication` delegates to application use-case services.
4. Application use cases load context through app ports, call
   `parabellum_game` mechanics or app policies, and build canonical command
   intent.
5. `parabellum_infra` implements the app ports and executes CQRS command
   intent:
   - append events
   - project read models
   - commit atomically (strict consistency)
6. API returns typed response or mapped error envelope.

## Application Layer Pattern

`parabellum_app` is the use-case orchestration layer. It establishes the
interfaces that infrastructure implements, but it does not know whether those
interfaces are backed by CQRS/ES, SQL, snapshots, or another runtime.

Application code should follow this shape:

```text
parabellum_app/
  application/
    game.rs                  # public facade used by web
  identity/
    requests.rs
    ports.rs
    use_cases.rs
  scheduler/
    requests.rs
    ports.rs
    use_cases.rs
  map/
    requests.rs
    ports.rs
    use_cases.rs
  villages/
    requests/
      buildings.rs           # transport-independent app inputs
      development.rs
      movements.rs
      marketplace.rs
      heroes.rs
    use_cases/
      buildings.rs
      development.rs
      movements.rs
      reinforcements.rs
      marketplace.rs
      heroes.rs
      traps.rs
      expansion.rs
    ports/
      command_executor.rs    # aggregate command execution gateway
      movement_reads.rs      # concern-specific read/context gateways
      marketplace_reads.rs
      village_army_reads.rs
      hero_reads.rs
      village_state_reads.rs
      clock.rs               # time source when a use case needs "now"
      ids.rs                 # id source when a use case creates canonical ids
    policies/
      ...                    # app policies combining domain rules and app context
    read_models/
      activity.rs            # app/API query response shapes
      marketplace.rs
      village_army.rs
    commands/
      ...                    # aggregate-local command intent and outcome commands
```

The exact files may evolve, but the dependency direction must not:

```text
web -> app facade -> app use case -> app ports -> infra implementation
                         |
                         v
                    game domain
```

### Use-Case Services

A use-case service owns application orchestration for one gameplay concern. It
may:

- accept transport-independent app request structs,
- load read-model context through narrow app ports,
- call pure domain behavior in `parabellum_game`,
- call app policies when a rule combines domain mechanics with ownership,
  pending actions, read-model commitments, or workflow choices,
- generate canonical ids and timestamps through app-owned `IdGenerator` and
  `Clock` ports,
- build aggregate command intent or workflow outcome command intent,
- delegate command execution to an app command-executor port.

A use-case service must not:

- call SQLx, Postgres, transaction, advisory-lock, or CQRS runtime APIs
  directly,
- duplicate game calculations that can live on `parabellum_game` domain
  models,
- filter broad read-model sets in Rust when the repository can answer the
  question directly,
- expose infrastructure error types in its public API.

Use cases should be small enough that their inputs document the consistency
boundary. Prefer named context structs such as `MovementSourceContext`,
`ExpansionTrainingContext`, or `MarketplaceAcceptanceContext` over loose tuples
or repeated multi-query call sites.

### App Ports

App ports describe what the application needs, not how infrastructure works.
They should be narrow and concern-oriented:

- read ports answer specific set/count/existence/snapshot questions,
- command ports execute canonical app command intent,
- clock/id ports make time and id creation explicit and testable,
- scheduler/process ports expose operational entrypoints without leaking queue
  table details.

Avoid new god traits. A broad facade is acceptable at composition boundaries
only when it delegates to smaller use-case services or ports internally.

Infrastructure adapters implement app ports. They may translate app requests to
CQRS/ES runtime calls, load Postgres read models, map infrastructure errors into
`ApplicationError`, and manage transaction boundaries. They must not own
gameplay decisions.

### Commands And Outcomes

Aggregate commands in `parabellum_app::villages::commands` should have one
clear role:

- immediate command intent validates aggregate-local invariants and emits
  canonical facts directly,
- scheduled command intent validates scheduling-time invariants and emits facts
  that reserve state plus typed workflow payloads,
- workflow outcome commands materialize deterministic outcome facts that were
  computed by workflow orchestration.

Do not mix these roles implicitly. If a command accepts a precomputed outcome or
plan, name and document it as an outcome/materialization command. If a command
is aggregate-local, it should not require infrastructure-shaped context.

### Domain Delegation

`parabellum_game` is the home for domain models, calculations, and pure rules.
When app code needs a calculation such as travel time, unit speed, merchant
capacity, building cost, battle math, trap capacity, hero resurrection cost, or
resource reservation, prefer adding or using a domain method in
`parabellum_game`.

Before adding a new domain helper, search `parabellum_game` for the existing
model behavior. The domain crate already contains many small helpers, and app
code should reuse or lightly improve them instead of adding parallel
calculations. For example, movement speed should use the army/domain model
behavior (`Army::speed` or a domain helper extracted from it), not an
infrastructure-local copy of the slowest-unit calculation.

When an existing domain helper is close but awkward for app orchestration,
prefer improving the domain API over recreating the rule in `parabellum_app`.
For example, a static/domain helper such as "speed for this tribe, troop set,
and optional hero" is acceptable if it keeps `Army::speed` and app movement
planning on one implementation.

`parabellum_app` may combine those domain answers with application context. For
example:

- "Can this player dispatch this army from this village?"
- "Do queued settlers and existing child villages consume all expansion slots?"
- "Is this movement still inside the cancel window?"
- "Which canonical ids and timestamps should this scheduled workflow use?"

Those are app orchestration/policy questions because they require command
intent, ownership, read models, pending actions, or workflow choices.

### Rust Documentation Standard

New app-layer modules, traits, and public structs must carry rustdoc that
states the layer contract:

- what the item owns,
- what layer implements it when it is a port,
- what it may call,
- what it must not know about.

Keep rustdoc operational and specific. For example, a read port should state
which context it provides and whether the answer is authoritative for command
validation. A use-case service should state that it orchestrates app context and
delegates mechanics to `parabellum_game`.

### Naming And File Conventions

Use names that describe the layer role, not the current implementation:

| Suffix/name | Use for | Example |
| --- | --- | --- |
| `Request` | Transport-independent input accepted by an app use case | `SendAttackRequest` |
| no suffix action name | Aggregate command intent | `SendAttack`, `TrainUnits` |
| `Outcome` | Deterministic workflow/materialization input, preferably named with the domain outcome being applied | `ApplyBattleOutcomeToVillage` |
| `Context` | App-loaded data needed by one use case or policy | `MovementDispatchContext` |
| `Snapshot` | Grouped read-model answer with a consistency boundary | `MovementCancellationSnapshot` |
| `View` | App/API query response shape | `VillageArmyStateView` |
| `*ReadModel` or concern-owned read model | App-facing projected/query shape | `MarketplaceData`, `VillageQueues` |
| `*ReadPort` | App-required read/context capability | `MovementReadPort` |
| `*Executor` | App-required command execution capability | `VillageCommandExecutor` |
| `*UseCases` | Public application service for one concern | `MovementUseCases` |

Use these file locations:

- app input DTOs: `parabellum_app/villages/requests/<concern>.rs`
- app use-case services: `parabellum_app/villages/use_cases/<concern>.rs`
- app ports required by use cases: `parabellum_app/villages/ports/<capability>.rs`
- village app-facing query shapes:
  `parabellum_app/villages/read_models/<concern>.rs`
- village CQRS/projection model contracts:
  `parabellum_app/villages/models/<concern>.rs`
- identity request DTOs: `parabellum_app/identity/requests.rs`
- identity use-case services: `parabellum_app/identity/use_cases.rs`
- identity ports and repositories: `parabellum_app/identity/ports.rs`
- aggregate command intent: `parabellum_app/villages/commands/<action>.rs`
- app policies: `parabellum_app/villages/policies/<concern>.rs`
- CQRS command dispatch helper: `parabellum_app/villages/cqrs_command_service.rs`
- mini-CQRS query handlers: `parabellum_app/villages/cqrs_queries.rs`
- projection repository contracts:
  `parabellum_app/villages/projection_repositories/<concern>.rs`

Prefer concern names over technical names:

- `movements`, `marketplace`, `heroes`, `traps`, `training`, `buildings`,
  `expansion`, `reports`
- not `cqrs`, `es`, `postgres`, `adapter`, `repository` in app use-case file
  names.

Use technical names only for explicit technical contracts. If a file is a
mini-CQRS command/query helper or a projection repository contract, make that
role visible in the filename instead of using generic names like `service.rs`,
`queries.rs`, or `repositories.rs`.

Do not create root app contracts under `parabellum_app/ports/*`. App contracts
belong with their concern:
`parabellum_app/identity/ports.rs`,
`parabellum_app/scheduler/ports.rs`, `parabellum_app/villages/ports`,
`parabellum_app/map/ports.rs`, `parabellum_app/leaderboards/ports.rs`, etc.

`read_models` names app-facing projected/query shapes owned by the concern that
returns them:

- village query shapes live in `parabellum_app/villages/read_models`;
- cross-context query shapes live in focused root modules such as
  `parabellum_app/read_models`, `parabellum_app/map`, or
  `parabellum_app/leaderboards`;
- infrastructure projection rows and SQL persistence types remain in
  `parabellum_infra`, not in app read-model modules.

Do not create generic query bags such as `ports::queries`. If a type is not a
port trait, it does not belong under `ports`. Put it next to the use case that
owns the returned view, then make focused read ports depend on that type.

Avoid these names for new app code:

- `VillageCommandsPort`
- `VillageQueryPort`
- `VillageService`
- `VillageEsAdapter`
- `ScheduledActionRepository` as an app-level dependency name

Use `VillageCommandService` only for the low-level CQRS command dispatch helper.
It must not grow app orchestration behavior; new gameplay behavior belongs in a
concern-specific `*UseCases` type.

If an old name must remain for a concrete release need, mark it as
compatibility-only with rustdoc, route it through the new concern-specific use
case or port, and include the removal condition.

### Compatibility Rules

Prefer one canonical app path for each operation. Compatibility facades,
adapter-heavy pass-through methods, and broad ports are exceptions, not
architectural context.

Git history is the long-term context for removed implementations. Tests and
short behavior notes are the working context when replacing an implementation.

When replacing a concern boundary:

1. Write behavior-focused tests that describe the target use case through app
   requests and observable command/fact outcomes.
2. Write a short behavior checklist before deleting the superseded
   implementation. The checklist should name observable behavior, not private
   helper structure.
3. Add the concern-specific `requests`, `ports`, `use_cases`, and
   `read_models` modules needed by those tests.
4. Move orchestration from infrastructure adapters into the app use case.
5. Update web/infra call sites to depend on the new app entrypoint.
6. Remove the superseded facade or adapter-heavy method as soon as the
   canonical path owns the behavior.

Temporary compatibility is an exception. If it is required, it must be
explicitly documented with a removal condition and should not be treated as
architectural context:

```rust
#[deprecated(note = "use `MovementUseCases::send_attack`")]
/// Compatibility facade for older callers.
///
/// New code should use `MovementUseCases::send_attack`.
/// Remove after web and infra composition no longer depend on `VillageCommandsPort`.
```

Use Rust `#[deprecated]` for temporary compatibility exceptions. Deprecation is
more predictable than rustdoc alone because remaining call sites become visible
during checks. The note must name the replacement.

### Normalization Rules

When several methods are identical except for one or two details, normalize the
shared behavior in the use-case layer before adding more call sites.

Good normalization candidates:

- shared context loading,
- repeated ownership checks,
- deterministic id allocation,
- common timestamp/travel-time planning,
- command execution error mapping at a boundary,
- repeated "load, validate, build command, execute" control flow.

Avoid abstractions that hide gameplay intent. The normalized helper should name
the business step, not the technical shape. Prefer names such as
`plan_round_trip_movement`, `load_dispatch_context`, or
`execute_village_command` over generic names such as `handle_request` or
`process_action`.

For movement dispatch, attack and scout share round-trip planning, while
reinforcement can share one-way dispatch planning. Settlers can share travel
planning but should keep target-valley validation explicit because it is a
different game intent.

### Testing Pattern

Tests should describe behavior and domain/application outcomes, not internal
implementation details.

Prefer tests that assert:

- a use case rejects invalid ownership, target, timing, or resource conditions,
- a use case builds the expected aggregate command intent from app context,
- aggregate commands emit the expected canonical facts,
- domain methods in `parabellum_game` produce expected game-rule answers,
- infrastructure implementations satisfy the app port contract.

Avoid tests that assert:

- a specific private helper was called,
- SQL shape outside repository tests,
- CQRS/ES runtime wiring from app-layer unit tests,
- adapter implementation details when testing app orchestration.

For substantial behavior changes, use TDD at the concern boundary:

1. Add app-layer use-case tests with fake read ports, fake command executor,
   fixed clock, and deterministic id generator.
2. Move behavior until those tests pass.
3. Add or keep aggregate command tests for canonical fact emission.
4. Add infra contract/integration tests only after the app port shape is stable.

The developer running the change should run `cargo fmt`, `cargo check`, and the
relevant tests before merging. When Codex is asked not to run those commands, it
must document that they were intentionally not run.

### Application Components

The app layer is organized by application concern. Each concern owns its
request types, orchestration service, and required ports. Public callers should
enter through `GameApplication` or through the concern use-case service during
composition and tests.

#### Game Application Facade

`GameApplication` is the public facade used by the web layer. It delegates to
concern-specific use-case services and should not contain gameplay
orchestration. Adding a new public operation means:

- add a transport-independent request type in the owning concern,
- implement orchestration in the owning `*UseCases` type,
- add the narrow read/executor port required by that use case,
- expose a small delegating method on `GameApplication` only when the web/API
  boundary needs it.

The facade may remain broad because it is a composition boundary. Its methods
should stay shallow.

#### Village Command Use Cases

Village command use cases translate player intent into aggregate command intent
or workflow command intent. They load the minimum read context needed for the
operation, perform app-level checks, delegate pure rules to `parabellum_game`,
and then execute through a focused command executor port.

Current village command concerns:

- `MovementUseCases` owns outbound dispatch: reinforcement, attack, scout, and
  settlers.
- `MovementControlUseCases` owns changes to already-dispatched movement
  workflows, such as cancellation.
- `ReinforcementUseCases` owns reinforcement recall/release and trapped-troop
  control.
- `BuildingUseCases` owns add, upgrade, downgrade, and cancellation of building
  work.
- `DevelopmentUseCases` owns unit training plus Academy and Smithy research.
- `MarketplaceUseCases` owns resource sending and marketplace offer lifecycle.
- `HeroUseCases` owns hero creation, revival, point allocation, reset, resource
  focus, and hero lifecycle reads.
- `TrapUseCases` owns trap construction planning.
- `VillageProfileUseCases` owns village metadata changes such as renaming.
- `ReportUseCases` owns report reads and the event-backed report read marker.

Use-case modules live in `parabellum_app/villages/use_cases/<concern>.rs`.
Request types live in `parabellum_app/villages/requests/<concern>.rs`.

#### Village Read Use Cases

Village read use cases return app-facing query shapes. They should be explicit
about whether they return full projected state, a compact reference, an activity
view, or a gameplay snapshot used by another command.

Current village read concerns:

- `VillageActivityUseCases` returns queues, troop movements, and cancelable
  outgoing movement ids.
- `VillageArmyUseCases` returns army occupancy/state views.
- `MarketplaceUseCases` returns marketplace data and individual offers.
- `VillageExpansionUseCases` returns culture-point information for expansion
  buildings and explicitly refreshes player culture points before reading the
  player snapshot.
- `VillageReferenceUseCases` returns compact labels and positions for related
  villages.
- `VillageStateUseCases` returns full `VillageModel` projections.
- `HeroUseCases` returns player hero state and pending revival timestamps.
- `ReportUseCases` returns projected reports and unread counts.

Read use cases depend on focused read ports, not on a broad village query
facade. Read ports live in `parabellum_app/villages/ports/*_reads.rs`.

#### Movement Semantics

Movement dispatch uses domain travel mechanics and deterministic app sources
for ids and timestamps:

- attack and scout dispatch plan an outbound arrival and a return journey,
- reinforcement dispatch plans one-way support movement,
- settler dispatch validates target valley/foundation context before command
  execution,
- scout dispatch enforces scout-only selection through app/domain policy,
- hero participation is loaded as explicit context when a request includes a
  hero.

Movement cancellation uses an explicit source context:

- `request.village_id` is the source village selected by the player,
- `CancelTroopMovementContext.source_village_id` is the canonical source from
  the scheduled movement,
- cancellation is rejected before owner lookup if those two villages differ,
- return timing is derived from elapsed outbound travel at cancellation time.

#### Building, Development, And Trap Semantics

Building, development, hero, and trap use cases keep server speed explicit in
settings structs such as `BuildingSettings`, `DevelopmentSettings`, and
`HeroSettings`. Server speed should not be hidden inside an infrastructure
adapter.

Aggregate commands keep aggregate-local validation such as queue capacity,
resource availability, building prerequisites, research state, and hero-point
rules. Use cases may perform app-visible pre-command checks when the answer
comes from read models or pending workflows, such as expansion-slot usage or a
pending hero revival.

Trap capacity and cost planning delegates to `parabellum_game` trapper/domain
helpers. Resource availability checks should hydrate a domain `Village` when the
rule depends on domain stock behavior.

#### Reports

Reports are projected read models, but marking a report as read is represented
as a village event. `ReportUseCases::mark_report_as_read` loads the report to
choose the stream anchor, obtains the read timestamp from `Clock`, and emits the
already-planned report read command intent.

Projectors own report materialization and audience rows. App use cases own only
the public operation semantics.

#### Root App Concerns

Some use cases are not village concerns:

- `identity` owns authentication, player/user lookup, and registration
  orchestration.
- `map` owns world-map reads through `MapUseCases` and `MapReadPort`.
- `leaderboards` owns ranking reads through metric-specific result types and
  `LeaderboardReadPort`.
- `scheduler` owns the public operation boundary for processing due actions.

Root concerns follow the same structure as village concerns:

```text
parabellum_app/<concern>/requests.rs
parabellum_app/<concern>/ports.rs
parabellum_app/<concern>/use_cases.rs
```

Map reads must stay outside the village module. Leaderboard result names should
include the metric, such as `PlayerPopulationLeaderboardPage`, because future
leaderboards may rank different entities and metrics.

#### Registration

Registration is a cross-context application workflow owned by
`RegistrationUseCases`:

- hash the submitted password,
- create user/player rows and reserve the initial map valley through
  `RegistrationIdentityPort`,
- build the initial domain `Village` from the reserved valley,
- apply optional seed/test setup overrides to the initial village plan,
- append `FoundVillage`,
- append the initial `CreateHero`,
- optionally append `SetVillageResources`,
- clean up committed identity rows and map reservation if village foundation or
  initial hero creation fails.

`IdentityPort` is for authentication and identity/player lookup only. Do not add
registration back to `IdentityPort`; keep registration orchestration explicit in
`RegistrationUseCases`.

#### Scheduler

Scheduler is an operational app concern. `SchedulerUseCases` owns the public
request semantics for processing due actions and delegates execution to
`SchedulerPort`.

Workflow claiming, advisory locking, scheduled fact append, and projector
execution are infrastructure responsibilities. The app scheduler use case should
not know queue table details or CQRS runtime internals.

The ES scheduled-action worker is a runtime loop only. It owns poll interval,
batch size, tick logging, and calls into `VillageEsService::process_due_actions`.
It must not decode scheduled payloads, mutate action status, or append workflow
facts directly.

#### Read Models

App read models are returned by focused use cases and are not ports.

- village activity shapes live in
  `parabellum_app/villages/read_models/activity.rs`,
- marketplace and merchant movement views live in
  `parabellum_app/villages/read_models/marketplace.rs`,
- army occupancy views live in
  `parabellum_app/villages/read_models/village_army.rs`,
- cross-context shapes live in focused root modules such as
  `parabellum_app/read_models`, `parabellum_app/map`, or
  `parabellum_app/leaderboards`.

Choose the owning use-case concern before adding a read model. Do not create a
generic query/read-model bag.

#### Projection Repository Contracts

Projection repositories are technical persistence contracts for village
projectors and CQRS read models. They are app-owned interfaces implemented by
`parabellum_infra`, but they are not use-case ports and should not be injected
into app use cases directly.

Projection repository contracts live under
`parabellum_app/villages/projection_repositories/<concern>.rs`:

- `armies.rs` for projected army placement,
- `scheduled_actions.rs` for scheduled action projections,
- `marketplace.rs` for marketplace offer projections,
- `merchant_movements.rs` for active merchant movements derived from scheduled actions,
- `reports.rs` for report projection and audience rows,
- `heroes.rs` for projected hero state,
- `movements.rs` for village movement rows,
- `villages.rs` for projected village state and map occupancy,
- `expansion.rs` for expansion projection snapshots.

`projection_repositories::...` is the public app import path. Submodules exist
to keep ownership clear; callers should prefer the re-export path unless they
are editing the contract module itself.

#### Village CQRS Models

Village CQRS models are app-owned technical contracts used by projectors,
scheduled actions, and ES workflows. They are not domain models; game rules and
calculations belong in `parabellum_game`.

CQRS model contracts live under `parabellum_app/villages/models/<concern>.rs`:

- `villages.rs` for full projected village state,
- `movements.rs` for projected movement rows,
- `marketplace.rs` for marketplace projection rows and snapshots,
- `reports.rs` for report projection rows,
- `scheduled_actions.rs` for scheduled action records and payload dispatch,
- `workflows.rs` for scheduled workflow payloads.

`villages::models::...` is the public app import path. Submodules exist to keep
ownership clear, not to create multiple public naming styles.

## Infrastructure Layer Pattern

`parabellum_infra` implements app ports, projection repository contracts, and
CQRS/ES persistence. It may know about SQLx, Postgres, transactions,
advisory locks, event-store tables, read-model tables, and runtime wiring. It
must not own gameplay decisions that belong in `parabellum_app` use cases or
pure rules that belong in `parabellum_game`.

Infrastructure code is organized by persistence role:

- event-store components own canonical event and snapshot persistence;
- projection repositories own rebuildable read-model and operational queue
  persistence;
- projectors translate canonical events into projection rows;
- workflow modules translate due operational payloads into canonical facts;
- adapters implement app ports by delegating to infra services and repositories.

### Persistence Boundaries

Events and projections have different durability semantics:

- `EventStoreDb` is the logical database boundary for canonical event-store
  tables such as `es_events` and `es_snapshots`.
- `ProjectionDb` is the logical database boundary for rebuildable projection,
  read-model, and operational tables such as `rm_village`, `rm_armies`,
  `rm_reports`, `rm_map_fields`, and `rm_scheduled_actions`.
- `InfraDb` groups both boundaries when one physical Postgres pool backs the
  whole runtime.

Both boundaries share the same `PgPool` today. Constructors should still ask
for the logical boundary they need:

- `PostgresEventStore::new(EventStoreDb)`
- `PostgresSnapshotStore::new(EventStoreDb)`
- `PostgresVillageRepository::new(ProjectionDb)`
- `PostgresScheduledActionRepository::new(ProjectionDb)`
- `PostgresMapRepository::new(ProjectionDb)`

This keeps event-store and projection dependencies visible at composition
sites. It also prepares the system for a later move to separate schemas or
databases.

Current workflow execution still appends events and updates projections inside
one physical Postgres transaction. Moving `EventStoreDb` and `ProjectionDb` to
separate databases requires an explicit runtime contract change: event append
must become the durable write, and projection updates must become replayable
eventual work.

Event-store persistence should be organized as its own store module:

- `stores/mod.rs` is an index and public re-export boundary only;
- `stores/event_store.rs` owns appending and loading canonical events from
  `es_events`;
- `stores/snapshots.rs` owns derived aggregate snapshots in `es_snapshots`;
- `stores/rows.rs` owns typed SQL row structs and conversions for event-store
  persistence.

Do not mix event appends, snapshot reads, and raw row conversion in one file.
The event store is the canonical durability boundary; snapshots are an
optimization and must remain rebuildable from events.

Replay tooling should mirror that separation:

- `replay/mod.rs` owns the public `ReplayService` type and re-exports replay
  request/summary models;
- `replay/request.rs` owns `ReplayRequest`, `ReplayTarget`, `ReplayMode`, and
  `ReplaySummary`;
- `replay/runner.rs` owns dry-run/full replay loops, projection reset, and
  projector dispatch;
- `replay/filters.rs` owns target/event acceptance rules;
- `replay/snapshots.rs` owns aggregate snapshot rebuilds.

Replay is allowed to reset projection tables because projections are
rebuildable. It must not mutate canonical event rows.

### Repository Shape

Postgres repositories implement app-owned contracts. They should expose a small
public surface:

- app trait methods required by `parabellum_app`,
- explicit `*_in_tx(&mut Transaction<...>, ...)` helpers used by projectors or
  workflow transaction boundaries,
- no app orchestration behavior.

Avoid `Option<&mut Transaction>` as the default transaction pattern. Use two
explicit entrypoints instead:

- pool-backed app/repository methods for standalone reads and writes,
- `*_in_tx` methods for projector and workflow code already inside a
  transaction.

If a query needs to run in both modes, keep SQL construction and row conversion
shared, not the transaction parameter itself.

### Infrastructure Service Organization

Infrastructure services are orchestration facades over repositories, CQRS
runtime wiring, workflow helpers, and projector transaction boundaries. They
should not grow into persistence modules themselves.

`VillageEsService` is the infrastructure facade for village CQRS/ES command,
scheduler, and read-helper flows. Its root module owns only the service type,
construction, exported context structs, and module declarations. Concern-specific
service helpers live in focused submodules:

- `village_service/commands.rs` for direct CQRS command dispatch helpers;
- `village_service/marketplace_commands.rs` for marketplace command
  orchestration that coordinates offer projections and workflow facts;
- `village_service/economy.rs` for pre-command resource materialization;
- `village_service/workflow_append.rs` for cross-stream workflow append,
  projector dispatch, and snapshot refresh mechanics;
- `village_service/scheduler.rs` for due-action processing and operational
  scheduler coordination;
- `village_service/queries/buildings.rs` for building cancellation read context;
- `village_service/queries/heroes.rs` for hero read-model lookups;
- `village_service/queries/marketplace.rs` for marketplace page read
  composition;
- `village_service/queries/movements.rs` for troop movement, army state, and
  movement-control read contexts;
- `village_service/queries/reports.rs` for report reads and command-backed
  mark-read orchestration;
- `village_service/queries/scheduled_actions.rs` for queue and status-count
  reads;
- `village_service/queries/villages.rs` for basic village state reads.

The `queries/mod.rs` file should remain an index with module docs and module
declarations only. Query modules may compose multiple repositories into one
API-facing read model, but broad SQL and row mapping still belongs in the
repositories that own the underlying projection tables.

When a service helper needs a repeated mapping from app projection models to UI
read models, keep that mapping local to the concern module unless it is shared
by multiple modules. Do not place concern-specific mapping helpers on the root
service type just because they are convenient to call.

### SQL Row And Query Conventions

SQL result shapes should use dedicated structs:

- derive `sqlx::FromRow` for concrete query result rows;
- name rows by query shape, for example `DbScheduledActionRow`,
  `DbPendingTroopArrivalActionRow`, or `DbMerchantMovementRow`;
- keep row structs and conversions near the repository concern, preferably in
  `rows.rs` when the repository is split into a directory module;
- use explicit `From`/`TryFrom` conversions between DB rows and app/domain
  models.

Avoid raw `PgRow` mapping unless the query shape is genuinely dynamic and a
typed row would add more complexity than clarity. App/domain models should not
double as SQL row structs unless the database shape truly is the app contract.

SQL strings and `QueryBuilder` construction belong near the repository concern,
preferably in `queries.rs` when a repository grows beyond one file. Repositories
should not inline large unrelated SQL blocks beside app-port implementations if
the file is already mixing writes, reads, row mapping, and helper queries.

Database enum values should use the same names as the app model variants they
persist. Do not normalize naming drift with hidden SQLx renames unless the
database is intentionally integrating with an external schema. For internal
tables, add a migration and align both sides.

Scheduled-action queries should use the app-owned `ScheduledActionFilter` for
semantic predicates such as action types, statuses, player, village, movement,
ordering, and limits. Infrastructure translates that filter into projection SQL
and JSON workflow-field predicates. App contracts must not expose SQL columns,
JSON paths, or raw payload keys as filtering concepts.
Composed projection views may use explicit SQL when a single result shape spans
multiple scheduled-action types, such as active merchant movements.

Report queries should use the app-owned `ReportFilter` for audience-scoped
predicates such as player visibility, report id, unread state, pagination, and
report category. Report category filters use `ReportKind` values; infrastructure
translates them to the canonical `rm_reports.report_type` discriminators.
Report projections are split between `rm_reports`, which stores payload and
actor/target context, and `rm_report_reads`, which stores per-player audience
visibility and read state. Application callers must not query reports without an
audience player.

Report projectors should centralize report materialization through one helper
that assigns the `ReportKind`, serializes the payload, stores actor/target
context, and writes audience rows. Event-specific projector modules should build
the report payload and audience rule only; they should not construct
`ProjectedReport` rows directly.

Village movement queries should use the app-owned `VillageMovementFilter`.
Every movement query is anchored to the viewing village and may optionally
filter by viewing direction or movement type. Infrastructure translates those
semantic filters to `rm_village_movements` columns and database enum values.

The village projection repository is the main read-model owner for `rm_village`
rows and may refresh derived read state before returning a `VillageModel`.
The app-facing `VillageRepository` is a read boundary only. It should not expose
table-field setters or projector maintenance operations.

Its SQL row adapters belong in `villages/rows.rs`, reusable village SELECT
builders belong in `villages/queries.rs`, and full-model village write SQL
builders belong in `villages/writes.rs`. When a projector changes only
`rm_village`, prefer storing a complete updated `VillageModel` through
`store_village_model_in_tx`. When an event changes multiple read models, keep
those writes inside one transaction and call the relevant table owners from the
projector.
For example, village lifecycle and conquest projection may update both
`rm_village` and `rm_map_fields`; village state must be stored through the
village repository, while map occupancy must be stored through the map
repository in the same transaction.

When an event carries absolute economy facts for `rm_village`, such as stored
resources or busy merchant counts, apply those facts through one typed
projector helper and store the full `VillageModel` once. Do not add one
repository setter per field or one helper per possible fact combination.
Meaningful lifecycle transitions such as conquest should also use named
projector fact applicators rather than anonymous field assignments.

Pool-backed methods and `*_in_tx` projector helpers must remain separate
entrypoints; shared construction should happen through typed projection/query
builders rather than `Option<&mut Transaction>`.
Helpers that commit village projection state changes belong in
`villages/state_changes.rs`; `villages/writes.rs` should stay focused on
full-model write SQL builder functions.

Read helpers that fetch `VillageModel` rows and refresh them for application
reads belong in `villages/reads.rs`; the repository trait impl should delegate
to those helpers instead of repeating row-to-model refresh loops. When derived
refresh needs to persist materialized state, store the refreshed complete
`VillageModel`; do not maintain partial derived-field update queries.

Building event projection should hydrate a domain `Village`, apply the event
through domain mutation methods such as `add_building_at_slot`,
`set_building_level_at_slot`, or `remove_building_at_slot`, copy the resulting
domain-owned state back into `VillageModel`, and store the full model. Do not
reimplement building-derived production, storage capacity, merchant count,
population, culture point, or resource ticking calculations in
`parabellum_infra`. Village founding follows the same rule: the lifecycle
projector builds a domain-hydrated `VillageModel` and stores it through the
full-model upsert path.

Concern-specific snapshot queries owned by `rm_village`, such as expansion
culture and ownership counters, should live in focused helper modules like
`villages/expansion.rs` and return app-owned snapshot structs through typed
SQLx row structs.

Pure derived read-model calculations belong in `villages/refresh.rs`; repository
methods should load the required context and delegate the synchronous refresh
calculation there. Cross-projection lookups used only by refresh, such as active
hero resource bonuses, should live in focused helper modules instead of the main
repository body. Gameplay formulas used during refresh, such as army upkeep
modifiers and loyalty regeneration, must be delegated to `parabellum_game`
domain helpers.

Repository contracts should follow projection ownership. Marketplace offer
reads belong to `MarketplaceRepository` because they read `rm_marketplace_offers`.
Active merchant movement reads belong to `MerchantMovementRepository` because
they are derived from scheduled merchant actions.

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
- Shared projector helper modules should be named by concern. For example,
  `village_projector/hydration.rs` owns complete domain village hydration, and
  `village_projector/economy.rs` owns fact-carried resource and merchant
  materialization.
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
- `VillageModel` is the village read-model shape without embedded army rows.
  Projector code that needs a domain `Village` must load `VillageArmyContext`
  from `rm_armies` and hydrate complete village state. Do not expose a generic
  "village from model" helper that silently omits armies.
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

## Repository Design Rules

- Repositories own SQL, read-model persistence details, and database-specific
  row shapes. Application, domain, query services, and workflows should ask
  repositories for set/count/existence/snapshot answers instead of composing
  SQL directly.
- When several repository methods differ only by predicates over the same
  result shape, expose one typed, builder-style filter object such as
  `ArmyListFilter::new().state(...).home_village(...)` instead of growing
  `list_*_by_*` variants. Named helpers are acceptable as thin compatibility
  or readability wrappers, but the SQL should live behind one list/query
  implementation.
- Multi-value read-model answers should use named snapshot structs when the
  values belong together. This keeps call sites from assembling the same
  repository reads repeatedly and documents the consistency boundary.
- Domain rehydration should use domain-owned snapshot/input structs, not
  database rows or app read-model structs directly. Translate read models into
  domain snapshots at application/infrastructure boundaries, and make missing
  context explicit in helper names. Avoid broad `From<ReadModel> for
  DomainModel` conversions because they hide missing context and make
  economy-only or partial hydration look authoritative.
- For row mapping, prefer database DTOs such as `DbVillageModelRow` with
  `sqlx::FromRow`, then convert with `From`/`TryFrom` into app or domain
  structs. Do not map SQL rows directly into domain structs unless the domain
  type is intentionally persistence-shaped.
- Manual `Row::get`/`Row::try_get` mapping is reserved for custom or dynamic
  query shapes, especially joins that assemble nested objects. Once a manual
  mapping is repeated, extract a `Db*Row` DTO or a single mapper function.
- Repeated column lists should be centralized with `*_select_sql()` helpers or
  typed query helpers. This keeps schema changes local and reduces accidental
  drift between read paths.
- Use SQL/read-model predicates for broad filtering. Rust filtering is reserved
  for domain behavior after a narrow read or for cases where the database
  cannot reasonably express the domain transformation.

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
