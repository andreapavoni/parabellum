# Runtime review and improvement plan

Reviewed 2026-09-22 against the current working tree, including existing local changes.

This is an initial static review of the runtime, scheduler, event/snapshot stores,
projection recovery, and operational documentation. It is not a complete gameplay
or security audit. No testing-server telemetry or heap profile was available; no
runtime memory leak has been reproduced. Existing application files were not changed.

## Findings

### 1. High: workflow memory and CPU cost grow with full village history

Evidence: `parabellum_infra/es/village_service/workflow_append.rs:48-50` loads
all events merely to obtain the expected stream version. Lines 105-118 load the
entire stream again and replay it to rebuild a snapshot after every workflow.
`es/stores/event_store.rs:154-184` implements loading with an unbounded
`fetch_all`, followed by conversion to another event vector.

Training emits one completion per unit and usually another scheduling event
(`es/workflows/training.rs:95-123`). Long training queues continually lengthen
the history being fetched and replayed. Across N completions in a growing stream,
this can approach quadratic cumulative replay work. Snapshots currently do not
bound this workflow path.

This is confirmed allocation amplification, not proof of permanently leaked
objects. It is the strongest memory-growth hypothesis found in this pass;
allocator retention after progressively larger allocations could make RSS remain
high after work finishes.

Plan: add a scalar stream-version query; load aggregate state from a snapshot
plus a bounded event tail; update snapshots incrementally. Verify the actual
`mini_cqrs_es` loading contract before changing the store interface. Benchmark
identical workflows against 1k, 10k, and 100k historical events, recording rows
loaded, bytes decoded, latency, and peak/live heap.

### 2. High: workflow commit and queue completion are not atomic

Evidence: `es/village_service/workflow_append.rs:90-91` commits events and
projections, then refreshes snapshots. `es/village_service/scheduler.rs:97-109`
updates queue status later in a separate operation.

A crash after commit but before queue completion leaves a processing action
eligible for stale recovery. Executing it again appends fresh events. For training,
`es/workflows/training.rs:95-123` emits another unit and a fresh random continuation
ID, and `es/consumers/village_projector/training.rs:121-153` adds the unit without
deduplicating the action. A snapshot error after commit instead makes the scheduler
report an already-applied action as failed.

Plan: atomically persist action completion, canonical events, and projections;
enforce action identity at the append boundary. Treat snapshot repair as a separate
derived-state failure unless snapshots are included in the same transaction.
Classify transient database errors for bounded retry instead of terminally failing
every non-conflict error.

Acceptance tests: inject failure immediately before and after commit, during
snapshot persistence, and during status persistence. Recover/restart and assert
exactly one unit, one continuation, and one set of canonical effects. Existing
`es/tests/scheduler.rs:45` idempotency coverage only polls again after success.

### 3. High: advisory-lock ownership can survive an error or cancellation

Evidence: `es/advisory_lock.rs:11-34` acquires a session advisory lock on a pooled
connection and releases it only through an explicit async method. Scheduler
requeue/claim errors return through `?` before `lock.release()`
(`es/village_service/scheduler.rs:56-84`). The wrapper has no cancellation cleanup.

Returning that connection to the pool does not explicitly unlock its PostgreSQL
session. Other sessions can consequently skip scheduler work; reacquiring on the
same session can also stack lock acquisitions.

Plan: make lock ownership cancellation-safe, using a dedicated connection that
is closed if not explicitly unlocked, or a transaction-scoped design with a
well-defined lifetime. Do not attempt asynchronous unlocking casually in Drop.

Acceptance tests: fail requeue and claim after lock acquisition; cancel the task
while holding the lock; verify a separate connection can subsequently acquire it.
Repeat with a small pool to expose reuse and nested-acquisition behavior.

### 4. High: filtered full replay clears more state than it rebuilds

Evidence: `es/replay/runner.rs:87-105` resets the complete selected projection
before applying `from_global_seq`, `to_global_seq`, and `aggregate_id` filters.
The reset at lines 152-193 deletes whole tables. The CLI accepts these combinations
in `parabellum_server/bin/parabellum_replay.rs:80-152`.

A full replay for one aggregate removes unrelated villages or reports; a replay
starting midway through history may omit the facts needed to reconstruct state.
The reset commits before replay, so an error can leave a partial projection.

Plan: initially reject filtered/windowed destructive rebuilds unless a supported
baseline and dependency scope are supplied. Separate diagnostic replay from
rebuild. For robust maintenance, rebuild into staging tables, validate, and swap
atomically while gameplay writes are quiesced. The scheduler lock alone is not a
general maintenance barrier for HTTP commands.

Acceptance tests: create two villages with cross-village movements, attempt a
filtered rebuild, and verify rejection without mutation or correct dependency-aware
preservation. Inject a mid-replay error and verify the previous read model remains
available. Compare full-history reconstruction with live state across armies,
economy, heroes, marketplace, ownership, and reports.

### 5. Medium: delayed snapshot writes can overwrite newer snapshots

Evidence: `es/stores/snapshots.rs:35-42` unconditionally overwrites state and
stream version on conflict. Concurrent commands/workflow refreshes can finish
saving in reverse version order.

The confirmed defect is snapshot version regression. Whether a later command can
use stale state depends on the dependency's snapshot-loading/tail-reconciliation
behavior, which was not inspected in this pass.

Plan: enforce monotonically increasing snapshot versions in the upsert; verify
snapshot-plus-tail reconciliation against the event-store head. Add a deliberately
reordered concurrent snapshot-write test and command/workflow race tests.

### 6. Lower priority: operational and documentation gaps

- `parabellum_server/src/logs.rs:48` deliberately forgets the logging guard.
  Return it to the runtime owner for shutdown flushing. This fixed startup
  allocation does not explain sustained memory growth by itself.
- The scheduler's spawned task has no exposed join handle or shutdown path
  (`es/worker.rs:54-71`). Supervise it and drain/stop it during shutdown.
- No memory/queue metrics or soak-test harness were found in the searched runtime.
  Add queue lag, outcomes, event-load bytes, snapshot timings, pool utilization,
  request concurrency, and process memory measurements.
- No production cleanup for completed scheduled actions or expired/revoked refresh
  sessions was found in the searched code. Define retention and bounded cleanup;
  distinguish database growth from application heap growth. Preserve canonical
  event history and durable deduplication semantics.
- README advertises Rust 1.85; workspace metadata specifies 1.95.
- README's frontend architecture link points to the backend architecture document.
- Production Compose declares dependencies on undefined services `caddy` and
  `parabellum_db`; the database service key is `db`. Validate the documented
  standalone startup with `docker compose config` and a clean deployment smoke test.

## Delivery order

1. Establish a reproducible memory baseline and capture one representative profile.
   Add the crash/retry and lock-release regression tests while fixing findings 2–3.
2. Remove full-history loading from workflows and make snapshot persistence
   monotonic/recoverable. Demonstrate bounded work with growing-history benchmarks.
3. Restrict unsafe replay modes, then implement tested rebuild recovery semantics.
4. Add supervised shutdown, operational metrics, retention jobs, and soak tests.
5. Repair onboarding documentation and deployment smoke tests before expanding
   gameplay scope (oases, alliances, Town Hall, and other README roadmap items).

## Memory investigation protocol

First identify the deployed commit, process/container that grows, world size,
server speed, active villages, event counts, training backlog, and request volume.
Separate application RSS/anonymous memory from PostgreSQL memory and container
file cache. Record CPU, queue lag, database size, and restarts alongside memory.

Run reproducible scenarios against a disposable database:

1. Idle runtime with no due actions and no browser polling.
2. Browser/API polling without scheduled work.
3. Continuous training on fixed villages, at several preseeded history lengths.
4. Mixed attacks, merchants, training, and building work.
5. Stop load and observe live allocations, RSS, and remaining tasks during cooldown.

Take heap profiles before load, during growth, and after cooldown using a profiler
appropriate to the deployment OS. Inspect retained allocation stacks as well as
peak allocations. Stable live heap with elevated RSS suggests allocator retention;
growing retained objects requires tracing owners and lifetimes. Avoid declaring a
leak solely from container memory graphs.

Use a short repeatable benchmark in CI and a longer staging soak. Success means
bounded live heap for a fixed active workload, no workload-dependent accumulation
of tasks/connections, stable per-workflow cost as old event history grows, and no
duplicated effects across crash recovery. Set numeric budgets after measuring the
baseline rather than inventing thresholds.

## Validation performed

Used indxr discovery and targeted source reads, checked cited locations against
the current working tree, inspected scheduler behavior tests and CI configuration,
and searched for allocation/retention and observability patterns. No application
tests, server load tests, heap captures, or deployment commands were run. Findings
describe reachable code paths; production attribution still requires telemetry
and the focused reproductions above.
