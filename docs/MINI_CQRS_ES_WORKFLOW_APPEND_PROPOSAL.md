# mini_cqrs_es Proposal Prompt: Scheduled Workflows and Multi-Stream Append

This note is a reusable prompt/proposal for improving `mini_cqrs_es` to support cross-stream workflow facts as first-class behavior.

## Why

Current `Cqrs::execute` is aggregate-centric and appends to one stream per command.  
Complex workflows (battle, merchants, marketplace settlement, reinforcements) need:

- one domain decision producing multiple facts
- atomic append across multiple streams
- strict optimistic concurrency on each stream
- deterministic processing order for consumers

Today this is implemented ad-hoc in app infra. It should be a library-level pattern.

## Goals

1. Add a first-class multi-stream append API with per-stream expected versions.
2. Preserve current trait-first design and optional defaults.
3. Keep aggregate command path unchanged for simple cases.
4. Support deterministic event ordering for consumer dispatch.
5. Keep failure semantics explicit (`Conflict`, fail-fast, no partial append).

## Non-goals

- replacing existing `execute` flow
- enforcing one event style (big vs small fact) at library level
- introducing opinionated process manager framework in first iteration

## Proposed API Surface (Sketch)

```rust
pub struct WorkflowStreamAppend {
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub expected_version: u64,
    pub events: Vec<NewEvent>,
}

pub trait WorkflowEventStore: EventStore {
    async fn save_workflow_events(
        &self,
        streams: &[WorkflowStreamAppend],
    ) -> Result<Vec<StoredEvent>, CqrsError>;
}
```

Optional higher-level helper:

```rust
pub trait WorkflowCqrs: QueryRunner + Send + Sync {
    async fn execute_workflow(
        &self,
        streams: Vec<WorkflowStreamAppend>,
    ) -> Result<Vec<StoredEvent>, CqrsError>;
}
```

`execute_workflow` default semantics:

1. call `save_workflow_events`
2. dispatch resulting stored events to consumers in global-order
3. return stored events for caller-side orchestration/inspection

## Concurrency Semantics

- validate each stream expected version before inserts
- fail fast on first mismatch with `CqrsError::Conflict`
- all inserts in one transaction
- commit only if all stream appends are valid

## Ordering Semantics

Returned events should preserve deterministic order for consumer processing:

- preferred: DB `global_seq` ascending
- fallback if no global sequence: insertion order within `streams` list + in-stream order

## Metadata Semantics

Encourage workflow-level metadata:

- `correlation_id` shared across all workflow facts
- `causation_id` per step when cascading workflows
- optional `actor`/`tenant_id` propagation

## Backward Compatibility

- additive API only
- no change required for existing single-aggregate command handlers
- existing `EventStore` impls can ignore workflow trait until adopted

## Suggested Rollout

1. introduce `WorkflowEventStore` trait + docs
2. provide default implementation examples for SQL backends
3. add conformance tests:
   - all-or-nothing transaction
   - conflict behavior
   - ordering guarantees
4. add optional `WorkflowCqrs` helper
5. publish migration examples from single-stream orchestration to workflow append

## Open Questions for Library Design

1. Should `aggregate_type` be per stream (fully generic) or provided once for homogeneous workflows?
2. Should `save_workflow_events` return stream-version map in addition to `StoredEvent` list?
3. Should consumer dispatch be built into helper only, never event store trait?
4. Should snapshots be touched in workflow helper, or remain app-managed?

## Minimal Acceptance Criteria

1. Multi-stream append works with strict optimistic concurrency.
2. No partial writes on conflict.
3. Consumer dispatch order is deterministic.
4. Existing single-stream `execute` path remains unchanged.

## Current App-Side Extraction Seam

`parabellum_infra::es::village_service::VillageEsService` now isolates workflow fact preparation in a dedicated helper (`build_village_workflow_appends`).  
This seam is intentionally shaped so we can migrate the generic grouping/version-fetch/appender workflow into `mini_cqrs_es` without touching domain orchestration code.
