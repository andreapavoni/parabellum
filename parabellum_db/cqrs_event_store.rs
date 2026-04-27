use chrono::{DateTime, Utc};
use mini_cqrs_es::{CqrsError, Event, EventPayload, EventStore, Uuid};
use parabellum_app::repository::CqrsEventStoreRepository;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPayload(serde_json::Value);

impl Display for StoredPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "StoredPayload")
    }
}

impl EventPayload for StoredPayload {}

impl EventStore for PostgresEventStore {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Event],
        expected_version: u64,
    ) -> Result<(), CqrsError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let current_version: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(version), 0)
            FROM cqrs_events
            WHERE aggregate_id = $1
            "#,
        )
        .bind(aggregate_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let expected_version_i64 = i64::try_from(expected_version)
            .map_err(|_| CqrsError::EventStore("expected_version overflow".to_string()))?;
        if current_version != expected_version_i64 {
            return Err(CqrsError::Conflict {
                expected_version,
                actual_version: u64::try_from(current_version).unwrap_or(0),
            });
        }

        for event in events {
            let payload = event
                .get_payload::<StoredPayload>()
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;

            let version = i64::try_from(event.version)
                .map_err(|_| CqrsError::EventStore("event version overflow".to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO cqrs_events (
                    id, aggregate_id, event_type, payload, version, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&event.id)
            .bind(aggregate_id)
            .bind(&event.event_type)
            .bind(payload.0)
            .bind(version)
            .bind(event.timestamp)
            .execute(&mut *tx)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<(Vec<Event>, u64), CqrsError> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, payload, version, occurred_at
            FROM cqrs_events
            WHERE aggregate_id = $1
            ORDER BY version ASC
            "#,
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut events = Vec::with_capacity(rows.len());
        let mut latest_version: u64 = 0;

        for row in rows {
            let id: String = row.try_get("id").map_err(map_store_decode_error)?;
            let event_type: String = row.try_get("event_type").map_err(map_store_decode_error)?;
            let payload: serde_json::Value = row.try_get("payload").map_err(map_store_decode_error)?;
            let version: i64 = row.try_get("version").map_err(map_store_decode_error)?;
            let occurred_at: DateTime<Utc> = row.try_get("occurred_at").map_err(map_store_decode_error)?;

            let version_u64 = u64::try_from(version)
                .map_err(|_| CqrsError::EventStore(format!("invalid negative version: {version}")))?;

            let mut event = Event::new(aggregate_id, StoredPayload(payload), version_u64)?;
            event.id = id;
            event.event_type = event_type;
            event.timestamp = occurred_at;

            latest_version = latest_version.max(version_u64);
            events.push(event);
        }

        Ok((events, latest_version))
    }
}

#[async_trait::async_trait]
impl CqrsEventStoreRepository for PostgresEventStore {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Event],
        expected_version: u64,
    ) -> Result<(), CqrsError> {
        <Self as EventStore>::save_events(self, aggregate_id, events, expected_version).await
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<(Vec<Event>, u64), CqrsError> {
        <Self as EventStore>::load_events(self, aggregate_id).await
    }
}

fn map_store_decode_error(err: sqlx::Error) -> CqrsError {
    CqrsError::EventStore(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::establish_test_connection_pool;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestPayload {
        value: String,
    }

    impl Display for TestPayload {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestPayload")
        }
    }

    impl EventPayload for TestPayload {}

    #[tokio::test]
    async fn postgres_event_store_roundtrip_and_conflict() -> Result<(), CqrsError> {
        let pool = establish_test_connection_pool()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let store = PostgresEventStore::new(pool.clone());
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cqrs_events (
                id TEXT PRIMARY KEY,
                aggregate_id UUID NOT NULL,
                event_type TEXT NOT NULL,
                payload JSONB NOT NULL,
                version BIGINT NOT NULL,
                occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        sqlx::query("DELETE FROM cqrs_events")
            .execute(&pool)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let aggregate_id = Uuid::new_v4();
        let event = Event::new(
            aggregate_id,
            TestPayload {
                value: "queued".to_string(),
            },
            1,
        )?;

        mini_cqrs_es::EventStore::save_events(&store, aggregate_id, &[event.clone()], 0).await?;

        let (events, version) = mini_cqrs_es::EventStore::load_events(&store, aggregate_id).await?;
        assert_eq!(events.len(), 1);
        assert_eq!(version, 1);
        let payload = events[0].get_payload::<TestPayload>()?;
        assert_eq!(payload.value, "queued");

        let conflict = mini_cqrs_es::EventStore::save_events(&store, aggregate_id, &[event], 0).await;
        assert!(matches!(conflict, Err(CqrsError::Conflict { .. })));

        Ok(())
    }
}
