use mini_cqrs_es::Aggregate;
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
    pub fn founded(
        id: u32,
        player_id: Uuid,
        stationed_units: TroopSet,
        buildings: Vec<VillageBuilding>,
    ) -> Self {
        Self {
            id,
            version: 0,
            village: VillageState::founded(
                id,
                format!("village-{id}"),
                parabellum_types::map::Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Roman,
                player_id,
                stationed_units,
                buildings,
            ),
        }
    }

    pub fn player_id(&self) -> Uuid {
        self.village.player_id()
    }

    pub fn stationed_units(&self) -> TroopSet {
        self.village.stationed_units()
    }

    pub fn has_units(&self, units: &TroopSet) -> bool {
        self.village.has_units(units)
    }

    pub fn village(&self) -> &VillageState {
        &self.village
    }
}

impl Aggregate for VillageAggregate {
    type Id = u32;
    type Event = VillageEvent;

    async fn apply(&mut self, event: &Self::Event) {
        match event {
            VillageEvent::VillageFounded {
                village_id,
                village_name,
                position,
                tribe,
                player_id,
                stationed_units,
                buildings,
            } => {
                self.id = *village_id;
                self.village = VillageState::founded(
                    *village_id,
                    village_name.clone(),
                    position.clone(),
                    tribe.clone(),
                    *player_id,
                    stationed_units.clone(),
                    buildings.clone(),
                );
            }
            VillageEvent::VillageConquered { player_id } => {
                self.village.village.player_id = *player_id;
            }
            VillageEvent::VillageArmyDetached { units, .. } => {
                self.village.detach_units(units);
            }
            VillageEvent::ReinforcementSent { .. } => {}
            VillageEvent::ReinforcementArrived { .. } => {}
            VillageEvent::BuildingConstructionScheduled {
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
            VillageEvent::BuildingUpgradeScheduled {
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
