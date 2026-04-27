use mini_cqrs_es::{CqrsError, Event, Uuid};

#[async_trait::async_trait]
pub trait CqrsEventStoreRepository: Send + Sync {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Event],
        expected_version: u64,
    ) -> Result<(), CqrsError>;

    async fn load_events(&self, aggregate_id: Uuid) -> Result<(Vec<Event>, u64), CqrsError>;
}
