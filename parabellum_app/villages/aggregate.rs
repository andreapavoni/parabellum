//! Event-sourced village aggregate state and event application rules.
//!
//! The aggregate mirrors domain state in `VillageState` and applies only
//! `VillageEvent` transitions.
use mini_cqrs_es::Aggregate;
use parabellum_game::models::army::Army;
use parabellum_game::models::village::{VillageBuilding, VillageSnapshot};
use parabellum_types::army::TroopSet;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::villages::{VillageEvent, state::VillageState};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VillageAggregate {
    id: u32,
    version: u64,
    village: VillageState,
}

impl VillageAggregate {
    pub fn founded(id: u32, player_id: Uuid, buildings: Vec<VillageBuilding>) -> Self {
        Self {
            id,
            version: 0,
            village: VillageState::founded(
                id,
                format!("village-{id}"),
                parabellum_types::map::Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Roman,
                player_id,
                None,
                buildings,
            ),
        }
    }

    pub fn player_id(&self) -> Uuid {
        self.village.player_id()
    }

    pub fn has_units(&self, units: &TroopSet) -> bool {
        self.village.has_units(units)
    }

    pub fn schedule_send_resources(
        &self,
        resources: parabellum_types::common::ResourceGroup,
        server_speed: i8,
    ) -> Result<u8, parabellum_types::errors::ApplicationError> {
        self.village
            .schedule_send_resources(resources, server_speed)
    }

    pub fn village(&self) -> &VillageState {
        &self.village
    }

    #[cfg(test)]
    pub fn set_resources_for_test(&mut self, resources: parabellum_types::common::ResourceGroup) {
        self.village.village.store_resources(&resources);
    }

    #[cfg(test)]
    pub fn set_academy_research_for_test(
        &mut self,
        unit: &parabellum_types::army::UnitName,
        is_researched: bool,
    ) {
        self.village
            .village
            .set_academy_research_for_test(unit, is_researched);
    }
}

impl Aggregate for VillageAggregate {
    type Id = u32;
    type Event = VillageEvent;

    async fn apply(&mut self, event: &Self::Event) {
        // Keep apply deterministic: no external reads/writes, only state transitions.
        match event {
            VillageEvent::VillageFounded {
                village_id,
                village_name,
                position,
                tribe,
                player_id,
                parent_village_id,
                buildings,
            } => {
                self.id = *village_id;
                self.village = VillageState::founded(
                    *village_id,
                    village_name.clone(),
                    position.clone(),
                    tribe.clone(),
                    *player_id,
                    *parent_village_id,
                    buildings.clone(),
                );
            }
            VillageEvent::VillageConquered { player_id, .. } => {
                self.village.village.player_id = *player_id;
            }
            VillageEvent::VillageRenamed { village_name, .. } => {
                self.village.village.name = village_name.clone();
            }
            VillageEvent::VillageResourcesSet { resources, .. } => {
                self.village.set_resources(resources.clone());
            }
            VillageEvent::VillageArmyDetached { army } => {
                self.village.detach_army(army);
            }
            VillageEvent::HeroCreated { hero, .. } => {
                let mut home_army = self
                    .village
                    .village
                    .army()
                    .cloned()
                    .unwrap_or_else(|| Army::new_village_army(&self.village.village));
                home_army.set_hero(Some(hero.clone()));
                let _ = self.village.village.set_army(Some(&home_army));
            }
            VillageEvent::HeroRevivalScheduled { cost, .. } => {
                let _ = self.village.village.deduct_resources(cost);
            }
            VillageEvent::HeroRevived { hero, .. } => {
                let mut home_army = self
                    .village
                    .village
                    .army()
                    .cloned()
                    .unwrap_or_else(|| Army::new_village_army(&self.village.village));
                home_army.set_hero(Some(hero.clone()));
                let _ = self.village.village.set_army(Some(&home_army));
            }
            VillageEvent::ReinforcementSent { .. } => {}
            VillageEvent::ReinforcementArrived { .. } => {}
            VillageEvent::ReinforcementAppliedToVillage { .. } => {}
            VillageEvent::ReinforcementsRecalled { .. } => {}
            VillageEvent::ReinforcementsReleased { .. } => {}
            VillageEvent::SettlersSent { .. } => {
                let _ = self.village.village.deduct_foundation_resources();
            }
            VillageEvent::SettlersArrived { .. } => {}
            VillageEvent::AttackSent { .. } => {}
            VillageEvent::AttackArrivalScheduled { .. } => {}
            VillageEvent::AttackArrived { .. } => {}
            VillageEvent::AttackBattleResolved { .. } => {}
            VillageEvent::BattleOutcomeAppliedToVillage {
                target_player_id,
                target_tribe,
                target_parent_village_id,
                target_loyalty,
                target_buildings,
                target_production: _,
                target_population: _,
                target_stocks,
                target_army,
                target_reinforcements,
                stationed_attacker_army,
                ..
            } => {
                let current = &self.village.village;
                let mut reinforcements = target_reinforcements.clone();
                if let Some(stationed_attacker) = stationed_attacker_army.clone() {
                    reinforcements.push(stationed_attacker);
                }
                self.village.village =
                    parabellum_game::models::village::Village::rehydrate(VillageSnapshot {
                        id: current.id,
                        name: current.name.clone(),
                        player_id: *target_player_id,
                        position: current.position.clone(),
                        tribe: target_tribe.clone(),
                        buildings: target_buildings.clone(),
                        oases: current.oases.clone(),
                        army: target_army.clone(),
                        reinforcements,
                        deployed_armies: current.deployed_armies().clone(),
                        loyalty: *target_loyalty,
                        is_capital: current.is_capital,
                        smithy: *current.smithy(),
                        stocks: target_stocks.clone(),
                        academy_research: current.academy_research().clone(),
                        culture_points: current.culture_points,
                        updated_at: chrono::Utc::now(),
                        parent_village_id: *target_parent_village_id,
                    });
            }
            VillageEvent::ArmyReturned { army, bounty, .. } => {
                let _ = self.village.merge_units_home(army.units());
                if let Some(bounty) = bounty {
                    self.village.village.store_resources(bounty);
                }
            }
            VillageEvent::ScoutSent { .. } => {}
            VillageEvent::TroopMovementCanceled { .. } => {}
            VillageEvent::ScoutArrived { .. } => {}
            VillageEvent::ScoutBattleResolved { .. } => {}
            VillageEvent::MerchantsTripScheduled {
                resources,
                merchants_used,
                resources_already_reserved,
                ..
            } => {
                if !resources_already_reserved {
                    let _ = self
                        .village
                        .apply_merchant_departure(resources, *merchants_used);
                }
            }
            VillageEvent::MerchantsArrived { .. } => {}
            VillageEvent::MerchantTransferAppliedToVillage { .. } => {}
            VillageEvent::MerchantsReturned { merchants_used, .. } => {
                self.village.apply_merchant_return(*merchants_used);
            }
            VillageEvent::MarketplaceOfferCreated {
                offer_resources,
                merchants_reserved,
                ..
            } => {
                let resources: parabellum_types::common::ResourceGroup = (*offer_resources).into();
                let _ = self
                    .village
                    .apply_merchant_departure(&resources, *merchants_reserved);
            }
            VillageEvent::MarketplaceOfferReservationAppliedToVillage { .. } => {}
            VillageEvent::MarketplaceOfferCanceled {
                owner_village_id,
                offer_resources,
                merchants_reserved,
                ..
            } => {
                if *owner_village_id == self.id {
                    let resources: parabellum_types::common::ResourceGroup =
                        (*offer_resources).into();
                    self.village
                        .village
                        .release_merchant_transfer(&resources, *merchants_reserved);
                }
            }
            VillageEvent::MarketplaceOfferReservationReleasedFromVillage { .. } => {}
            VillageEvent::MarketplaceOfferAccepted { .. } => {}
            VillageEvent::MarketplaceOfferAcceptanceAppliedToVillage {
                village_id,
                stocks,
                busy_merchants,
                ..
            } => {
                if *village_id == self.id {
                    let current = self.village.village.stored_resources();
                    let desired = parabellum_types::common::ResourceGroup::new(
                        stocks.lumber,
                        stocks.clay,
                        stocks.iron,
                        stocks.crop.max(0) as u32,
                    );
                    let delta_add = parabellum_types::common::ResourceGroup::new(
                        desired.lumber().saturating_sub(current.lumber()),
                        desired.clay().saturating_sub(current.clay()),
                        desired.iron().saturating_sub(current.iron()),
                        desired.crop().saturating_sub(current.crop()),
                    );
                    let delta_sub = parabellum_types::common::ResourceGroup::new(
                        current.lumber().saturating_sub(desired.lumber()),
                        current.clay().saturating_sub(desired.clay()),
                        current.iron().saturating_sub(desired.iron()),
                        current.crop().saturating_sub(desired.crop()),
                    );
                    if delta_add.total() > 0 {
                        self.village.village.store_resources(&delta_add);
                    }
                    if delta_sub.total() > 0 {
                        let _ = self.village.village.deduct_resources(&delta_sub);
                    }
                    self.village.village.busy_merchants = *busy_merchants;
                }
            }
            VillageEvent::BuildingConstructionScheduled {
                action_id,
                slot_id,
                building_name,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village.record_building_action_scheduled(
                    *action_id,
                    crate::villages::models::BuildingWorkflowKind::Add,
                    *slot_id,
                    building_name.clone(),
                    *execute_at,
                );
            }
            VillageEvent::BuildingUpgradeScheduled {
                action_id,
                slot_id,
                building_name,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village.record_building_action_scheduled(
                    *action_id,
                    crate::villages::models::BuildingWorkflowKind::Upgrade,
                    *slot_id,
                    building_name.clone(),
                    *execute_at,
                );
            }
            VillageEvent::BuildingDowngradeScheduled {
                action_id,
                slot_id,
                building_name,
                execute_at,
                ..
            } => self.village.record_building_action_scheduled(
                *action_id,
                crate::villages::models::BuildingWorkflowKind::Downgrade,
                *slot_id,
                building_name.clone(),
                *execute_at,
            ),
            VillageEvent::BuildingConstructionCanceled {
                action_ids, refund, ..
            } => {
                for action_id in action_ids {
                    self.village.mark_building_action_consumed(*action_id);
                }
                self.village.village.store_resources(refund);
            }
            VillageEvent::BuildingAdded {
                action_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => {
                self.village.mark_building_action_consumed(*action_id);
                self.village
                    .set_building_level(*slot_id, building_name.clone(), *level, *speed);
            }
            VillageEvent::BuildingUpgraded {
                action_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => {
                self.village.mark_building_action_consumed(*action_id);
                self.village
                    .set_building_level(*slot_id, building_name.clone(), *level, *speed);
            }
            VillageEvent::BuildingDowngraded {
                action_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => {
                self.village.mark_building_action_consumed(*action_id);
                self.village
                    .set_building_level(*slot_id, building_name.clone(), *level, *speed);
            }
            VillageEvent::UnitTrainingScheduled {
                action_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village.record_training_action_scheduled(
                    *action_id,
                    *slot_id,
                    unit.clone(),
                    *time_per_unit,
                    *quantity_remaining,
                    *execute_at,
                );
            }
            VillageEvent::UnitTrained {
                action_id,
                unit,
                quantity_trained,
                ..
            } => {
                self.village.mark_training_action_consumed(*action_id);
                let _ = self.village.train_units(unit.clone(), *quantity_trained);
            }
            VillageEvent::AcademyResearchScheduled {
                action_id,
                unit,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village
                    .record_academy_action_scheduled(*action_id, unit.clone(), *execute_at);
            }
            VillageEvent::AcademyResearchCompleted {
                action_id, unit, ..
            } => {
                self.village.mark_academy_action_consumed(*action_id);
                let _ = self.village.apply_academy_research_completed(unit.clone());
            }
            VillageEvent::SmithyResearchScheduled {
                action_id,
                unit,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village
                    .record_smithy_action_scheduled(*action_id, unit.clone(), *execute_at);
            }
            VillageEvent::SmithyResearchCompleted {
                action_id, unit, ..
            } => {
                self.village.mark_smithy_action_consumed(*action_id);
                let _ = self.village.apply_smithy_research_completed(unit.clone());
            }
            VillageEvent::ReportMarkedAsRead { .. } => {}
        }
    }

    fn aggregate_id(&self) -> Self::Id {
        self.id
    }

    fn set_aggregate_id(&mut self, id: Self::Id) {
        self.id = id;
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn set_version(&mut self, version: u64) {
        self.version = version;
    }
}
