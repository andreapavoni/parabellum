use std::{collections::HashMap, fmt::Display};

use mini_cqrs_es::{Aggregate, Command, CqrsError, EventPayload, Uuid};
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
}
