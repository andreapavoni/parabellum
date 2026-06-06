# API Contract Matrix (v1)

This matrix documents the current JSON contract under `/api/v1` and the hardened error behavior.

Related conventions:

- [`TESTING_AND_ERROR_CONVENTIONS.md`](TESTING_AND_ERROR_CONVENTIONS.md)

## Resolution Order (Extractor-First)

For handlers using path/JSON extractors, request parsing happens before auth checks.

- malformed path/body can return `400`/`422` even without a bearer token
- structurally valid requests with missing/invalid bearer token return `401`

## OpenAPI

- `GET /openapi.json`
  - `200` OpenAPI 3.1 document

## Auth

- `POST /auth/token/login`
  - `200` token payload
  - `401` invalid credentials
  - `422` missing `email`/`password`
- `POST /auth/token/register`
  - `200` token payload
  - `409` duplicate email
  - `422` missing required fields / invalid password
- `POST /auth/refresh`
  - `200` token payload
  - `401` invalid/expired/revoked refresh token
  - `422` missing `refresh_token`
- `POST /auth/token/logout`
  - `200` `{ "success": true }`
  - `401` invalid token / auth required
  - `422` missing `refresh_token`

## Player/Session

- `GET /me/session`
- `GET /me/context`
- `POST /me/village/current`
- `GET /players/{id}`
- `GET /stats`

All return:
- `200` success payload
- `401` missing/invalid bearer token
- `404` only for missing target resources (player/village/report)

## Villages / Buildings / Actions

- `GET /game/context`
- `GET /buildings/{slot_id}`
- `POST /buildings/add`
- `POST /buildings/upgrade`
- `POST /academy/research`
- `POST /smithy/research`
- `POST /army/train`
- `POST /army/send`
- `POST /army/recall`
- `POST /army/release`
- `POST /map/found-village`

Common behavior:
- `200` action ack or detail payload
- `401` missing/invalid bearer token
- `400` invalid path parameter shape (for typed route params)
- `404` village/army/target resource not available for current player
- `409` queue conflicts
- `422` domain validation failures

## Marketplace

- `POST /marketplace/send`
- `POST /marketplace/offers`
- `POST /marketplace/offers/{offer_id}/accept`
- `POST /marketplace/offers/{offer_id}/cancel`

Common behavior:
- `200` action ack payload
- `401` missing/invalid bearer token
- `400` invalid path parameter shape (e.g. malformed `offer_id`)
- `404` missing offer/village
- `409` queue or offer state conflicts
- `422` domain validation failures

## Map / Reports

- `GET /map/region`
- `GET /map/fields/{id}`
- `GET /reports`
- `GET /reports/{id}`

Common behavior:
- `200` success payload
- `401` missing/invalid bearer token
- `400` invalid path parameter shape
- `404` missing field/report

## Error Envelope

All API errors use:

```json
{
  "code": "string",
  "message": "string",
  "field_errors": {
    "field": "reason"
  }
}
```

`field_errors` is optional.

## Contract Invariants (Enforced Tests)

Primary enforcement lives in:

- `parabellum_server/tests/web_api_contract_test.rs`

Representative invariants covered by executable tests:

- auth happy-path and token lifecycle:
  - login/refresh/me context contracts
- auth validation and unauthorized envelopes:
  - `422 validation_error` for missing fields
  - `401 unauthorized` for invalid credentials/tokens
- OpenAPI availability:
  - `/api/v1/openapi.json` returns `200` with OpenAPI document shape
- protected route baseline:
  - representative protected endpoints return `401` for structurally valid unauthenticated requests
- villages/contracts:
  - unknown village overview id returns `404 not_found` (authenticated request)
  - unknown village resources id returns `404 not_found` (authenticated request)
- report contracts:
  - unknown report id returns `404 not_found`
  - malformed report id path returns extractor `400` before auth
- map contracts:
  - unknown field id returns `404 not_found`
  - partial coordinate query returns `400 bad_request`
- marketplace contracts:
  - unknown offer accept target returns `404 not_found`
  - unknown offer cancel target returns `404 not_found`
- action validation:
  - invalid training quantity returns `422 validation_error`
- marketplace transition conflict:
  - owner accepting own offer returns `409 conflict`
