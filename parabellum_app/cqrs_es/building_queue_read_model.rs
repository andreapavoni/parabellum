use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use mini_cqrs_es::{Event, EventConsumer, Query, Uuid};

use crate::cqrs_es::building_queue::BuildingQueueEvent;

#[derive(Clone, Default)]
pub struct BuildingQueueReadModel {
    inner: Arc<RwLock<HashMap<Uuid, HashMap<u8, u8>>>>,
}

impl BuildingQueueReadModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queued_level_for_slot(&self, aggregate_id: Uuid, slot_id: u8) -> Option<u8> {
        self.inner
            .read()
            .ok()
            .and_then(|by_aggregate| by_aggregate.get(&aggregate_id).cloned())
            .and_then(|by_slot| by_slot.get(&slot_id).copied())
    }
}

impl EventConsumer for BuildingQueueReadModel {
    async fn process(&self, event: &Event) {
        let Ok(payload) = event.get_payload::<BuildingQueueEvent>() else {
            return;
        };
        let Ok(mut guard) = self.inner.write() else {
            return;
        };
        let by_slot = guard.entry(event.aggregate_id).or_default();
        match payload {
            BuildingQueueEvent::BuildingAdded {
                slot_id,
                target_level,
                ..
            }
            | BuildingQueueEvent::BuildingUpgraded {
                slot_id,
                target_level,
                ..
            } => {
                by_slot.insert(slot_id, target_level);
            }
            BuildingQueueEvent::BuildingDowngraded {
                slot_id,
                target_level,
                ..
            } => {
                if target_level == 0 {
                    by_slot.remove(&slot_id);
                } else {
                    by_slot.insert(slot_id, target_level);
                }
            }
        }
    }
}

pub struct GetQueuedBuildingLevel {
    pub model: BuildingQueueReadModel,
    pub aggregate_id: Uuid,
    pub slot_id: u8,
}

impl Query for GetQueuedBuildingLevel {
    type Output = Option<u8>;

    async fn apply(&self) -> Self::Output {
        self.model
            .queued_level_for_slot(self.aggregate_id, self.slot_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_types::buildings::BuildingName;

    #[tokio::test]
    async fn read_model_tracks_last_target_level() {
        let model = BuildingQueueReadModel::new();
        let aggregate_id = Uuid::new_v4();

        let event = Event::new(
            aggregate_id,
            BuildingQueueEvent::BuildingUpgraded {
                slot_id: 19,
                name: BuildingName::MainBuilding,
                target_level: 3,
            },
            1,
        )
        .expect("event creation");
        model.process(&event).await;

        assert_eq!(model.queued_level_for_slot(aggregate_id, 19), Some(3));
    }

    #[tokio::test]
    async fn query_returns_projected_level() {
        let model = BuildingQueueReadModel::new();
        let aggregate_id = Uuid::new_v4();

        let event = Event::new(
            aggregate_id,
            BuildingQueueEvent::BuildingAdded {
                slot_id: 22,
                name: BuildingName::Barracks,
                target_level: 1,
            },
            1,
        )
        .expect("event creation");
        model.process(&event).await;

        let query = GetQueuedBuildingLevel {
            model: model.clone(),
            aggregate_id,
            slot_id: 22,
        };

        let level = query.apply().await;
        assert_eq!(level, Some(1));
    }
}
