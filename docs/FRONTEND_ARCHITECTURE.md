# Parabellum Frontend Architecture

The frontend is a Preact SPA backed by the JSON API under `/api/v1`.

## State Boundaries

Frontend state is split into three categories:

- **Server state**: data fetched from the API and cached by TanStack Query.
- **Auth shell state**: the current token/session shell held in `AppStoreProvider`.
- **Local UI state**: form inputs, hover state, open/closed controls, previews, and transient errors.

Server state should not be copied into local component state unless it is being edited in a form.

## Query Runtime

TanStack Query is initialized in `frontend/src/query/client.ts` and mounted in
`frontend/src/main.tsx`.

Canonical query keys live in `frontend/src/query/keys.ts`.

Primary keys:

- `["session"]`
- `["gameContext"]`
- `["villageOverview", villageId]`
- `["villageResources", villageId]`
- `["building", slotId]`
- `["reports", page, perPage]`
- `["report", reportId]`
- `["mapRegion", x, y, villageId]`
- `["mapField", fieldId]`
- `["stats", page]`
- `["player", playerId]`

Read hooks live in `frontend/src/query/hooks.ts`.

## Game Context

`GET /api/v1/me/context` is the current game hydration endpoint.

It owns shell-level authenticated state:

- server time
- world size
- server speed
- unread reports count
- player summary
- current village summary
- village switcher list

Pages may fetch their own projections, but shared shell data should come from
the game context instead of page loaders.

## Mutations

State-changing POST requests are wrapped in mutation hooks in
`frontend/src/query/mutations.ts`.

Mutation hooks must invalidate the smallest practical set of query keys:

- building/construction/training/research commands invalidate `gameContext`,
  current village overview/resources, and the affected building.
- marketplace and troop movement commands also invalidate report-related data
  when they can create reports.
- village rename invalidates game context, current village projections, and map
  queries.
- founding a village invalidates game context and map queries.

Preview POST requests do not mutate server state and can remain direct API
calls.

## Timers

Client timers are prediction only. They never decide game outcomes.

Live derived hooks live under `frontend/src/live/`:

- `useServerClock` derives the visible server clock from the last server time.
- `useLiveResources` derives visible resources from stored amounts and
  production per hour.

When a server-owned timer elapses, components should invalidate the relevant
query keys and let the backend return the authoritative state.

## Realtime Roadmap

The current architecture is polling/refetch based. Future SSE or WebSocket
support should not replace page code directly.

Preferred shape:

1. Server sends compact invalidation events, for example `gameContext`,
   `village:{id}`, `building:{slotId}`, or `reports`.
2. A single realtime adapter maps those events to query invalidations.
3. Components continue to consume TanStack Query hooks.
