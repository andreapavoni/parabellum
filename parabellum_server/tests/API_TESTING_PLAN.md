# JSON API Test Improvements

This file tracks practical improvements for API integration tests after the SPA + JSON backend migration.

## Current baseline

- Existing integration tests validate core game workflows (`app_*_test.rs`).
- Auth integration tests now target bearer endpoints:
  - `web_player_authentication_test.rs`
  - `web_player_registration_test.rs`
- New bearer-gate regression tests:
  - `web_api_bearer_auth_test.rs` (`missing token` + `invalid token`).
- API payload contract moved toward raw data:
  - reports now expose `reportType`, `payload`, `createdAt` (no backend-rendered title/summary/date).
  - report detail exposes `createdAt` raw timestamp.
  - marketplace offers expose `createdAt` raw timestamp.

## Recommended improvements

1. Shared auth helper for tests (done)
- `login_tokens(client, email, password)` added in `test_utils`.
- Returns `{ access_token, refresh_token }`.
- Removes repeated login boilerplate.

2. Refresh lifecycle tests (partially done)
- Added `refresh rotation` test (`web_player_authentication_test.rs`):
  - old refresh token is rejected with `session_revoked`.
- Still missing:
  - `logout -> refresh denied`
  - explicit `refresh_expired` coverage.

3. Bearer-required negative tests (partially done)
- Done:
  - missing `Authorization` -> `401` + `unauthorized`
  - invalid bearer token -> `401` + `unauthorized`
- Missing:
  - expired access token -> `401` + `token_expired`

4. Add village-switch token rotation tests
- `POST /api/v1/village/current` with bearer returns rotated `accessToken`.
- New token changes effective current village.

5. Stabilize flaky integration tests
- A few battle/scout tests can fail due non-deterministic setup.
- Prefer deterministic tribe/setup fixtures and explicit assertions on setup state before command execution.

6. Add report payload contract tests
- Assert report list items include `reportType`, `payload`, `createdAt`, `isRead`.
- Assert no formatted fields (`title`, `summary`, `createdAtFormatted`) are present.

## CI recommendations

- Keep unit tests as-is.
- Split integration tests in two groups:
  - deterministic core (`app_*`)
  - potentially flaky/scenario-heavy (`hero/scout`) with retries at job level.
- Add a dedicated job for web auth integration tests with `timeout` guard and verbose logs,
  since `web_player_*` suites can hang in current setup.
