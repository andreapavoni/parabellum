//! Bounded snapshot-plus-tail loading shared by commands and workflows.
use mini_cqrs_es::{Aggregate, CqrsError, StoredEvent};
use sqlx::{Postgres, Transaction};

use super::{
    PostgresEventStore,
    rows::{SnapshotRow, StoredEventRow},
};

const EVENT_PAGE_SIZE: i64 = 256;

impl PostgresEventStore {
    pub(crate) async fn load_aggregate_in_tx<A: Aggregate>(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &A::Id,
    ) -> Result<A, CqrsError> {
        let started = std::time::Instant::now();
        let aggregate_type = std::any::type_name::<A>();
        let aggregate_id = id.to_string();
        let head: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(stream_version), 0) FROM es_events
            WHERE aggregate_type = $1 AND aggregate_id = $2
            "#,
        )
        .bind(aggregate_type)
        .bind(&aggregate_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(CqrsError::domain_source)?;
        let snapshot = sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT state, stream_version FROM es_snapshots
            WHERE aggregate_type = $1 AND aggregate_id = $2 AND stream_version <= $3
            "#,
        )
        .bind(aggregate_type)
        .bind(&aggregate_id)
        .bind(head)
        .fetch_optional(&mut **tx)
        .await
        .map_err(CqrsError::domain_source)?;
        let mut aggregate = A::default();
        let mut version = 0;
        if let Some(snapshot) = snapshot {
            aggregate = serde_json::from_value(snapshot.state)?;
            version = snapshot.stream_version;
        }
        aggregate.set_aggregate_id(id.clone());
        let snapshot_version = version;
        while version < head {
            let rows = sqlx::query_as::<_, StoredEventRow>(
                r#"
                SELECT event_id, aggregate_type, aggregate_id, stream_version,
                       event_type, payload, metadata, global_seq, occurred_at
                FROM es_events
                WHERE aggregate_type = $1 AND aggregate_id = $2
                  AND stream_version > $3 AND stream_version <= $4
                ORDER BY stream_version LIMIT $5
                "#,
            )
            .bind(aggregate_type)
            .bind(&aggregate_id)
            .bind(version)
            .bind(head)
            .bind(EVENT_PAGE_SIZE)
            .fetch_all(&mut **tx)
            .await
            .map_err(CqrsError::domain_source)?;
            if rows.is_empty() {
                return Err(CqrsError::EventStore(
                    "incomplete aggregate event stream".into(),
                ));
            }
            for row in rows {
                let event: StoredEvent = row.try_into()?;
                if event.version != version as u64 + 1 {
                    return Err(CqrsError::EventStore(
                        "non-contiguous aggregate event stream".into(),
                    ));
                }
                aggregate.apply_events(std::slice::from_ref(&event)).await?;
                version = event.version as i64;
            }
        }
        aggregate.set_version(head as u64);
        tracing::debug!(
            aggregate_id,
            snapshot_version,
            stream_version = head,
            events_loaded = head - snapshot_version,
            elapsed_ms = started.elapsed().as_millis(),
            "aggregate loaded from snapshot and event tail"
        );
        Ok(aggregate)
    }
}
