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
      training.rs
      movements.rs
      marketplace.rs
      heroes.rs
    use_cases/
      buildings.rs
      training.rs
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
      army_reads.rs
      hero_reads.rs
      village_reads.rs
      clock.rs               # time source when a use case needs "now"
      ids.rs                 # id source when a use case creates canonical ids
    policies/
      ...                    # app policies combining domain rules and app context
    views/
      queues.rs              # app/API query response shapes
      movements.rs
      marketplace.rs
      reports.rs
      army_state.rs
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
refactors should reuse or lightly improve them instead of adding parallel
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

`parabellum_app/ports/*` is deprecated as an app organization pattern. App
contracts belong with their concern:
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

### Deprecation And Migration Rules

Use replacement-first migration for architectural refactors. The preferred
default is to capture current behavior, introduce the new pattern, rewire call
sites, and remove the old path in the same slice. Do not keep old broad ports or
adapter-heavy methods as compatibility context unless there is a concrete
release need.

Git history is the long-term context for removed implementations. Tests and
short migration notes are the working context during the refactor.

Migration steps for each concern:

1. Write behavior-focused tests that describe the target use case through app
   requests and observable command/fact outcomes.
2. Write a short behavior checklist from the old implementation before deleting
   it. The checklist should name observable behavior, not private helper
   structure.
3. Add the concern-specific `requests`, `ports`, `use_cases`, and
   `read_models` modules needed by those tests.
4. Move orchestration from infrastructure adapters into the app use case.
5. Update web/infra call sites to depend on the new app entrypoint.
6. Remove the old facade or adapter-heavy method as soon as the new path owns
   the behavior.

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

### Normalization During Rewrites

When a refactor reveals several methods that are identical except for one or two
details, normalize the shared behavior in the new use-case layer before wiring
new call sites.

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

For movement dispatch, attack and scout can share round-trip planning, while
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

For refactors, use TDD at the concern boundary:

1. Add app-layer use-case tests with fake read ports, fake command executor,
   fixed clock, and deterministic id generator.
2. Move behavior until those tests pass.
3. Add or keep aggregate command tests for canonical fact emission.
4. Add infra contract/integration tests only after the app port shape is stable.

The developer running the change should run `cargo fmt`, `cargo check`, and the
relevant tests before merging. When Codex is asked not to run those commands, it
must document that they were intentionally not run.

### First Refactor Target: Movement Dispatch

The first vertical slice is outbound movement dispatch:

- `send_reinforcement`
- `send_attack`
- `send_scout`
- `send_settlers`

This slice is the pattern-setter because it exercises source/target
context loading, hero context, movement speed, travel duration, canonical id
generation, timestamp generation, aggregate command construction, and domain
dispatch policies.

Target app modules:

```text
parabellum_app/villages/requests/movements.rs
parabellum_app/villages/use_cases/movements.rs
parabellum_app/villages/ports/command_executor.rs
parabellum_app/villages/ports/movement_reads.rs
parabellum_app/villages/ports/clock.rs
parabellum_app/villages/ports/ids.rs
```

Target public names:

- `MovementUseCases`
- `MovementReadPort`
- `VillageCommandExecutor`
- `Clock`
- `IdGenerator`
- `MovementDispatchContext`
- `SendAttackRequest`
- `SendReinforcementRequest`
- `SendScoutRequest`
- `SendSettlersRequest`

Finalized movement dispatch means:

- movement request types live under `parabellum_app/villages/requests`,
- app orchestration lives in `MovementUseCases`,
- the broad `VillageCommandsPort` no longer exposes dispatch methods,
- infrastructure implements focused read/executor ports for the slice,
- HTTP handlers import movement requests from the new request module,
- public boundary types have rustdoc,
- behavior tests cover app-visible outcomes and command intent.

Behavior tests cover:

- attack dispatch computes arrival/return timestamps from domain travel
  mechanics and deterministic clock/id inputs,
- reinforcement dispatch includes optional hero context when present,
- scout dispatch enforces scout-only dispatch through the app/domain policy,
- settlers dispatch rejects occupied or non-valley targets before command
  execution,
- ownership errors are reported as `ApplicationError::Game` and do not execute
  commands.

### Refactor Slice: Reinforcement Control

Reinforcement and trapped-troop control follows the same replacement-first
application pattern as movement dispatch:

- request types live in `parabellum_app/villages/requests/reinforcements.rs`,
- orchestration lives in `ReinforcementUseCases`,
- infrastructure implements `ReinforcementReadPort` and
  `ReinforcementCommandExecutor`,
- `VillageCommandsPort` does not expose recall/release/disband methods,
- HTTP handlers import reinforcement requests from the new request module,
- selected-unit validation delegates to `ReinforcementControl`,
- trap release state is planned in the use case from current read-model
  context and `Trapper` domain helpers,
- behavior tests assert ownership, deterministic ids/timestamps, and command
  intent without binding to adapter internals.

### Refactor Slice: Movement Control

Movement control covers operations that alter already-dispatched movement
workflows. The first operation in this slice is troop movement cancellation.

This slice follows the same app-layer pattern:

- request types live in `parabellum_app/villages/requests/movement_control.rs`,
- orchestration lives in `MovementControlUseCases`,
- infrastructure implements `MovementControlReadPort` and
  `MovementControlCommandExecutor`,
- `VillageCommandsPort` does not expose movement-control methods,
- HTTP handlers import movement-control requests from the new request module,
- the use case owns app-level ownership and cancel-window checks,
- command construction uses injected `Clock` and `IdGenerator`,
- behavior tests assert source ownership, arrived/expired rejection,
  deterministic ids/timestamps, and command intent.

Movement cancellation context is intentionally explicit:

- `request.village_id` is the source village chosen by the player,
- `CancelTroopMovementContext.source_village_id` is the movement's canonical
  source village from the read model,
- cancellation is rejected before owner lookup when those two villages differ,
- the source village owner must match `request.player_id`,
- return timing is derived from elapsed outbound travel at cancellation time.

### Refactor Slice: Building Lifecycle

Building lifecycle covers scheduling and canceling building construction:

- `add_building`
- `upgrade_building`
- `downgrade_building`
- `cancel_building_construction`

This slice follows the same app-layer pattern:

- request types live in `parabellum_app/villages/requests/buildings.rs`,
- orchestration lives in `BuildingUseCases`,
- infrastructure implements `BuildingReadPort` and `BuildingCommandExecutor`,
- `VillageCommandsPort` does not expose building lifecycle methods,
- HTTP handlers import building requests from the new request module,
- server speed is explicit `BuildingSettings`, not hidden adapter state,
- add/upgrade/downgrade use cases translate player intent into command intent,
- cancellation loads scheduled-action context through the read port, checks
  ownership and execution time, then builds cancellation command intent.

Building lifecycle tests should stay behavior-oriented:

- command intent contains the configured server speed,
- cancellation rejects non-owner context before command execution,
- cancellation rejects actions whose execution time has passed,
- cancellation uses read-model refund/action context and injected time.

### Refactor Slice: Village Profile

Village profile covers metadata changes for an existing village. The first
operation in this slice is `rename_village`.

This slice follows the same app-layer pattern:

- request types live in
  `parabellum_app/villages/requests/village_profile.rs`,
- orchestration lives in `VillageProfileUseCases`,
- infrastructure implements `VillageProfileCommandExecutor`,
- `VillageCommandsPort` does not expose village profile methods,
- HTTP handlers import village profile requests from the new request module,
- command validation, including name trimming and length checks, remains in
  the aggregate command handler.

Village profile tests should assert command intent and avoid duplicating domain
validation already covered by aggregate command tests.

### Refactor Slice: Village Development

Village development covers queueing unit training and research:

- `train_units`
- `research_academy`
- `research_smithy`

This slice follows the same app-layer pattern:

- request types live in `parabellum_app/villages/requests/development.rs`,
- orchestration lives in `DevelopmentUseCases`,
- infrastructure implements `DevelopmentReadPort` and
  `DevelopmentCommandExecutor`,
- `VillageCommandsPort` does not expose training or research methods,
- HTTP handlers import development requests from the new request module,
- server speed is explicit `DevelopmentSettings`, not hidden adapter state,
- aggregate commands keep queue, resource, building, and research rules,
- the app use case performs read-side expansion-unit validation before
  queueing settlers or chiefs.

Development tests should assert command intent, configured server speed, and
app-visible pre-command guards such as invalid unit index or expansion-slot
rejection. They should not duplicate aggregate queue/resource/building tests.

### Refactor Slice: Marketplace Queries

Marketplace queries covers app-facing marketplace reads:

- `get_marketplace_offer`
- `get_marketplace_data`

This slice keeps marketplace reads inside the marketplace concern:

- request types live in `parabellum_app/villages/requests/marketplace.rs`,
- read orchestration lives in `MarketplaceUseCases`,
- infrastructure implements the existing `MarketplaceReadPort`,
- `VillageQueryPort` no longer exposes marketplace methods,
- offer and marketplace view reads share the same port used by marketplace
  command orchestration.

Marketplace query tests should assert read-port delegation and no command
execution. They should not duplicate marketplace repository or projection tests.

### Refactor Slice: Village Activity Queries

Village activity queries covers app-facing queue and movement activity reads:

- `get_village_queues`
- `get_village_troop_movements`
- `list_cancelable_outgoing_movement_ids`

This slice keeps dashboard/activity reads out of the broad query port:

- request types live in `parabellum_app/villages/requests/activity.rs`,
- read orchestration lives in `VillageActivityUseCases`,
- infrastructure implements `VillageActivityReadPort`,
- `VillageQueryPort` no longer exposes queue or troop-movement activity methods,
- cancelable movement filtering receives `Clock::now()` from the use case, not
  from the adapter.

Activity query tests should assert read-port delegation and deterministic clock
usage. They should not duplicate scheduled-action or movement repository tests.

### Refactor Slice: Village Army Queries

Village army queries covers app-facing army state reads:

- `get_village_army_state_view`

This slice keeps army views separate from queue/movement activity reads:

- request types live in `parabellum_app/villages/requests/village_army.rs`,
- read orchestration lives in `VillageArmyUseCases`,
- infrastructure implements `VillageArmyReadPort`,
- `VillageQueryPort` no longer exposes army state methods.

Army query tests should assert read-port delegation. They should not duplicate
army repository, reinforcement, or trap projection tests.

### Refactor Slice: Map Queries

Map queries are cross-context reads and are not village use cases:

- `get_map_region`
- `get_map_field`
- `get_map_region_tile_by_field_id`

This slice keeps world-map reads outside the village module:

- request types live in `parabellum_app/map/requests.rs`,
- read contracts live in `parabellum_app/map/ports.rs`,
- orchestration lives in `MapUseCases`,
- map reads use `MapReadPort`,
- `GameApplication` delegates map facade methods to `MapUseCases`,
- `VillageQueryPort` does not expose map methods,
- infrastructure composes `PostgresMapRepository` into `MapUseCases`,
- `VillageEsService` must not grow pass-through map query helpers.

Map query tests should assert app-visible repository delegation and any
coordinate/id wrapping behavior owned by the map repository. They should not
duplicate village projection tests.

### Refactor Slice: Map Organization

Map is a first-class app concern. It should not use root `ports` or a flat
`map.rs` module.

This slice normalizes map into the same concern-owned layout:

- map request shapes live in `parabellum_app/map/requests.rs`,
- map read contracts live in `parabellum_app/map/ports.rs`,
- map orchestration lives in `parabellum_app/map/use_cases.rs`,
- `MapRepository` is renamed to `MapReadPort`,
- infrastructure keeps the concrete adapter name `PostgresMapRepository`,
- root `parabellum_app/ports` is removed.

Do not reintroduce `parabellum_app::ports`. New contracts must live with their
owning app concern.

### Refactor Slice: Leaderboards

Leaderboards are root application reads because future rankings may aggregate
player, village, army, alliance, or other cross-context metrics.

The first leaderboard metric is player population:

- `get_player_population_leaderboard_page`

This slice keeps ranking reads outside identity and villages:

- request types live in `parabellum_app/leaderboards/requests.rs`,
- app orchestration lives in `LeaderboardUseCases`,
- result types are metric-specific, for example
  `PlayerPopulationLeaderboardPage` and `PlayerPopulationLeaderboardEntry`,
- read ports live in `parabellum_app/leaderboards/ports.rs`,
- infrastructure implements `LeaderboardReadPort` directly,
- `PlayerRepository` does not expose leaderboard methods,
- `VillageQueryPort` and `VillageEsService` do not expose leaderboard methods.

Leaderboard naming must include the metric when the app boundary can be called
from another layer. Avoid generic names like `LeaderboardPage` once more than
one ranking can plausibly exist.

Leaderboard tests should assert pagination normalization and read-port
delegation. SQL ordering and aggregation should be covered by infra repository
or endpoint integration tests.

### Refactor Slice: Village Expansion Reads

Village expansion reads cover culture-point information used by expansion
buildings:

- `get_expansion_culture_info`

This slice keeps expansion-specific read behavior out of the broad village
query port:

- request types live in `parabellum_app/villages/requests/expansion.rs`,
- orchestration lives in `VillageExpansionUseCases`,
- read ports live in `parabellum_app/villages/ports/expansion_reads.rs`,
- infrastructure implements `ExpansionReadPort`,
- `VillageQueryPort` does not expose expansion culture methods,
- `VillageEsService` does not expose expansion culture pass-through helpers.

This use case intentionally refreshes player culture points before returning
the player culture snapshot. That write-through behavior must remain explicit
in `VillageExpansionUseCases`; it must not be hidden inside a generic query
port or a service method named like a pure read.

Expansion tests should assert refresh-before-player-read ordering, required CP
calculation, and app-visible output. Repository tests should cover SQL snapshot
aggregation.

### Refactor Slice: Village References

Village references are compact village labels used to render relationships in
other views:

- village name,
- village position,
- village id.

This slice replaces vague `VillageInfo` naming:

- request types live in `parabellum_app/villages/requests/village_references.rs`,
- orchestration lives in `VillageReferenceUseCases`,
- read ports live in `parabellum_app/villages/ports/village_reference_reads.rs`,
- compact read model is `VillageReference`,
- app facade method is `get_village_references`,
- embedded maps should be named `village_references`,
- the old broad `VillageQueryPort` does not expose reference lookup methods.

Use `VillageReference` only for display labels and route/distance context. Do
not use it as a substitute for full village state.

### Refactor Slice: Village State Reads

Village state reads return full `VillageModel` projections. They are heavier
than directory summaries and should be named as full state reads:

- `get_village_state`,
- `list_player_village_states`.

This slice removes the remaining broad query port:

- request types live in `parabellum_app/villages/requests/village_state.rs`,
- orchestration lives in `VillageStateUseCases`,
- read ports live in `parabellum_app/villages/ports/village_state_reads.rs`,
- infrastructure implements `VillageStateReadPort`,
- app facade methods use `state`, not `model`,
- `VillageQueryPort` is removed.

Use full village state only where callers need the complete projection or
domain hydration. Player profiles, public directories, and lightweight UI lists
should use a later summary/directory read model instead of `VillageModel`.

### Refactor Slice: Heroes

Heroes covers player hero lifecycle and hero profile updates:

- `create_hero`
- `revive_hero`
- `assign_hero_points`
- `reset_hero_points`
- `set_hero_resource_focus`

This slice follows the same app-layer pattern:

- request types live in `parabellum_app/villages/requests/heroes.rs`,
- orchestration lives in `HeroUseCases`,
- infrastructure implements `HeroReadPort` and `HeroCommandExecutor`,
- the old broad `VillageCommandsPort` is removed instead of left empty,
- HTTP handlers import hero requests from the new request module,
- server speed is explicit `HeroSettings`, not hidden adapter state,
- revival uses injected `Clock` and `IdGenerator`,
- aggregate commands keep ownership, resource, mansion, and hero-point domain
  validation.

Hero tests should assert loaded-context command intent, deterministic revival
id/time, and pre-command guards such as pending revival or existing living hero.
They should not duplicate aggregate hero validation.

### Refactor Slice: Hero Queries

Hero queries covers app-facing hero lifecycle reads:

- `get_hero_by_player`
- `get_pending_hero_revival_at`

This slice keeps hero reads in the hero concern instead of the broad query port:

- request types live in `parabellum_app/villages/requests/heroes.rs`,
- read orchestration lives in `HeroUseCases`,
- infrastructure implements the existing `HeroReadPort`,
- `VillageQueryPort` no longer exposes hero methods,
- app-facing naming includes `_at` for timestamp-returning revival reads.

Hero query tests should assert read-port delegation and naming semantics. They
should not duplicate hero repository or scheduled-action repository tests.

### Refactor Slice: Reports

Reports covers projected report reads and the event-backed read marker:

- `list_reports_for_player`
- `get_report_for_player`
- `count_unread_reports_for_player`
- `mark_report_as_read`

This slice follows the app-layer pattern with one important distinction:
reports are projected read models, but marking a report as read still appends a
village event.

- request types live in `parabellum_app/villages/requests/reports.rs`,
- orchestration lives in `ReportUseCases`,
- infrastructure implements `ReportReadPort` and `ReportCommandExecutor`,
- `VillageQueryPort` no longer exposes report methods,
- `mark_report_as_read` loads the report first to determine its village stream
  anchor, then emits `MarkReportRead`,
- the read timestamp comes from injected `Clock`, not from the command handler,
- aggregate commands only emit the already-planned report event.

Report tests should assert read-port delegation, stream-anchor selection,
deterministic read timestamps, and missing-report rejection before command
execution. They should not duplicate report projector or repository tests.

### Refactor Slice: Trap Building

Trap building follows the same app-layer pattern:

- request types live in `parabellum_app/villages/requests/traps.rs`,
- orchestration lives in `TrapUseCases`,
- infrastructure implements `TrapReadPort` and `TrapCommandExecutor`,
- `VillageCommandsPort` does not expose trap-building methods,
- HTTP handlers import trap requests from the new request module,
- trap capacity and cost planning delegates to `parabellum_game::models::trapper`,
- resource availability checks use the domain `Village` model via hydration,
- behavior tests assert ownership, resource/capacity rejection, deterministic
  ids/timestamps, and command intent.

### Refactor Slice: Village Read Models

Village read models are app-facing query shapes returned by focused village use
cases. They are not ports and must not live under `parabellum_app/ports`.

This slice removes the old generic `ports::queries` module and assigns read
models to their owning village concerns:

- activity queues and troop movements live in
  `parabellum_app/villages/read_models/activity.rs`,
- marketplace offers and merchant movements live in
  `parabellum_app/villages/read_models/marketplace.rs`,
- army occupancy views live in
  `parabellum_app/villages/read_models/village_army.rs`,
- focused village read ports return these concern-owned read models,
- web and infra import app query shapes through
  `parabellum_app::villages::read_models`,
- root `parabellum_app/ports` is not used for read model ownership.

When adding a new query shape, choose the owning use-case concern first. If no
single concern owns it, create a focused root module such as `map` or
`leaderboards`; do not add another generic query module.

### Refactor Slice: Projection Repository Contracts

Projection repositories are technical persistence contracts for village
projectors and CQRS read models. They are app-owned interfaces implemented by
`parabellum_infra`, but they are not use-case ports.

This slice replaces the old single-file repository bag with focused contract
modules:

- army projection contracts live in
  `parabellum_app/villages/projection_repositories/armies.rs`,
- scheduled-action projection contracts live in
  `parabellum_app/villages/projection_repositories/scheduled_actions.rs`,
- marketplace projection contracts live in
  `parabellum_app/villages/projection_repositories/marketplace.rs`,
- report projection contracts live in
  `parabellum_app/villages/projection_repositories/reports.rs`,
- hero projection contracts live in
  `parabellum_app/villages/projection_repositories/heroes.rs`,
- village movement projection contracts live in
  `parabellum_app/villages/projection_repositories/movements.rs`,
- village state and map occupancy projection contracts live in
  `parabellum_app/villages/projection_repositories/villages.rs`,
- expansion projection snapshots live in
  `parabellum_app/villages/projection_repositories/expansion.rs`.

`projection_repositories::...` re-exports remain the public app path for these
technical contracts. This keeps imports concise while preventing unrelated
repository traits from accumulating in one large file.

### Refactor Slice: Village CQRS Models

Village CQRS models are app-owned technical contracts used by projectors,
scheduled actions, and ES workflows. They are not domain models; game rules and
calculations still belong in `parabellum_game`.

This slice replaces the old broad `parabellum_app/villages/models.rs` file with
focused modules:

- projected village state lives in `parabellum_app/villages/models/villages.rs`,
- projected village movement rows live in
  `parabellum_app/villages/models/movements.rs`,
- marketplace projection rows live in
  `parabellum_app/villages/models/marketplace.rs`,
- report projection rows live in `parabellum_app/villages/models/reports.rs`,
- scheduled action records and payload dispatch live in
  `parabellum_app/villages/models/scheduled_actions.rs`,
- scheduled workflow payloads live in
  `parabellum_app/villages/models/workflows.rs`.

`villages::models::...` remains the public import path for these contracts.
Submodules exist to keep ownership clear, not to create multiple public naming
styles.

### Refactor Slice: Identity Organization

Identity is a first-class app concern, not a miscellaneous root port file.

This slice moves identity contracts out of `parabellum_app/ports/identity.rs`
and assigns them to the identity concern:

- registration request shapes live in `parabellum_app/identity/requests.rs`,
- authentication, registration, user, and player contracts live in
  `parabellum_app/identity/ports.rs`,
- public callers import through `parabellum_app::identity`,
- root `parabellum_app/ports` no longer exposes identity contracts.

### Refactor Slice: Registration Orchestration

Registration is a cross-context application workflow. The app layer owns the
ordering and command planning:

- hash the submitted password,
- create user/player rows and reserve the initial map valley through
  `RegistrationIdentityPort`,
- build the initial domain `Village` from the reserved valley,
- apply seed/test setup overrides to the initial village plan,
- append `FoundVillage`,
- append the initial `CreateHero`,
- optionally append `SetVillageResources`,
- clean up committed identity rows and map reservation if village foundation or
  initial hero creation fails.

Infrastructure still owns the database transaction that creates identity rows
and soft-reserves the map field. That transaction is exposed as a focused app
port instead of being hidden inside the registration orchestration. Initial
village ES commands are exposed through `InitialVillageCommandExecutor`.

Do not reintroduce `IdentityPort::register_player`. Registration belongs in
`RegistrationUseCases`; `IdentityPort` is for authentication and identity/player
lookup only.

### Refactor Slice: Scheduler

Scheduler is an operational application concern, not a direct `GameApplication`
port dependency.

This slice moves scheduler contracts out of `parabellum_app/ports/scheduler.rs`
and assigns them to the scheduler concern:

- scheduler request shapes live in `parabellum_app/scheduler/requests.rs`,
- scheduler execution contracts live in `parabellum_app/scheduler/ports.rs`,
- scheduler orchestration lives in `SchedulerUseCases`,
- `GameApplication::process_due_actions` delegates to `SchedulerUseCases`,
- infrastructure implements `SchedulerPort` through the ES adapter,
- root `parabellum_app/ports` no longer exposes scheduler contracts.

Keep workflow claiming, locking, and fact append behavior in infrastructure.
The app use case owns only the public operation boundary and request semantics.

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
  domain snapshots at application/infrastructure boundaries, and use explicit
  hydration helpers when required context may be partial. Avoid broad
  `From<ReadModel> for DomainModel` conversions because they hide missing
  context and make economy-only or partial hydration look authoritative.
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
