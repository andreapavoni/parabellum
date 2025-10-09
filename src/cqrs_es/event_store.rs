use anyhow::Error;
use async_trait::async_trait;
use mini_cqrs_es::{Event, EventStore as CqrsEventStore, Uuid};
use polodb_core::{bson::doc, Collection, CollectionT, Database};

// Event Store

pub struct EventStore {
    events: Collection<Event>,
}

impl EventStore {
    pub fn new(path: &str) -> Result<Self, Error> {
        let db = Database::open_path(path)?;
        let events: Collection<Event> = db.collection::<Event>("events");

        Ok(Self { events })
    }
}

#[async_trait]
impl CqrsEventStore for EventStore {
    async fn save_events(&mut self, _aggregate_id: Uuid, events: &[Event]) -> Result<(), Error> {
        self.events.insert_many(events)?;
        Ok(())
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<Event>, Error> {
        let uuid_str = aggregate_id.to_string();
        let res = self.events.find(doc! {"aggregate_id": uuid_str}).run()?;

        let events = res.into_iter().map(|e| e.unwrap()).collect();

        Ok(events)
    }
}
