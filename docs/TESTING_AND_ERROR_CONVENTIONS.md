# Testing and Error Conventions

This document defines the current, target conventions for tests and error handling across ES/infra and HTTP/API layers.

## Goals

- Prefer behavior-driven assertions over implementation guardrails.
- Keep setup predictable (`convention over configuration`) so TDD stays cheap.
- Map known infra/domain failures to typed `ApplicationError` variants, not opaque `Unknown`.
- Keep web contracts stable and enforced by executable tests.

## Testing Conventions

### 1) ES/Infra tests are behavior tests

Scope: `parabellum_infra/es/tests`.

Default shape:

1. Build service + `EsScenario`.
2. Build actors/villages with fixtures (`village`, `village_for_player`).
3. Use shared helpers for repetitive setup (`refill_resources`, training/research helpers, queue helpers).
4. Drive time with bounded due windows (`process_until` / due timestamps near action windows).
5. Assert via canonical read-model helpers:
   - `home_units`
   - `stationed_units`
   - `deployed_units`
   - `village_owner`

Policy:

- Prefer assertions on observable outcomes (owner, stocks, movement state, queue status).
- Avoid aggregate-shape checks (`len`, `is_empty`) when canonical assertions express intent.
- Do not build per-test custom fixtures unless reused meaningfully.

### Isolated PostgreSQL fixtures

`parabellum_infra::test_support::IsolatedTestDatabase` (the `test-utils` feature)
is shared by infrastructure and API tests. It creates a UUID-named database,
applies all migrations, and drops that database when its owner is released,
including during panic unwinding. Keep the owner alive as long as the scenario.
ES fixtures additionally bootstrap the world and create villages through commands.
No fixture resets the database named in `TEST_DATABASE_URL`.

The role in `TEST_DATABASE_URL` must have `CREATEDB`; the local Compose role and
CI PostgreSQL role do. The URL may come from the process environment or `.env`.
Each process permits four live fixture databases, each with at most five pooled
connections. Separate processes use distinct databases and advisory-lock namespaces.
Abrupt process termination (for example SIGKILL) cannot run Rust cleanup; orphaned
`parabellum_test_<uuid>` databases may then need manual removal.

Run the workspace with:

```sh
PARABELLUM_SERVER_SPEED=1 cargo test --workspace
```

Fixtures do not mutate server speed. Some legacy runtime paths still read
`Config::from_env`, so the ES suite requires speed 1 at process launch; injecting
settings into those remaining paths is a separate follow-up.

### 2) Allowed low-level tests

Raw SQL/manual setup is still valid when SQL state is the contract under test:

- replay/corruption recovery boundaries
- scheduler locking/requeue behavior
- projector table-level invariants

If low-level setup is required, keep assertions focused on the exact invariant being protected.

### 3) HTTP/API tests validate contracts, not internals

Scope: `parabellum_server/tests/web_api_contract_test.rs`.

HTTP tests must enforce:

- status code correctness
- stable error envelope shape (`code`, `message`, optional `field_errors`)
- extractor-first semantics (parser errors can precede auth)
- auth/ownership boundaries

When behavior is already tested in ES, HTTP tests assert the resulting contract only.

## Error Mapping Conventions

## 1) Typed-first mapping

At adapter boundaries:

- commands and queries share `adapters/cqrs_errors::map_cqrs_error`
- projector and CQRS query failures preserve sources with `CqrsError::domain_source`
- optimistic version conflicts use `AppError::OptimisticConflict` (`409`)
- optional report context skips only an explicit `DbError::VillageNotFound`; SQL failures propagate

Known cases should map to typed variants (`Game`, `Db`, `App`) consumed by web error mapping.

## 2) `Unknown` is fallback only

`ApplicationError::Unknown` is acceptable only for:

- true unexpected failures
- explicit temporary guard branches not yet represented by a typed error

It must not be the default wrapper for `.map_err(|e| ...)` in adapters/ports.

## 3) Contract coupling

API-level status codes depend on typed errors:

- `Db::*NotFound` -> `404`
- conflict-like app/domain errors -> `409`
- domain validation -> `422`
- auth failures -> `401`

If mapping regresses to opaque `Unknown`, web layer drifts toward `500` and breaks contract stability.

## Maintenance Rules

- When adding new command/query paths, map errors through canonical mappers by default.
- If a new domain/infra failure repeats, introduce/extend typed variant mapping instead of adding ad-hoc string wrappers.
- Keep contract coverage updated in `web_api_contract_test`.
- Keep [api-contract-matrix.md](api-contract-matrix.md) aligned with executable tests.

