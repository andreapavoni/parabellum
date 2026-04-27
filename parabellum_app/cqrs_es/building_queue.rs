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
use crate::cqrs_es::building_queue_read_model::BuildingQueueReadModel;
use crate::jobs::{
    Job,
    tasks::{AddBuildingTask, BuildingUpgradeTask},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildingQueueAggregate {
    aggregate_id: Uuid,
    version: u64,
    queued_names_by_slot: HashMap<u8, BuildingName>,
    queued_levels_by_slot: HashMap<u8, u8>,
}

impl Aggregate for BuildingQueueAggregate {
    type Event = BuildingQueueEvent;

    async fn apply(&mut self, event: &Self::Event) {
        match event {
            BuildingQueueEvent::BuildingAdded {
                slot_id,
                name,
                target_level,
            }
            | BuildingQueueEvent::BuildingUpgraded {
                slot_id,
                name,
                target_level,
                ..
            } => {
                self.queued_names_by_slot.insert(*slot_id, name.clone());
                self.queued_levels_by_slot.insert(*slot_id, *target_level);
            }
            BuildingQueueEvent::BuildingDowngraded {
                slot_id,
                name,
                target_level,
                ..
            } => {
                if *target_level == 0 {
                    self.queued_names_by_slot.remove(slot_id);
                    self.queued_levels_by_slot.remove(slot_id);
                } else {
                    self.queued_names_by_slot.insert(*slot_id, name.clone());
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

impl BuildingQueueAggregate {
    pub fn from_building_jobs(jobs: &[Job]) -> Self {
        let mut aggregate = Self::default();
        for job in jobs {
            match job.task.task_type.as_str() {
                "AddBuilding" => {
                    let Ok(payload) = serde_json::from_value::<AddBuildingTask>(job.task.data.clone()) else {
                        continue;
                    };
                    aggregate
                        .queued_names_by_slot
                        .entry(payload.slot_id)
                        .or_insert(payload.name.clone());
                    aggregate
                        .queued_levels_by_slot
                        .entry(payload.slot_id)
                        .and_modify(|level| *level = (*level).max(1))
                        .or_insert(1);
                }
                "BuildingUpgrade" => {
                    let Ok(payload) =
                        serde_json::from_value::<BuildingUpgradeTask>(job.task.data.clone())
                    else {
                        continue;
                    };
                    aggregate
                        .queued_names_by_slot
                        .entry(payload.slot_id)
                        .or_insert(payload.building_name.clone());
                    aggregate
                        .queued_levels_by_slot
                        .entry(payload.slot_id)
                        .and_modify(|level| *level = (*level).max(payload.level))
                        .or_insert(payload.level);
                }
                _ => {}
            }
        }
        aggregate
    }

    pub fn queued_level_for_slot(&self, slot_id: u8) -> Option<u8> {
        self.queued_levels_by_slot.get(&slot_id).copied()
    }

    pub fn queued_name_for_slot(&self, slot_id: u8) -> Option<BuildingName> {
        self.queued_names_by_slot.get(&slot_id).cloned()
    }

    pub fn queued_state_for_slot(&self, slot_id: u8) -> Option<(BuildingName, u8)> {
        let name = self.queued_name_for_slot(slot_id)?;
        let level = self.queued_level_for_slot(slot_id)?;
        Some((name, level))
    }

    pub fn queued_building_names(&self) -> Vec<BuildingName> {
        self.queued_names_by_slot.values().cloned().collect()
    }
}

pub async fn execute_add_command(
    aggregate: &BuildingQueueAggregate,
    slot_id: u8,
    name: BuildingName,
) -> Result<(), CqrsError> {
    let command = QueueAddBuildingCommand { slot_id, name };
    let _ = command.handle(aggregate).await?;
    Ok(())
}

pub async fn next_upgrade_target_level(
    aggregate: &BuildingQueueAggregate,
    slot_id: u8,
    name: BuildingName,
) -> Result<u8, CqrsError> {
    let command = QueueUpgradeBuildingCommand { slot_id, name };
    let events = command.handle(aggregate).await?;
    let Some(BuildingQueueEvent::BuildingUpgraded { target_level, .. }) = events.first() else {
        return Err(CqrsError::new("upgrade command emitted no event".to_string()));
    };
    Ok(*target_level)
}

pub async fn execute_add_via_cqrs(
    jobs: &[Job],
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
) -> Result<(), CqrsError> {
    let _ = queue_add_event_via_cqrs(jobs, village_id, slot_id, name).await?;
    Ok(())
}

pub async fn queue_add_event_via_cqrs(
    jobs: &[Job],
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
) -> Result<BuildingQueueEvent, CqrsError> {
    use mini_cqrs_es::Cqrs;

    let aggregate_id = Uuid::from_u128(village_id as u128);
    let event_store = seeded_event_store(jobs, aggregate_id).await?;
    let read_model = BuildingQueueReadModel::new();
    let cqrs = build_building_queue_cqrs_with_projection(event_store, read_model.clone());
    let command = QueueAddBuildingCommand {
        slot_id,
        name: name.clone(),
    };
    let _ = cqrs.execute(aggregate_id, &command).await?;
    let Some(target_level) = read_model.last_target_level_for_slot(aggregate_id, slot_id) else {
        return Err(CqrsError::new(
            "projection not updated after add command".to_string(),
        ));
    };
    Ok(BuildingQueueEvent::BuildingAdded {
        slot_id,
        name,
        target_level,
    })
}

pub async fn next_upgrade_target_level_via_cqrs(
    jobs: &[Job],
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
) -> Result<u8, CqrsError> {
    let event = queue_upgrade_event_via_cqrs(jobs, village_id, slot_id, name).await?;
    let BuildingQueueEvent::BuildingUpgraded { target_level, .. } = event else {
        return Err(CqrsError::new(
            "queue upgrade command did not emit BuildingUpgraded".to_string(),
        ));
    };
    Ok(target_level)
}

pub async fn queue_upgrade_event_via_cqrs(
    jobs: &[Job],
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
) -> Result<BuildingQueueEvent, CqrsError> {
    use mini_cqrs_es::Cqrs;

    let aggregate_id = Uuid::from_u128(village_id as u128);
    let event_store = seeded_event_store(jobs, aggregate_id).await?;
    let read_model = BuildingQueueReadModel::new();
    let cqrs = build_building_queue_cqrs_with_projection(event_store, read_model.clone());
    let command = QueueUpgradeBuildingCommand {
        slot_id,
        name: name.clone(),
    };
    let _ = cqrs.execute(aggregate_id, &command).await?;

    let Some(target_level) = read_model.last_target_level_for_slot(aggregate_id, slot_id) else {
        return Err(CqrsError::new(
            "projection not updated after upgrade command".to_string(),
        ));
    };
    Ok(BuildingQueueEvent::BuildingUpgraded {
        slot_id,
        name,
        target_level,
    })
}

pub async fn next_downgrade_target_level_via_cqrs(
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
    current_level: u8,
) -> Result<u8, CqrsError> {
    let event =
        queue_downgrade_event_via_cqrs(village_id, slot_id, name, current_level).await?;
    let BuildingQueueEvent::BuildingDowngraded { target_level, .. } = event else {
        return Err(CqrsError::new(
            "queue downgrade command did not emit BuildingDowngraded".to_string(),
        ));
    };
    Ok(target_level)
}

pub async fn queue_downgrade_event_via_cqrs(
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
    current_level: u8,
) -> Result<BuildingQueueEvent, CqrsError> {
    use mini_cqrs_es::Cqrs;

    let aggregate_id = Uuid::from_u128(village_id as u128);
    let event_store = InMemoryBuildingQueueEventStore::new();
    if current_level > 0 {
        let seed_event = Event::new(
            aggregate_id,
            BuildingQueueEvent::BuildingUpgraded {
                slot_id,
                name: name.clone(),
                target_level: current_level,
            },
            1,
        )?;
        event_store.save_events(aggregate_id, &[seed_event], 0).await?;
    }

    let read_model = BuildingQueueReadModel::new();
    let cqrs = build_building_queue_cqrs_with_projection(event_store, read_model.clone());
    let command = QueueDowngradeBuildingCommand {
        slot_id,
        name: name.clone(),
    };
    let _ = cqrs.execute(aggregate_id, &command).await?;

    let Some(target_level) = read_model.last_target_level_for_slot(aggregate_id, slot_id) else {
        return Err(CqrsError::new(
            "projection not updated after downgrade command".to_string(),
        ));
    };
    Ok(BuildingQueueEvent::BuildingDowngraded {
        slot_id,
        name,
        target_level,
    })
}

async fn seeded_event_store(
    jobs: &[Job],
    aggregate_id: Uuid,
) -> Result<InMemoryBuildingQueueEventStore, CqrsError> {
    let event_store = InMemoryBuildingQueueEventStore::new();
    let events = events_from_jobs(jobs, aggregate_id)?;
    if !events.is_empty() {
        event_store.save_events(aggregate_id, &events, 0).await?;
    }
    Ok(event_store)
}

fn events_from_jobs(jobs: &[Job], aggregate_id: Uuid) -> Result<Vec<Event>, CqrsError> {
    let mut building_jobs: Vec<_> = jobs
        .iter()
        .filter(|job| matches!(job.task.task_type.as_str(), "AddBuilding" | "BuildingUpgrade"))
        .collect();
    building_jobs.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

    let mut version: u64 = 0;
    let mut events = Vec::with_capacity(building_jobs.len());
    for job in building_jobs {
        let payload = match job.task.task_type.as_str() {
            "AddBuilding" => {
                let add = serde_json::from_value::<AddBuildingTask>(job.task.data.clone())
                    .map_err(|e| CqrsError::new(e.to_string()))?;
                BuildingQueueEvent::BuildingAdded {
                    slot_id: add.slot_id,
                    name: add.name,
                    target_level: 1,
                }
            }
            "BuildingUpgrade" => {
                let up = serde_json::from_value::<BuildingUpgradeTask>(job.task.data.clone())
                    .map_err(|e| CqrsError::new(e.to_string()))?;
                BuildingQueueEvent::BuildingUpgraded {
                    slot_id: up.slot_id,
                    name: up.building_name,
                    target_level: up.level,
                }
            }
            _ => continue,
        };
        version = version.saturating_add(1);
        events.push(Event::new(aggregate_id, payload, version)?);
    }
    Ok(events)
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
    build_building_queue_cqrs_with_projection(event_store, BuildingQueueReadModel::new())
}

pub fn build_building_queue_cqrs_with_projection(
    event_store: InMemoryBuildingQueueEventStore,
    read_model: BuildingQueueReadModel,
) -> SimpleCqrs<InMemoryBuildingQueueEventStore, SimpleAggregateManager<InMemoryBuildingQueueEventStore>>
{
    let aggregate_manager = SimpleAggregateManager::new(event_store.clone());
    let consumers = EventConsumers::new().with(read_model);
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

    #[tokio::test]
    async fn cqrs_downgrade_returns_n_minus_one() {
        let target = next_downgrade_target_level_via_cqrs(
            99,
            19,
            BuildingName::MainBuilding,
            10,
        )
        .await
        .expect("downgrade should calculate target");
        assert_eq!(target, 9);
    }
}
