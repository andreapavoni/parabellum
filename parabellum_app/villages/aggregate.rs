//! Event-sourced village aggregate state and event application rules.
//!
//! The aggregate mirrors domain state in `VillageState` and applies only
//! `VillageEvent` transitions.
use mini_cqrs_es::Aggregate;
use parabellum_game::models::army::Army;
use parabellum_game::models::village::VillageBuilding;
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
    ) -> Result<u8, parabellum_types::errors::ApplicationError> {
        self.village.schedule_send_resources(resources)
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
            VillageEvent::VillageResourcesSet { resources, .. } => {
                self.village.set_resources(resources.clone());
            }
            VillageEvent::VillageArmyDetached { army } => {
                self.village.detach_units(army.units());
                if army.hero().is_some()
                    && let Some(mut home_army) = self.village.village.army().cloned()
                {
                    home_army.set_hero(None);
                    let next = if home_army.immensity() == 0 {
                        None
                    } else {
                        Some(home_army)
                    };
                    let _ = self.village.village.set_army(next.as_ref());
                }
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
            VillageEvent::ReinforcementsRecalled { .. } => {}
            VillageEvent::ReinforcementsReleased { .. } => {}
            VillageEvent::SettlersSent { .. } => {
                let resources = parabellum_types::common::ResourceGroup::new(800, 800, 800, 800);
                let _ = self.village.village.deduct_resources(&resources);
            }
            VillageEvent::SettlersArrived { .. } => {}
            VillageEvent::AttackSent { .. } => {}
            VillageEvent::AttackArrived { .. } => {}
            VillageEvent::ArmyReturned { army, bounty, .. } => {
                let _ = self.village.merge_units_home(army.units());
                if let Some(bounty) = bounty {
                    self.village.village.store_resources(bounty);
                }
            }
            VillageEvent::ScoutSent { .. } => {}
            VillageEvent::ScoutArrived { .. } => {}
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
            VillageEvent::MarketplaceOfferCanceled {
                owner_village_id,
                offer_resources,
                merchants_reserved,
                ..
            } => {
                if *owner_village_id == self.id {
                    let resources: parabellum_types::common::ResourceGroup =
                        (*offer_resources).into();
                    self.village.village.store_resources(&resources);
                    self.village.apply_merchant_return(*merchants_reserved);
                }
            }
            VillageEvent::MarketplaceOfferAccepted {
                accepting_village_id,
                seek_resources,
                accepting_merchants_used,
                ..
            } => {
                if *accepting_village_id == self.id {
                    let resources: parabellum_types::common::ResourceGroup =
                        (*seek_resources).into();
                    let _ = self
                        .village
                        .apply_merchant_departure(&resources, *accepting_merchants_used);
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
                self.village.register_building_action(
                    *action_id,
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
                self.village.register_building_action(
                    *action_id,
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
            } => self.village.register_building_action(
                *action_id,
                *slot_id,
                building_name.clone(),
                *execute_at,
            ),
            VillageEvent::BuildingAdded {
                action_id,
                slot_id,
                building_name,
                level,
                speed,
                ..
            } => {
                self.village.complete_building_action(*action_id);
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
                self.village.complete_building_action(*action_id);
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
                self.village.complete_building_action(*action_id);
                self.village
                    .set_building_level(*slot_id, building_name.clone(), *level, *speed);
            }
            VillageEvent::UnitTrainingScheduled {
                action_id,
                slot_id,
                unit,
                quantity_remaining,
                cost,
                execute_at,
                ..
            } => {
                let _ = self.village.village.deduct_resources(cost);
                self.village.register_training_action(
                    *action_id,
                    *slot_id,
                    unit.clone(),
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
                self.village.complete_training_action(*action_id);
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
                    .register_academy_action(*action_id, unit.clone(), *execute_at);
            }
            VillageEvent::AcademyResearchCompleted {
                action_id, unit, ..
            } => {
                self.village.complete_academy_action(*action_id);
                let _ = self.village.complete_academy_research(unit.clone());
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
                    .register_smithy_action(*action_id, unit.clone(), *execute_at);
            }
            VillageEvent::SmithyResearchCompleted {
                action_id, unit, ..
            } => {
                self.village.complete_smithy_action(*action_id);
                let _ = self.village.complete_smithy_research(unit.clone());
            }
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
