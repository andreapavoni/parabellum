//! Postgres implementation of aggregate snapshot storage.

use mini_cqrs_es::{Aggregate, AggregateSnapshot, CqrsError, SnapshotStore};
use sqlx::{PgPool, types::Json};

use crate::EventStoreDb;

use super::rows::SnapshotRow;

#[derive(Debug, Clone)]
pub struct PostgresSnapshotStore {
    pool: PgPool,
}

impl PostgresSnapshotStore {
    pub fn new(db: EventStoreDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
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
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (aggregate_type, aggregate_id)
            DO UPDATE SET
                stream_version = EXCLUDED.stream_version,
                state = EXCLUDED.state,
                updated_at = NOW()
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(snapshot.version as i64)
        .bind(Json(&state))
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

        let row = sqlx::query_as::<_, SnapshotRow>(
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

        let row = row.ok_or_else(|| {
            CqrsError::SnapshotStore(format!("snapshot not found for aggregate `{aggregate_id}`"))
        })?;
        let version = row.stream_version as u64;
        let mut aggregate: T = serde_json::from_value(row.state)?;
        aggregate.set_aggregate_id(aggregate_id.clone());
        aggregate.set_version(version);

        AggregateSnapshot::new(&aggregate, Some(version))
    }
}
