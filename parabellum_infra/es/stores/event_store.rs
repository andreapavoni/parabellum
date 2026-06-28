//! Postgres implementation of the append-only event store.

use mini_cqrs_es::{CqrsError, EventStore, NewEvent, StoredEvent};
use sqlx::{PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

use crate::EventStoreDb;

use super::rows::{InsertedEventRow, StoredEventRow};

#[derive(Clone, Debug)]
/// One stream append unit inside a workflow transaction.
///
/// `expected_version` is checked before inserts. Any mismatch aborts the whole
/// workflow append with `CqrsError::Conflict`.
pub struct WorkflowStreamAppend {
    pub aggregate_id: String,
    pub expected_version: u64,
    pub events: Vec<NewEvent>,
}

#[derive(Debug, Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(db: EventStoreDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn load_events_by_global_seq(
        &self,
        from_global_seq: i64,
        to_global_seq: Option<i64>,
        aggregate_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        let rows = sqlx::query_as::<_, StoredEventRow>(
            r#"
            SELECT event_id, aggregate_type, aggregate_id, stream_version, event_type, payload, metadata, global_seq, occurred_at
            FROM es_events
            WHERE global_seq >= $1
              AND ($2::BIGINT IS NULL OR global_seq <= $2)
              AND ($3::TEXT IS NULL OR aggregate_id = $3)
            ORDER BY global_seq ASC
            LIMIT $4
            "#,
        )
        .bind(from_global_seq)
        .bind(to_global_seq)
        .bind(aggregate_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(CqrsError::domain_source)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Atomically appends a workflow spanning multiple streams.
    ///
    /// Contract:
    /// - checks each stream `expected_version` inside the same DB transaction
    /// - returns `CqrsError::Conflict` on first mismatch
    /// - commits only if all streams are valid and all inserts succeed
    /// - never produces partial cross-stream writes
    pub async fn append_workflow_events(
        &self,
        aggregate_type: &str,
        streams: &[WorkflowStreamAppend],
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        let stored = self
            .append_workflow_events_in_tx(&mut tx, aggregate_type, streams)
            .await?;
        tx.commit().await.map_err(CqrsError::domain_source)?;
        Ok(stored)
    }

    pub async fn append_workflow_events_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        aggregate_type: &str,
        streams: &[WorkflowStreamAppend],
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        for stream in streams {
            assert_expected_version_in_tx(
                tx,
                aggregate_type,
                &stream.aggregate_id,
                stream.expected_version,
            )
            .await?;
        }

        let mut stored = Vec::new();
        for stream in streams {
            for (idx, event) in stream.events.iter().enumerate() {
                let version = stream.expected_version + idx as u64 + 1;
                stored.push(
                    insert_event_in_tx(
                        tx,
                        EventInsert {
                            aggregate_type,
                            aggregate_id: &stream.aggregate_id,
                            version,
                            event,
                        },
                    )
                    .await?,
                );
            }
        }
        Ok(stored)
    }
}

impl EventStore for PostgresEventStore {
    async fn save_events(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        events: &[NewEvent],
        expected_version: u64,
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        assert_expected_version_in_tx(&mut tx, aggregate_type, aggregate_id, expected_version)
            .await?;

        let mut stored = Vec::with_capacity(events.len());
        for (idx, event) in events.iter().enumerate() {
            let version = expected_version + idx as u64 + 1;
            stored.push(
                insert_event_in_tx(
                    &mut tx,
                    EventInsert {
                        aggregate_type,
                        aggregate_id,
                        version,
                        event,
                    },
                )
                .await?,
            );
        }

        tx.commit().await.map_err(CqrsError::domain_source)?;
        Ok(stored)
    }

    async fn load_events(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<(Vec<StoredEvent>, u64), CqrsError> {
        let rows = sqlx::query_as::<_, StoredEventRow>(
            r#"
            SELECT event_id, aggregate_type, aggregate_id, stream_version, event_type, payload, metadata, global_seq, occurred_at
            FROM es_events
            WHERE aggregate_type = $1 AND aggregate_id = $2
            ORDER BY stream_version ASC
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(CqrsError::domain_source)?;

        if rows.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let mut events = Vec::with_capacity(rows.len());
        let mut version = 0_u64;
        for row in rows {
            let event: StoredEvent = row.try_into()?;
            version = event.version;
            events.push(event);
        }

        Ok((events, version))
    }
}

struct EventInsert<'a> {
    aggregate_type: &'a str,
    aggregate_id: &'a str,
    version: u64,
    event: &'a NewEvent,
}

async fn assert_expected_version_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    aggregate_type: &str,
    aggregate_id: &str,
    expected_version: u64,
) -> Result<(), CqrsError> {
    let current_version: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(stream_version), 0)
        FROM es_events
        WHERE aggregate_type = $1 AND aggregate_id = $2
        "#,
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(CqrsError::domain_source)?;

    let actual_version = current_version as u64;
    if actual_version != expected_version {
        return Err(CqrsError::Conflict {
            expected_version,
            actual_version,
        });
    }
    Ok(())
}

async fn insert_event_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    insert: EventInsert<'_>,
) -> Result<StoredEvent, CqrsError> {
    let event_id = Uuid::new_v4().to_string();
    let row = sqlx::query_as::<_, InsertedEventRow>(
        r#"
        INSERT INTO es_events (
            event_id,
            aggregate_type,
            aggregate_id,
            stream_version,
            event_type,
            payload,
            metadata,
            occurred_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING global_seq, occurred_at
        "#,
    )
    .bind(&event_id)
    .bind(insert.aggregate_type)
    .bind(insert.aggregate_id)
    .bind(insert.version as i64)
    .bind(&insert.event.event_type)
    .bind(Json(&insert.event.payload))
    .bind(Json(&insert.event.metadata))
    .bind(insert.event.timestamp)
    .fetch_one(&mut **tx)
    .await
    .map_err(CqrsError::domain_source)?;

    Ok(StoredEvent {
        id: event_id,
        aggregate_id: insert.aggregate_id.to_string(),
        aggregate_type: insert.aggregate_type.to_string(),
        version: insert.version,
        event_type: insert.event.event_type.clone(),
        payload: insert.event.payload.clone(),
        metadata: insert.event.metadata.clone(),
        global_sequence: Some(row.global_seq),
        timestamp: row.occurred_at,
    })
}
