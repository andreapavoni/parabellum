# Test Refactoring Tracker

This document tracks the progressive refactor of the test suite toward:

- convention over configuration
- canonical-read-model assertions
- predictable TDD-friendly fixtures
- minimal ad-hoc raw SQL in behavior tests

## Goals

1. Make tests read like game behavior specs, not storage implementation scripts.
2. Keep intentional low-level SQL only for corruption/recovery scenarios.
3. Standardize fixture entrypoints so new feature tests can be added quickly with minimal boilerplate.
4. Align assertions with architecture ownership contracts in `docs/ARCHITECTURE.md`.

## Passes

### Pass 1 (completed)

Scope:

- strengthen selected ES integration tests
- introduce canonical army read-model helpers in shared fixtures
- reduce direct SQL in test bodies where repository/service API exists
- make test DB cleanup explicit for CQRS tables

Completed:

- Added canonical troop-state fixture helpers in [fixtures.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/fixtures.rs): `home_units`, `stationed_units`, `deployed_units`.
- Added scheduled-action corruption and status-count fixture helpers in [fixtures.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/fixtures.rs).
- Extended table reset coverage in [fixtures.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/fixtures.rs) to include `rm_reports`, `rm_report_reads`, and `es_projector_offsets`.
- Converted selected scheduler assertions in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) from compatibility snapshots to canonical troop read-model checks.
- Converted one reinforcement flow test in [reinforcement.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/reinforcement.rs) to canonical troop read-model checks.
- Replaced direct scheduled-action SQL setup in selected tests with fixture helpers in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) and [replay.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/replay.rs).

Open in pass:

- none

### Pass 2 (planned)

Scope:

- centralize scenario setup as a predictable test DSL (`TestWorld`/`EsScenario`)
- remove duplicated local helpers (`troops_sum`, `army_units`, `hero_mansion`, etc.) where possible
- split intentional corruption helpers from normal happy-path fixture API

Target outcomes:

- one obvious fixture path for behavior tests
- one obvious fixture path for failure/corruption tests

Progress update:

- Added scenario helpers in [fixtures.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/fixtures.rs):
`process_due_until`, `refill_resources`, `research_and_complete`, `train_and_complete`.
- Migrated replay conquer window setup in [replay.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/replay.rs) to scenario helpers.
- Renamed replay test to reflect invariant:
`replay_full_mode_is_idempotent_for_attack_outcome_window`.
- Migrated repeated setup/queue-processing flows in [reports.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/reports.rs) to scenario helpers.
- Replaced one compatibility snapshot check in reports tests with canonical deployed-army read-model assertion.
- Migrated selected high-churn setup blocks in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) to scenario helpers (`train_and_complete`, `research_and_complete`, `process_due_until`, `refill_resources`).
- Replaced scout scheduler assertions from compatibility snapshots to canonical `home_units`/`deployed_units` helper checks.
- Migrated attack-arrival and prereq-blocked attack setup flows in scheduler tests to helper-based setup.
- Replaced additional attack-arrival assertions from `VillageModel` compatibility snapshots to canonical troop read-model helper checks.
- Removed remaining `scheduler.rs` local compatibility helper usage (`army_units` / `troops_sum`) by replacing assertions with canonical fixture helpers.

### Pass 3 (completed)

Scope:

- expand API contract tests in `parabellum_server/tests` based on `docs/api-contract-matrix.md`
- assert error envelope and ownership boundaries for representative endpoints

Target outcomes:

- server-level regression net for contract stability
- clearer separation between infra behavior tests and HTTP contract tests

Progress update:

- Added focused HTTP contract suite in [web_api_contract_test.rs](/Users/andrea/Code/Apps/parabellum/parabellum_server/tests/web_api_contract_test.rs) with stable assertions for:
  - auth validation envelopes (`422` + `field_errors`)
  - unauthorized contracts (`401`) for protected endpoints and invalid credentials/token flows
- Consolidated bearer unauthorized checks into the contract suite and removed overlapping legacy file `web_api_bearer_auth_test.rs`.
- Converted remaining `reinforcement.rs` local compatibility helpers (`troops_sum`, `army_units`) to explicit assertions and removed helper functions.
- Switched stable reinforcement assertions to canonical read-model helpers (`home_units`/`deployed_units`/`stationed_units`) where projections are deterministic.
- Kept transient partial-split checkpoints asserted via explicit aggregate projection (`get_village().deployed_armies/reinforcements`) where `rm_armies` state is not the immediate source of truth during recall/release transition boundaries.
- Converted replay idempotence assertions in [replay.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/replay.rs) from aggregate-list length checks to canonical troop-state counts (`stationed_units` / `deployed_units`) for deterministic before/after invariants.
- Converted scheduler wipeout checkpoint in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) from `deployed_armies.is_empty()` compatibility view to canonical deployed-unit assertions by unit type.
- Removed another compatibility snapshot assertion in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) (`get_village_army_state_view().home_army`) where canonical `home_units` assertions already cover the invariant.
- Normalized scheduler research setup in [scheduler.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/scheduler.rs) to single-snapshot village fetches per phase (stocks + tribe), reducing duplicated read-model calls and making fixtures more predictable.
- Added fixture-level ownership helper in [fixtures.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/fixtures.rs) (`village_owner`) and migrated conquer ownership assertions to helper-based checks.
- Added deterministic seeded-auth web test fixture in [test_utils.rs](/Users/andrea/Code/Apps/parabellum/parabellum_server/tests/test_utils.rs) (`setup_web_app_with_seeded_user`) and restored happy-path login/refresh/me contract assertions in [web_api_contract_test.rs](/Users/andrea/Code/Apps/parabellum/parabellum_server/tests/web_api_contract_test.rs).
- Fixed web contract test isolation bug by retaining schema handles for test lifetime (`_schema` binding) instead of dropping fixtures immediately via discard pattern (`_`).
- Removed fixture-level manual SQL enum mapping by adding `Display`/`FromStr` for `ScheduledActionStatus` in [models.rs](/Users/andrea/Code/Apps/parabellum/parabellum_app/villages/models.rs), and updated ES fixtures to bind status via trait conversion.
- Removed non-deterministic legacy conquer-positive scheduler test and kept deterministic conquer-negative coverage; scheduler suite now runs with no ignored tests.
- Converted remaining hero reinforcement snapshot assertions in [heroes.rs](/Users/andrea/Code/Apps/parabellum/parabellum_infra/es/tests/heroes.rs) to canonical helper checks (`deployed_units` / `stationed_units`) where deterministic.

Next in pass:

- none

## Constraints / notes

- `ScheduledActionStatus` (in `parabellum_app`) cannot directly implement SQLx encode/decode traits in `parabellum_infra` due to Rust orphan rules (both trait and type are external to `parabellum_infra`).
- Test enum SQL bindings now use `ScheduledActionStatus` textual traits (`Display`/`FromStr`) rather than per-test local mapping helpers.
