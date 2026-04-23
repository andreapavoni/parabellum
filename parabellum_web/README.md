# `parabellum_web` Guide

This crate is the HTTP delivery layer for Parabellum:

- Serves the SPA shell and static assets.
- Exposes JSON API endpoints under `/api/v1/*`.
- Authenticates API requests with bearer access tokens.
- Maps web payloads to application commands/queries (`parabellum_app`).

## High-level architecture

- Router and state: [http.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/http.rs)
- API handlers:
  - Auth: [api/auth.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/auth.rs)
  - Game pages/data: [api/game.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/game.rs)
  - Mutations/actions: [api/actions.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/actions.rs)
  - Building detail API: [api/buildings.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/buildings.rs)
- Shared API support:
  - Error model: [api/errors.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/errors.rs)
  - DTO mapping: [api/dto.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/dto.rs)
  - Auth extraction helper: [api/helpers.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/helpers.rs)
- Token subsystem:
  - Token issue/verify/refresh-session persistence: [auth_tokens.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/auth_tokens.rs)
  - Counters/telemetry helpers: [auth_metrics.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/auth_metrics.rs)
- SPA shell:
  - Handler: [web/spa.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/web/spa.rs)
  - Template: [templates/spa_shell.html](/Users/andrea/Code/Apps/parabellum/parabellum_web/templates/spa_shell.html)

## Request lifecycle

1. `WebRouter` receives request.
2. `/assets` and `/static` are served directly from filesystem.
3. `/api/v1/*` goes to API handlers.
4. Authenticated handlers call `authenticated_user(...)`:
   - reads `Authorization: Bearer ...`
   - validates access token signature/expiry
   - validates refresh session (not expired/revoked)
   - loads user/player/current village context from app queries
5. Handler executes app command/query via `AppBus`.
6. Handler returns typed JSON DTO or `ApiError`.

## Auth contract (current)

- Login: `POST /api/v1/auth/token/login`
- Register: `POST /api/v1/auth/token/register`
- Refresh: `POST /api/v1/auth/refresh`
- Logout: `POST /api/v1/auth/token/logout`
- Session check: `GET /api/v1/auth/token/session`

Access tokens are JWT (HMAC), short-lived.
Refresh tokens are opaque values, hashed in DB (`auth_refresh_sessions`).

## Adding a new endpoint

1. Add request/response structs in the target handler file.
2. Route it in [http.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/http.rs) under `/api/v1`.
3. Use `authenticated_user(...)` for protected endpoints.
4. Map domain/app errors to `ApiError` with stable `code` values.
5. Add/extend integration tests in `parabellum_server/tests`.

## API handler conventions

- Keep handlers thin: validate payload, call app bus, map response.
- Avoid domain logic in this crate; put game rules in `parabellum_game`/`parabellum_app`.
- Keep wire names in `camelCase` (`#[serde(rename_all = "camelCase")]`).
- Use deterministic errors for frontend handling (`unauthorized`, `token_expired`, etc).

## Payload contract (backend vs frontend)

- Backend returns canonical game/application data.
- Frontend owns display formatting (date/time strings, relative times, localized labels, report summaries, emoji-rich text).
- Backend may return temporary compatibility display fields during migrations, but new endpoints/fields should default to raw values:
  - timestamps as unix seconds
  - enum keys/identifiers instead of rendered labels
  - numeric durations (`*_secs`) instead of formatted `HH:MM:SS`

## Build and frontend integration

`parabellum_web/build.rs` triggers `bun run build:release` during Cargo builds (unless `SKIP_FRONTEND` is set).

The resulting frontend bundle is expected at:
- `frontend/assets/*` (built files)
- `frontend/static/*` (static runtime files)
