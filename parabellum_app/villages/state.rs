use chrono::Utc;
use parabellum_game::models::{
    army::Army,
    buildings::{Building, get_building_data},
    village::{AcademyResearch, Village, VillageBuilding, VillageProduction, VillageStocks},
};
use parabellum_types::{
    army::TroopSet,
    buildings::BuildingName,
    map::Position,
    tribe::Tribe,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VillageState {
    pub village: Village,
    pending_building_actions: Vec<PendingBuildingAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingBuildingAction {
    action_id: Uuid,
    slot_id: u8,
    building_name: BuildingName,
    execute_at: chrono::DateTime<Utc>,
}

impl Default for VillageState {
    fn default() -> Self {
        let village = Village::from_persistence(
            0,
            "village-0".to_string(),
            Uuid::nil(),
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![],
            vec![],
            2,
            None,
            vec![],
            vec![],
            100,
            VillageProduction::default(),
            false,
            [0, 0, 0, 0, 0, 0, 0, 0],
            VillageStocks::default(),
            AcademyResearch::default(),
            0,
            0,
            Utc::now(),
            None,
        );
        Self {
            village,
            pending_building_actions: vec![],
        }
    }
}

impl VillageState {
    pub fn founded(
        id: u32,
        name: String,
        position: Position,
        tribe: Tribe,
        player_id: Uuid,
        stationed_units: TroopSet,
        buildings: Vec<VillageBuilding>,
    ) -> Self {
        let mut village = Village::from_persistence(
            id,
            name,
            player_id,
            position,
            tribe,
            buildings,
            vec![],
            2,
            None,
            vec![],
            vec![],
            100,
            VillageProduction::default(),
            false,
            [0, 0, 0, 0, 0, 0, 0, 0],
            VillageStocks::default(),
            AcademyResearch::default(),
            0,
            0,
            Utc::now(),
            None,
        );
        let army = Army::new(
            None,
            village.id,
            Some(village.id),
            village.player_id,
            village.tribe.clone(),
            &stationed_units,
            village.smithy(),
            None,
        );
        let _ = village.set_army(Some(&army));
        Self {
            village,
            pending_building_actions: vec![],
        }
    }

    pub fn player_id(&self) -> Uuid {
        self.village.player_id
    }

    pub fn building_level(&self, name: BuildingName) -> u8 {
        self.village
            .get_building_by_name(&name)
            .map(|b| b.building.level)
            .unwrap_or(0)
    }

    pub fn main_building_level(&self) -> u8 {
        self.village.main_building_level()
    }

    pub fn find_building_by_slot(&self, slot_id: u8) -> Option<VillageBuilding> {
        self.village.get_building_by_slot_id(slot_id)
    }

    pub fn has_units(&self, units: &TroopSet) -> bool {
        self.village.army().is_some_and(|army| {
            army.units()
                .units()
                .iter()
                .zip(units.units().iter())
                .all(|(available, requested)| available >= requested)
        })
    }

    pub fn detach_units(&mut self, units: &TroopSet) {
        let mut next = self
            .village
            .army()
            .cloned()
            .unwrap_or_else(|| Army::new_village_army(&self.village));
        let mut remaining = next.units().clone();
        for idx in 0..10 {
            remaining.remove(idx, units.get(idx));
        }
        next.update_units(&remaining);
        let _ = self.village.set_army(Some(&next));
    }

    pub fn stationed_units(&self) -> TroopSet {
        self.village
            .army()
            .map(|a| a.units().clone())
            .unwrap_or_default()
    }

    pub fn set_building_level(
        &mut self,
        slot_id: u8,
        _building_name: BuildingName,
        level: u8,
        speed: i8,
    ) {
        if level == 0 {
            let _ = self.village.remove_building_at_slot(slot_id, speed);
            return;
        }
        let _ = self.village.set_building_level_at_slot(slot_id, level, speed);
    }

    pub fn schedule_add_building(
        &self,
        slot_id: u8,
        building_name: BuildingName,
        speed: i8,
    ) -> Result<i64, String> {
        self.enforce_building_queue_capacity()?;
        self.ensure_add_queue_allows_building(&building_name)?;
        let mut village = self.village.clone();
        village
            .init_building_construction(slot_id, building_name, speed)
            .map(|secs| secs as i64)
            .map_err(|e| e.to_string())
    }

    pub fn schedule_upgrade_building(
        &self,
        slot_id: u8,
        speed: i8,
    ) -> Result<(BuildingName, u8, i64), String> {
        self.enforce_building_queue_capacity()?;
        let current = self
            .find_building_by_slot(slot_id)
            .ok_or_else(|| "empty slot".to_string())?;
        let max = get_building_data(&current.building.name)
            .map_err(|e| e.to_string())?
            .rules
            .max_level;
        if current.building.level >= max {
            return Err("building max level reached".to_string());
        }
        let next_level = current.building.level + 1;
        let target = Building::new(current.building.name.clone(), speed)
            .at_level(next_level, speed)
            .map_err(|e| e.to_string())?;
        let mut village = self.village.clone();
        let data = get_building_data(&current.building.name).map_err(|e| e.to_string())?;
        village
            .validate_building_requirements(data.rules.requirements)
            .map_err(|e| e.to_string())?;
        village
            .deduct_resources(&target.cost().resources)
            .map_err(|e| e.to_string())?;
        let duration_secs = target.calculate_build_time_secs(&speed, &self.main_building_level());
        Ok((current.building.name.clone(), next_level, duration_secs as i64))
    }

    pub fn schedule_downgrade_building(
        &self,
        slot_id: u8,
        speed: i8,
    ) -> Result<(BuildingName, u8, i64), String> {
        self.enforce_building_queue_capacity()?;
        if self.village.main_building_level() < 10 {
            return Err("main building level 10 required for downgrade".to_string());
        }
        let current = self
            .find_building_by_slot(slot_id)
            .ok_or_else(|| "empty slot".to_string())?;
        if current.building.level == 0 {
            return Err("invalid building level".to_string());
        }
        let next_level = current.building.level - 1;
        let target = Building::new(current.building.name.clone(), speed)
            .at_level(next_level, speed)
            .map_err(|e| e.to_string())?;
        let duration_secs = target.calculate_build_time_secs(&speed, &self.main_building_level());
        Ok((current.building.name.clone(), next_level, duration_secs as i64))
    }

    pub fn register_building_action(
        &mut self,
        action_id: Uuid,
        slot_id: u8,
        building_name: BuildingName,
        execute_at: chrono::DateTime<Utc>,
    ) {
        self.pending_building_actions.push(PendingBuildingAction {
            action_id,
            slot_id,
            building_name,
            execute_at,
        });
    }

    pub fn complete_building_action(&mut self, action_id: Uuid) {
        self.pending_building_actions
            .retain(|action| action.action_id != action_id);
    }

    pub fn next_execution_time_for_slot(
        &self,
        slot_id: u8,
        duration_secs: i64,
    ) -> chrono::DateTime<Utc> {
        let now = Utc::now();
        let ready_at = self
            .pending_building_actions
            .iter()
            .filter(|action| action.slot_id == slot_id)
            .map(|action| action.execute_at)
            .max()
            .filter(|time| *time > now)
            .unwrap_or(now);
        ready_at + chrono::Duration::seconds(duration_secs.max(1))
    }

    fn enforce_building_queue_capacity(&self) -> Result<(), String> {
        let limit = if matches!(self.village.tribe, Tribe::Roman) {
            3usize
        } else {
            2usize
        };
        if self.pending_building_actions.len() >= limit {
            return Err("building queue limit reached".to_string());
        }
        Ok(())
    }

    fn ensure_add_queue_allows_building(&self, candidate: &BuildingName) -> Result<(), String> {
        if self.pending_building_actions.is_empty() {
            return Ok(());
        }

        let candidate_data = get_building_data(candidate).map_err(|e| e.to_string())?;
        for action in &self.pending_building_actions {
            let queued_name = action.building_name.clone();
            if queued_name == *candidate && !candidate_data.rules.allow_multiple {
                return Err(format!("building {} cannot be queued multiple times", candidate));
            }

            if candidate_data
                .rules
                .conflicts
                .iter()
                .any(|conflict| conflict.0 == queued_name)
            {
                return Err(format!(
                    "building {} conflicts with queued {}",
                    candidate, queued_name
                ));
            }

            if let Ok(queued_data) = get_building_data(&queued_name)
                && queued_data
                    .rules
                    .conflicts
                    .iter()
                    .any(|conflict| conflict.0 == *candidate)
            {
                return Err(format!(
                    "building {} conflicts with queued {}",
                    candidate, queued_name
                ));
            }
        }
        Ok(())
    }
}
