# API Contract Matrix (v1)

This matrix documents the current JSON contract under `/api/v1` and the hardened error behavior.

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

- `GET /villages/{id}/overview`
- `GET /villages/{id}/resources`
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
