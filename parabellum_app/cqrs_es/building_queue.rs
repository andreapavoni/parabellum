use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, RwLock},
};

use mini_cqrs_es::{
    Aggregate, Command, CqrsError, Event, EventConsumers, EventPayload, EventStore,
    SimpleAggregateManager, SimpleCqrs, Uuid,
};
use serde::{Deserialize, Serialize};

use parabellum_types::buildings::BuildingName;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildingQueueAggregate {
    aggregate_id: Uuid,
    version: u64,
    queued_levels_by_slot: HashMap<u8, u8>,
}

impl Aggregate for BuildingQueueAggregate {
    type Event = BuildingQueueEvent;

    async fn apply(&mut self, event: &Self::Event) {
        match event {
            BuildingQueueEvent::BuildingAdded { slot_id, target_level, .. }
            | BuildingQueueEvent::BuildingUpgraded {
                slot_id,
                target_level,
                ..
            } => {
                self.queued_levels_by_slot.insert(*slot_id, *target_level);
            }
            BuildingQueueEvent::BuildingDowngraded {
                slot_id,
                target_level,
                ..
            } => {
                if *target_level == 0 {
                    self.queued_levels_by_slot.remove(slot_id);
                } else {
                    self.queued_levels_by_slot.insert(*slot_id, *target_level);
                }
            }
        }
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn set_aggregate_id(&mut self, id: Uuid) {
        self.aggregate_id = id;
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn set_version(&mut self, version: u64) {
        self.version = version;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildingQueueEvent {
    BuildingAdded {
        slot_id: u8,
        name: BuildingName,
        target_level: u8,
    },
    BuildingUpgraded {
        slot_id: u8,
        name: BuildingName,
        target_level: u8,
    },
    BuildingDowngraded {
        slot_id: u8,
        name: BuildingName,
        target_level: u8,
    },
}

impl Display for BuildingQueueEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildingQueueEvent::BuildingAdded { .. } => write!(f, "BuildingAdded"),
            BuildingQueueEvent::BuildingUpgraded { .. } => write!(f, "BuildingUpgraded"),
            BuildingQueueEvent::BuildingDowngraded { .. } => write!(f, "BuildingDowngraded"),
        }
    }
}

impl EventPayload for BuildingQueueEvent {}

#[derive(Clone, Default)]
pub struct InMemoryBuildingQueueEventStore {
    events: Arc<RwLock<HashMap<Uuid, Vec<Event>>>>,
}

impl InMemoryBuildingQueueEventStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventStore for InMemoryBuildingQueueEventStore {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Event],
        expected_version: u64,
    ) -> Result<(), CqrsError> {
        let mut guard = self
            .events
            .write()
            .map_err(|_| CqrsError::EventStore("event store lock poisoned".to_string()))?;
        let stream = guard.entry(aggregate_id).or_default();
        let actual_version = stream.last().map(|event| event.version).unwrap_or(0);
        if actual_version != expected_version {
            return Err(CqrsError::Conflict {
                expected_version,
                actual_version,
            });
        }
        stream.extend_from_slice(events);
        Ok(())
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<(Vec<Event>, u64), CqrsError> {
        let guard = self
            .events
            .read()
            .map_err(|_| CqrsError::EventStore("event store lock poisoned".to_string()))?;
        let stream = guard.get(&aggregate_id).cloned().unwrap_or_default();
        let version = stream.last().map(|event| event.version).unwrap_or(0);
        Ok((stream, version))
    }
}

pub fn build_building_queue_cqrs(
    event_store: InMemoryBuildingQueueEventStore,
) -> SimpleCqrs<InMemoryBuildingQueueEventStore, SimpleAggregateManager<InMemoryBuildingQueueEventStore>>
{
    let aggregate_manager = SimpleAggregateManager::new(event_store.clone());
    let consumers = EventConsumers::new();
    SimpleCqrs::new(aggregate_manager, event_store, consumers)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAddBuildingCommand {
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for QueueAddBuildingCommand {
    type Aggregate = BuildingQueueAggregate;

    async fn handle(
        &self,
        aggregate: &Self::Aggregate,
    ) -> Result<Vec<<Self::Aggregate as Aggregate>::Event>, CqrsError> {
        if aggregate.queued_levels_by_slot.contains_key(&self.slot_id) {
            return Err(CqrsError::new(format!(
                "slot {} already has queued changes",
                self.slot_id
            )));
        }

        Ok(vec![BuildingQueueEvent::BuildingAdded {
            slot_id: self.slot_id,
            name: self.name.clone(),
            target_level: 1,
        }])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueUpgradeBuildingCommand {
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for QueueUpgradeBuildingCommand {
    type Aggregate = BuildingQueueAggregate;

    async fn handle(
        &self,
        aggregate: &Self::Aggregate,
    ) -> Result<Vec<<Self::Aggregate as Aggregate>::Event>, CqrsError> {
        let current_level = aggregate
            .queued_levels_by_slot
            .get(&self.slot_id)
            .copied()
            .unwrap_or(0);
        let target_level = current_level.saturating_add(1);

        Ok(vec![BuildingQueueEvent::BuildingUpgraded {
            slot_id: self.slot_id,
            name: self.name.clone(),
            target_level,
        }])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDowngradeBuildingCommand {
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for QueueDowngradeBuildingCommand {
    type Aggregate = BuildingQueueAggregate;

    async fn handle(
        &self,
        aggregate: &Self::Aggregate,
    ) -> Result<Vec<<Self::Aggregate as Aggregate>::Event>, CqrsError> {
        let current_level = aggregate
            .queued_levels_by_slot
            .get(&self.slot_id)
            .copied()
            .unwrap_or(0);
        if current_level == 0 {
            return Err(CqrsError::new(format!(
                "slot {} has no queued level to downgrade",
                self.slot_id
            )));
        }
        let target_level = current_level.saturating_sub(1);

        Ok(vec![BuildingQueueEvent::BuildingDowngraded {
            slot_id: self.slot_id,
            name: self.name.clone(),
            target_level,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_cqrs_es::Cqrs;
    use parabellum_types::buildings::BuildingName;

    #[tokio::test]
    async fn queue_add_building_emits_event_and_updates_aggregate() {
        let mut aggregate = BuildingQueueAggregate::default();
        aggregate.set_aggregate_id(Uuid::new_v4());

        let command = QueueAddBuildingCommand {
            slot_id: 19,
            name: BuildingName::MainBuilding,
        };
        let events = command
            .handle(&aggregate)
            .await
            .expect("add building command should emit event");
        assert_eq!(events.len(), 1);

        aggregate.apply(&events[0]).await;
        assert_eq!(aggregate.queued_levels_by_slot.get(&19), Some(&1));
    }

    #[tokio::test]
    async fn queue_downgrade_requires_existing_level() {
        let aggregate = BuildingQueueAggregate::default();
        let command = QueueDowngradeBuildingCommand {
            slot_id: 19,
            name: BuildingName::MainBuilding,
        };
        let result = command.handle(&aggregate).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cqrs_execute_persists_and_replays_building_queue_state() {
        let event_store = InMemoryBuildingQueueEventStore::new();
        let cqrs = build_building_queue_cqrs(event_store.clone());
        let aggregate_id = Uuid::new_v4();

        let add = QueueAddBuildingCommand {
            slot_id: 19,
            name: BuildingName::MainBuilding,
        };
        cqrs.execute(aggregate_id, &add)
            .await
            .expect("add should execute through cqrs");

        let upgrade = QueueUpgradeBuildingCommand {
            slot_id: 19,
            name: BuildingName::MainBuilding,
        };
        cqrs.execute(aggregate_id, &upgrade)
            .await
            .expect("upgrade should execute through cqrs");

        let (events, version) = event_store
            .load_events(aggregate_id)
            .await
            .expect("events should be stored");
        assert_eq!(events.len(), 2);
        assert_eq!(version, 2);

        let mut replayed = BuildingQueueAggregate::default();
        replayed.set_aggregate_id(aggregate_id);
        replayed
            .apply_events(&events)
            .await
            .expect("replay should apply stored events");
        assert_eq!(replayed.queued_levels_by_slot.get(&19), Some(&2));
    }
}
