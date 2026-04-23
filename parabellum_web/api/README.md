# `parabellum_web::api` Notes

This module contains all JSON handlers exposed under `/api/v1`.

## Handler groups

- `auth.rs`: login/register/refresh/logout/session for bearer auth.
- `game.rs`: read-oriented endpoints used to bootstrap and navigate SPA views.
- `buildings.rs`: building-specific detail payloads.
- `actions.rs`: mutating commands (build, train, troops, marketplace, research).

## Shared support

- `dto.rs`: maps app/domain models into stable API payloads.
- `errors.rs`: normalized API error envelope with stable `code`.
- `helpers.rs`: bearer extraction + authenticated-user resolver.

## Conventions

- Keep handlers orchestration-only. No game rules here.
- Use `camelCase` wire fields.
- Map errors into explicit API codes (`unauthorized`, `token_expired`, `validation_error`, ...).
- Prefer endpoint-specific request/response structs over generic maps.
- Return canonical values, not rendered strings:
  - `createdAt` unix timestamps instead of preformatted datetime strings
  - `timeRemainingSecs`/`timeSeconds` instead of preformatted `HH:MM:SS`
  - report payload raw data; summaries/titles are rendered by frontend
  - unit/building fields carry enum keys (e.g. `MainBuilding`, `Legionnaire`) and frontend resolves labels/i18n
