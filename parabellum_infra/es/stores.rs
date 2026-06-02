use chrono::Utc;
use mini_cqrs_es::{
    Aggregate, AggregateSnapshot, CqrsError, EventMetadata, EventStore, NewEvent, SnapshotStore,
    StoredEvent,
};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

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
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_events_by_global_seq(
        &self,
        from_global_seq: i64,
        to_global_seq: Option<i64>,
        aggregate_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        let rows = sqlx::query(
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

        rows.into_iter().map(row_to_stored_event).collect()
    }

    /// Atomically appends a workflow spanning multiple streams.
    ///
    /// Contract:
    /// - checks each stream `expected_version` inside the same DB transaction
    /// - returns `CqrsError::Conflict` on first mismatch (fail fast)
    /// - commits only if all streams are valid and all inserts succeed
    /// - never produces partial cross-stream writes
    pub async fn append_workflow_events(
        &self,
        aggregate_type: &str,
        streams: &[WorkflowStreamAppend],
    ) -> Result<Vec<StoredEvent>, CqrsError> {
        // Workflow boundary: all stream version checks and all inserts happen in
        // one DB transaction. We fail fast on the first conflict and commit only
        // if every stream append is valid.
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
            let current_version: i64 = sqlx::query_scalar(
                r#"
                SELECT COALESCE(MAX(stream_version), 0)
                FROM es_events
                WHERE aggregate_type = $1 AND aggregate_id = $2
                "#,
            )
            .bind(aggregate_type)
            .bind(&stream.aggregate_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(CqrsError::domain_source)?;

            let actual_version = current_version as u64;
            if actual_version != stream.expected_version {
                return Err(CqrsError::Conflict {
                    expected_version: stream.expected_version,
                    actual_version,
                });
            }
        }

        let mut stored = Vec::new();
        for stream in streams {
            for (idx, event) in stream.events.iter().enumerate() {
                let version = stream.expected_version + idx as u64 + 1;
                let event_id = Uuid::new_v4().to_string();
                let row = sqlx::query(
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
                .bind(aggregate_type)
                .bind(&stream.aggregate_id)
                .bind(version as i64)
                .bind(&event.event_type)
                .bind(Json(&event.payload))
                .bind(Json(&event.metadata))
                .bind(event.timestamp)
                .fetch_one(&mut **tx)
                .await
                .map_err(CqrsError::domain_source)?;

                stored.push(StoredEvent {
                    id: event_id,
                    aggregate_id: stream.aggregate_id.clone(),
                    aggregate_type: aggregate_type.to_string(),
                    version,
                    event_type: event.event_type.clone(),
                    payload: event.payload.clone(),
                    metadata: event.metadata.clone(),
                    global_sequence: Some(row.get::<i64, _>("global_seq")),
                    timestamp: row.get("occurred_at"),
                });
            }
        }
        Ok(stored)
    }
}

fn row_to_stored_event(row: PgRow) -> Result<StoredEvent, CqrsError> {
    let metadata_value = row
        .try_get::<serde_json::Value, _>("metadata")
        .map_err(CqrsError::domain_source)?;
    let metadata: EventMetadata = serde_json::from_value(metadata_value)?;
    let stream_version = row
        .try_get::<i64, _>("stream_version")
        .map_err(CqrsError::domain_source)? as u64;

    Ok(StoredEvent {
        id: row.try_get("event_id").map_err(CqrsError::domain_source)?,
        aggregate_id: row
            .try_get("aggregate_id")
            .map_err(CqrsError::domain_source)?,
        aggregate_type: row
            .try_get("aggregate_type")
            .map_err(CqrsError::domain_source)?,
        version: stream_version,
        event_type: row
            .try_get("event_type")
            .map_err(CqrsError::domain_source)?,
        payload: row
            .try_get::<serde_json::Value, _>("payload")
            .map_err(CqrsError::domain_source)?,
        metadata,
        global_sequence: Some(
            row.try_get("global_seq")
                .map_err(CqrsError::domain_source)?,
        ),
        timestamp: row
            .try_get("occurred_at")
            .map_err(CqrsError::domain_source)?,
    })
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

        let current_version: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(stream_version), 0)
            FROM es_events
            WHERE aggregate_type = $1 AND aggregate_id = $2
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(CqrsError::domain_source)?;

        let actual_version = current_version as u64;
        if actual_version != expected_version {
            return Err(CqrsError::Conflict {
                expected_version,
                actual_version,
            });
        }

        let mut stored = Vec::with_capacity(events.len());
        for (idx, event) in events.iter().enumerate() {
            let version = expected_version + idx as u64 + 1;
            let event_id = Uuid::new_v4().to_string();
            let row = sqlx::query(
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
            .bind(aggregate_type)
            .bind(aggregate_id)
            .bind(version as i64)
            .bind(&event.event_type)
            .bind(Json(&event.payload))
            .bind(Json(&event.metadata))
            .bind(event.timestamp)
            .fetch_one(&mut *tx)
            .await
            .map_err(CqrsError::domain_source)?;

            stored.push(StoredEvent {
                id: event_id,
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: aggregate_type.to_string(),
                version,
                event_type: event.event_type.clone(),
                payload: event.payload.clone(),
                metadata: event.metadata.clone(),
                global_sequence: Some(row.get::<i64, _>("global_seq")),
                timestamp: row.get("occurred_at"),
            });
        }

        tx.commit().await.map_err(CqrsError::domain_source)?;

        Ok(stored)
    }

    async fn load_events(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<(Vec<StoredEvent>, u64), CqrsError> {
        let rows = sqlx::query(
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
            let event = row_to_stored_event(row)?;
            version = event.version;
            events.push(event);
        }

        Ok((events, version))
    }
}

#[derive(Debug, Clone)]
pub struct PostgresSnapshotStore {
    pool: PgPool,
}

impl PostgresSnapshotStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SnapshotStore for PostgresSnapshotStore {
    async fn save_snapshot<T>(&self, snapshot: AggregateSnapshot<T>) -> Result<(), CqrsError>
    where
        T: Aggregate,
    {
        let aggregate = snapshot.get_payload::<T>()?;
        let aggregate_type = std::any::type_name::<T>();
        let aggregate_id = snapshot.aggregate_id.to_string();
        let state = serde_json::to_value(aggregate)?;

        sqlx::query(
            r#"
            INSERT INTO es_snapshots (aggregate_type, aggregate_id, stream_version, state, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (aggregate_type, aggregate_id)
            DO UPDATE SET
                stream_version = EXCLUDED.stream_version,
                state = EXCLUDED.state,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(snapshot.version as i64)
        .bind(Json(&state))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| CqrsError::SnapshotStore(e.to_string()))?;

        Ok(())
    }

    async fn load_snapshot<T>(
        &self,
        aggregate_id: &T::Id,
    ) -> Result<AggregateSnapshot<T>, CqrsError>
    where
        T: Aggregate,
    {
        let aggregate_type = std::any::type_name::<T>();
        let aggregate_id_str = aggregate_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT state, stream_version
            FROM es_snapshots
            WHERE aggregate_type = $1 AND aggregate_id = $2
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CqrsError::SnapshotStore(e.to_string()))?;

        let row = row.ok_or_else(|| CqrsError::AggregateNotFound(aggregate_id.to_string()))?;
        let version = row
            .try_get::<i64, _>("stream_version")
            .map_err(|e| CqrsError::SnapshotStore(e.to_string()))? as u64;
        let mut aggregate: T = serde_json::from_value(
            row.try_get::<serde_json::Value, _>("state")
                .map_err(|e| CqrsError::SnapshotStore(e.to_string()))?,
        )?;
        aggregate.set_aggregate_id(aggregate_id.clone());
        aggregate.set_version(version);

        AggregateSnapshot::new(&aggregate, Some(version))
    }
}
