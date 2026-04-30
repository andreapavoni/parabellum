use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MovementDirection, MovementType, ScheduledAction, ScheduledActionPayload,
    ScheduledActionStatus, VillageMovement,
};
use parabellum_app::villages::repositories::{
    ScheduledActionRepository, VillageModelRepository, VillageMovementRepository,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::es::{
    PostgresScheduledActionRepository, PostgresVillageModelRepository,
    PostgresVillageMovementRepository,
};

#[derive(Debug, Clone)]
pub struct VillageProjector {
    village: PostgresVillageModelRepository,
    movements: PostgresVillageMovementRepository,
    actions: PostgresScheduledActionRepository,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            village: PostgresVillageModelRepository::new(pool.clone()),
            movements: PostgresVillageMovementRepository::new(pool.clone()),
            actions: PostgresScheduledActionRepository::new(pool),
        }
    }
}

impl EventConsumer for VillageProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        match domain_event {
            VillageEvent::VillageFounded {
                village_id,
                player_id,
                stationed_units,
                ..
            } => {
                self.village
                    .upsert_from_village(village_id, player_id, &stationed_units)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageConquered { player_id } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .update_player_id(village_id, player_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageResourcesSet { village_id, .. } => {
                self.village
                    .refresh_from_source(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::VillageArmyDetached { units, .. } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_units = current.stationed_army;
                for idx in 0..10 {
                    next_units.remove(idx, units.get(idx));
                }
                self.village
                    .update_stationed_army(village_id, &next_units)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ReinforcementSent {
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                units,
                hero_id,
                arrives_at,
            } => {
                let outgoing = VillageMovement {
                    movement_id,
                    movement_type: MovementType::Reinforcement,
                    direction: MovementDirection::Outgoing,
                    origin_village_id: source_village_id,
                    origin_village_name: None,
                    origin_player_id: player_id,
                    origin_position: None,
                    target_village_id,
                    target_village_name: None,
                    target_player_id: None,
                    target_position: None,
                    arrives_at,
                    time_seconds: None,
                    units: units.clone(),
                    tribe: None,
                };

                let incoming = VillageMovement {
                    direction: MovementDirection::Incoming,
                    ..outgoing.clone()
                };

                self.movements
                    .upsert(&outgoing)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.movements
                    .upsert(&incoming)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: Uuid::new_v4(),
                        action_type: ScheduledActionPayload::ReinforcementArrival {
                            movement_id,
                            army_id,
                            player_id,
                            source_village_id,
                            target_village_id,
                            units: units.clone(),
                            hero_id,
                            arrives_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(
                            ScheduledActionPayload::ReinforcementArrival {
                                movement_id,
                                army_id,
                                player_id,
                                source_village_id,
                                target_village_id,
                                units,
                                hero_id,
                                arrives_at,
                            },
                        )
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::ReinforcementArrived { movement_id, .. } => {
                self.movements
                    .delete_by_movement_id(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingConstructionScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::AddBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::AddBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingUpgradeScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::UpgradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::UpgradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingDowngradeScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                execute_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: ScheduledActionPayload::DowngradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name: building_name.clone(),
                            level,
                            speed,
                        }
                        .action_type(),
                        execute_at,
                        payload: serde_json::to_value(ScheduledActionPayload::DowngradeBuilding {
                            village_id,
                            player_id,
                            slot_id,
                            building_name,
                            level,
                            speed,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::BuildingAdded { .. }
            | VillageEvent::BuildingUpgraded { .. }
            | VillageEvent::BuildingDowngraded { .. } => {
                let village_id = event
                    .aggregate_id
                    .parse::<u32>()
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .refresh_from_source(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::UnitTrainingScheduled {
                action_id,
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::TrainUnit {
                    action_id,
                    village_id,
                    player_id,
                    slot_id,
                    unit,
                    time_per_unit,
                    quantity_remaining,
                    execute_at,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::UnitTrained {
                village_id,
                unit,
                quantity_trained,
                ..
            } => {
                let current = self
                    .village
                    .get_by_village_id(village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut next_units = current.stationed_army;
                if let Some(idx) = current.tribe.get_unit_idx_by_name(&unit) {
                    next_units.add(idx, quantity_trained);
                    self.village
                        .update_stationed_army(village_id, &next_units)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            VillageEvent::AcademyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchAcademy {
                    action_id,
                    village_id,
                    player_id,
                    unit,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::AcademyResearchCompleted { .. } => {}
            VillageEvent::SmithyResearchScheduled {
                action_id,
                player_id,
                village_id,
                unit,
                execute_at,
            } => {
                let payload = ScheduledActionPayload::ResearchSmithy {
                    action_id,
                    village_id,
                    player_id,
                    unit,
                };
                self.actions
                    .add(&ScheduledAction {
                        id: action_id,
                        action_type: payload.action_type(),
                        execute_at,
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::SmithyResearchCompleted { .. } => {}
        }

        Ok(())
    }
}
