//! Synchronous village event projector for ES read models.
//!
//! This consumer runs in the command transaction scope and must keep read-model
//! updates consistent with event appends.
use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MovementDirection, MovementType, ScheduledAction, ScheduledActionPayload,
    ScheduledActionStatus, VillageMovement,
};
use parabellum_app::villages::repositories::{
    MarketplaceOfferRepository, ScheduledActionRepository, VillageModelRepository,
    VillageMovementRepository,
};
use parabellum_types::army::TroopSet;
use sqlx::PgPool;
use uuid::Uuid;

use crate::es::{
    PostgresMarketplaceOfferRepository, PostgresScheduledActionRepository,
    PostgresVillageModelRepository, PostgresVillageMovementRepository,
};

#[derive(Debug, Clone)]
pub struct VillageProjector {
    village: PostgresVillageModelRepository,
    movements: PostgresVillageMovementRepository,
    actions: PostgresScheduledActionRepository,
    offers: PostgresMarketplaceOfferRepository,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            village: PostgresVillageModelRepository::new(pool.clone()),
            movements: PostgresVillageMovementRepository::new(pool.clone()),
            actions: PostgresScheduledActionRepository::new(pool.clone()),
            offers: PostgresMarketplaceOfferRepository::new(pool),
        }
    }
}

impl EventConsumer for VillageProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        // Projection contract by event family:
        // - founded/conquered/resources/buildings -> rm_village
        // - reinforcement -> rm_village_movements + rm_scheduled_actions
        // - training/research scheduling -> rm_scheduled_actions
        match domain_event {
            VillageEvent::VillageFounded {
                village_id,
                village_name,
                position,
                tribe,
                player_id,
                buildings,
                ..
            } => {
                self.village
                    .upsert_from_village(
                        village_id,
                        player_id,
                        &village_name,
                        &position,
                        tribe,
                        &buildings,
                        &TroopSet::default(),
                    )
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
            VillageEvent::VillageResourcesSet {
                village_id,
                resources,
                ..
            } => {
                self.village
                    .set_stored_resources(village_id, resources)
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
                let mut next_units = current.army;
                for idx in 0..10 {
                    next_units.remove(idx, units.get(idx));
                }
                self.village
                    .update_army(village_id, &next_units)
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
            VillageEvent::ReinforcementArrived {
                movement_id,
                source_village_id,
                target_village_id,
                units,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut source_deployed = source.deployed_armies;
                for idx in 0..10 {
                    source_deployed.add(idx, units.get(idx));
                }
                self.village
                    .update_deployed_armies(source_village_id, &source_deployed)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let target = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let mut target_reinforcements = target.reinforcements;
                for idx in 0..10 {
                    target_reinforcements.add(idx, units.get(idx));
                }
                self.village
                    .update_reinforcements(target_village_id, &target_reinforcements)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.movements
                    .delete_by_movement_id(movement_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                resources,
                merchants_used,
                resources_already_reserved,
                arrives_at,
                returns_at,
            } => {
                self.actions
                    .add(&ScheduledAction {
                        id: arrival_action_id,
                        action_type: ScheduledActionPayload::MerchantsArrival {
                            action_id: arrival_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            resources: resources.clone(),
                            merchants_used,
                            arrives_at,
                        }
                        .action_type(),
                        execute_at: arrives_at,
                        payload: serde_json::to_value(ScheduledActionPayload::MerchantsArrival {
                            action_id: arrival_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            resources: resources.clone(),
                            merchants_used,
                            arrives_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                self.actions
                    .add(&ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::MerchantsReturn {
                            action_id: return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            player_id,
                            merchants_used,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::MerchantsReturn {
                            action_id: return_action_id,
                            village_id: source_village_id,
                            source_village_id,
                            player_id,
                            merchants_used,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                if !resources_already_reserved {
                    let source = self
                        .village
                        .get_by_village_id(source_village_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let next_resources = parabellum_types::common::ResourceGroup::new(
                        source.stocks.lumber.saturating_sub(resources.lumber()),
                        source.stocks.clay.saturating_sub(resources.clay()),
                        source.stocks.iron.saturating_sub(resources.iron()),
                        (source.stocks.crop.max(0) as u32).saturating_sub(resources.crop()),
                    );
                    self.village
                        .set_stored_resources(source_village_id, next_resources)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let next_busy = source.busy_merchants.saturating_add(merchants_used);
                    self.village
                        .set_busy_merchants(source_village_id, next_busy)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            VillageEvent::MerchantsArrived {
                target_village_id,
                resources,
                ..
            } => {
                let target = self
                    .village
                    .get_by_village_id(target_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    target.stocks.lumber.saturating_add(resources.lumber()),
                    target.stocks.clay.saturating_add(resources.clay()),
                    target.stocks.iron.saturating_add(resources.iron()),
                    (target.stocks.crop.max(0) as u32).saturating_add(resources.crop()),
                );
                self.village
                    .set_stored_resources(target_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MerchantsReturned {
                source_village_id,
                merchants_used,
                ..
            } => {
                let source = self
                    .village
                    .get_by_village_id(source_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let next_busy = source.busy_merchants.saturating_sub(merchants_used);
                self.village
                    .set_busy_merchants(source_village_id, next_busy)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferCreated {
                offer_id,
                owner_player_id,
                owner_village_id,
                offer_resources,
                seek_resources,
                merchants_reserved,
                created_at,
            } => {
                self.offers
                    .upsert(&parabellum_app::villages::models::MarketplaceOfferModel {
                        offer_id,
                        owner_player_id,
                        owner_village_id,
                        offer_resources,
                        seek_resources,
                        merchants_reserved,
                        status: parabellum_app::villages::models::MarketplaceOfferStatus::Open,
                        accepted_by_player_id: None,
                        accepted_by_village_id: None,
                        created_at,
                        accepted_at: None,
                        canceled_at: None,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let owner = self
                    .village
                    .get_by_village_id(owner_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let reserved: parabellum_types::common::ResourceGroup = offer_resources.into();
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    owner.stocks.lumber.saturating_sub(reserved.lumber()),
                    owner.stocks.clay.saturating_sub(reserved.clay()),
                    owner.stocks.iron.saturating_sub(reserved.iron()),
                    (owner.stocks.crop.max(0) as u32).saturating_sub(reserved.crop()),
                );
                self.village
                    .set_stored_resources(owner_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_busy_merchants(
                        owner_village_id,
                        owner.busy_merchants.saturating_add(merchants_reserved),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferCanceled {
                offer_id,
                owner_village_id,
                offer_resources,
                merchants_reserved,
                canceled_at,
                ..
            } => {
                self.offers
                    .set_status(
                        offer_id,
                        parabellum_app::villages::models::MarketplaceOfferStatus::Canceled,
                        None,
                        None,
                        canceled_at,
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;

                let owner = self
                    .village
                    .get_by_village_id(owner_village_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let refund: parabellum_types::common::ResourceGroup = offer_resources.into();
                let next_resources = parabellum_types::common::ResourceGroup::new(
                    owner.stocks.lumber.saturating_add(refund.lumber()),
                    owner.stocks.clay.saturating_add(refund.clay()),
                    owner.stocks.iron.saturating_add(refund.iron()),
                    (owner.stocks.crop.max(0) as u32).saturating_add(refund.crop()),
                );
                self.village
                    .set_stored_resources(owner_village_id, next_resources)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                self.village
                    .set_busy_merchants(
                        owner_village_id,
                        owner.busy_merchants.saturating_sub(merchants_reserved),
                    )
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
            VillageEvent::MarketplaceOfferAccepted {
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
                ..
            } => {
                self.offers
                    .set_status(
                        offer_id,
                        parabellum_app::villages::models::MarketplaceOfferStatus::Accepted,
                        Some(accepting_player_id),
                        Some(accepting_village_id),
                        accepted_at,
                    )
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
            VillageEvent::BuildingAdded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingUpgraded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            }
            | VillageEvent::BuildingDowngraded {
                village_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => {
                self.village
                    .update_building(village_id, slot_id, building_name, level, speed)
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
                let mut next_units = current.army;
                if let Some(idx) = current.tribe.get_unit_idx_by_name(&unit) {
                    next_units.add(idx, quantity_trained);
                    self.village
                        .update_army(village_id, &next_units)
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
