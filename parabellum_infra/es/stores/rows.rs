//! Typed rows for event-store persistence.

use chrono::{DateTime, Utc};
use mini_cqrs_es::{CqrsError, EventMetadata, StoredEvent};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(super) struct StoredEventRow {
    event_id: String,
    aggregate_type: String,
    aggregate_id: String,
    stream_version: i64,
    event_type: String,
    payload: serde_json::Value,
    metadata: serde_json::Value,
    global_seq: i64,
    occurred_at: DateTime<Utc>,
}

impl TryFrom<StoredEventRow> for StoredEvent {
    type Error = CqrsError;

    fn try_from(row: StoredEventRow) -> Result<Self, Self::Error> {
        let metadata: EventMetadata = serde_json::from_value(row.metadata)?;

        Ok(StoredEvent {
            id: row.event_id,
            aggregate_id: row.aggregate_id,
            aggregate_type: row.aggregate_type,
            version: row.stream_version as u64,
            event_type: row.event_type,
            payload: row.payload,
            metadata,
            global_sequence: Some(row.global_seq),
            timestamp: row.occurred_at,
        })
    }
}

#[derive(Debug, FromRow)]
pub(super) struct InsertedEventRow {
    pub global_seq: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct SnapshotRow {
    pub state: serde_json::Value,
    pub stream_version: i64,
}
