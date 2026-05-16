# Parabellum Architecture

Parabellum is a multiplayer strategy game backend organized as a layered CQRS/ES system with strict consistency for game actions.

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
  - Hosts CQRS runtime wiring, event store/snapshot store, projectors, scheduler worker, and read-model repositories.

- `parabellum_web`
  - HTTP API and session/auth token handling.
  - Calls `GameApplication` only.

- `parabellum_server`
  - Runtime composition and startup.
  - Wires `GameApplication` with DB adapters, starts HTTP server and scheduler.

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
- Scheduling:
  - validations at scheduling time
  - completion commands deterministic
  - scheduler reads due actions from `rm_scheduled_actions`
  - scheduler does not mutate state directly; it issues completion commands.
- Read models are query sources (`rm_village`, `rm_armies`, `rm_village_movements`, `rm_reports`, `rm_map_fields`, etc.).

## Read-Model Ownership Contract

To avoid drift, each gameplay concern has exactly one canonical read-model owner:

- `rm_armies`:
  - canonical source for troop state (`home`, `stationed`, `moving`)
  - canonical source for UI troop availability and army cards
- `rm_village_movements`:
  - canonical source for movement timeline (outgoing/incoming/return)
- `rm_village`:
  - canonical source for village economy/buildings/production/research
  - canonical source for village CPP/day (`culture_points_production`)
  - village cumulative CP is not authoritative
  - `army`/`reinforcements`/`deployed_armies` fields are compatibility snapshots, not query authority

- `players`:
  - canonical source for cumulative culture points (`culture_points`)
  - `culture_points` advances from elapsed time using summed village CPP/day

Command-side rule:
- `VillageAggregate` remains fully aware of army data for invariants and domain behavior.
- Projectors materialize that state into read models with the ownership split above.

## Game World Data

- `rm_map_fields` is the canonical world map table.
- Each map field id is deterministic (`position.to_id(world_size)`), so `map_field_id`, `village_id`, and `oasis_id` remain aligned when applicable.
- Field occupancy is updated by domain actions (for example, village foundation on settlers arrival).

## Key Design Rules

- `parabellum_app` and domain crates do not depend on SQLx.
- Infrastructure-specific mapping stays in `parabellum_infra`.
- Domain/game rules live in `parabellum_game` whenever possible.
- No legacy UnitOfWork/job-handler path in the new ES flow.

## API Surface

- HTTP API is served under `/api/v1`.
- Contracts and error envelopes are documented in [`docs/api-contract-matrix.md`](docs/api-contract-matrix.md).
- Machine-readable contract entrypoint: `GET /api/v1/openapi.json`.
