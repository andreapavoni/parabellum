//! Internal village aggregate state and deterministic transition helpers.
//!
//! Scheduling validation lives on scheduling methods; completion methods assume
//! work was already validated at scheduling time.
use chrono::Utc;
use parabellum_game::models::{
    army::Army,
    buildings::{Building, get_building_data},
    village::{AcademyResearch, Village, VillageBuilding, VillageProduction, VillageStocks},
};
use parabellum_types::{
    army::{TroopSet, UnitName},
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
    errors::{AppError, ApplicationError, GameError},
    map::Position,
    tribe::Tribe,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::villages::models::BuildingWorkflowKind;
use crate::villages::{ExpansionSlotUsage, ExpansionTrainingCommitment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VillageState {
    pub village: Village,
    pending_building_actions: Vec<PendingBuildingAction>,
    pending_training_actions: Vec<PendingTrainingAction>,
    pending_academy_actions: Vec<PendingAcademyAction>,
    pending_smithy_actions: Vec<PendingSmithyAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingBuildingAction {
    action_id: Uuid,
    #[serde(default = "default_pending_building_kind")]
    kind: BuildingWorkflowKind,
    slot_id: u8,
    building_name: BuildingName,
    execute_at: chrono::DateTime<Utc>,
}

fn default_pending_building_kind() -> BuildingWorkflowKind {
    BuildingWorkflowKind::Upgrade
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingTrainingAction {
    action_id: Uuid,
    slot_id: u8,
    unit: UnitName,
    #[serde(default = "default_pending_training_time_per_unit")]
    time_per_unit: i32,
    quantity_remaining: i32,
    execute_at: chrono::DateTime<Utc>,
}

fn default_pending_training_time_per_unit() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingAcademyAction {
    action_id: Uuid,
    unit: UnitName,
    execute_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingSmithyAction {
    action_id: Uuid,
    unit: UnitName,
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
            pending_training_actions: vec![],
            pending_academy_actions: vec![],
            pending_smithy_actions: vec![],
        }
    }
}

impl VillageState {
    fn deducted_cost(before: ResourceGroup, after: ResourceGroup) -> ResourceGroup {
        ResourceGroup::new(
            before.lumber().saturating_sub(after.lumber()),
            before.clay().saturating_sub(after.clay()),
            before.iron().saturating_sub(after.iron()),
            before.crop().saturating_sub(after.crop()),
        )
    }

    pub fn founded(
        id: u32,
        name: String,
        position: Position,
        tribe: Tribe,
        player_id: Uuid,
        parent_village_id: Option<u32>,
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
            parent_village_id,
        );
        let _ = village.set_army(None);
        Self {
            village,
            pending_building_actions: vec![],
            pending_training_actions: vec![],
            pending_academy_actions: vec![],
            pending_smithy_actions: vec![],
        }
    }

    pub fn player_id(&self) -> Uuid {
        self.village.player_id
    }

    pub fn tribe(&self) -> &Tribe {
        &self.village.tribe
    }

    pub fn validate_hero_creation_requirements(&self) -> Result<(), ApplicationError> {
        self.village
            .validate_building_requirements(&[BuildingRequirement(BuildingName::HeroMansion, 1)])
            .map_err(Into::into)
    }

    pub fn validate_can_deduct_resources(
        &self,
        resources: &parabellum_types::common::ResourceGroup,
    ) -> Result<(), ApplicationError> {
        let mut village = self.village.clone();
        village.deduct_resources(resources).map_err(Into::into)
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
        self.village
            .army()
            .is_some_and(|army| army.has_units(units))
    }

    /// Sets stored resources to requested absolute quantities.
    ///
    /// This method first removes any excess from current stocks, then stores the
    /// missing delta through domain storage logic, so final values are capped by
    /// current warehouse/granary capacities.
    pub fn set_resources(&mut self, resources: parabellum_types::common::ResourceGroup) {
        let current = self.village.stored_resources();
        let to_remove = parabellum_types::common::ResourceGroup::new(
            current.lumber().saturating_sub(resources.lumber()),
            current.clay().saturating_sub(resources.clay()),
            current.iron().saturating_sub(resources.iron()),
            current.crop().saturating_sub(resources.crop()),
        );
        let _ = self.village.deduct_resources(&to_remove);

        let after_remove = self.village.stored_resources();
        let to_add = parabellum_types::common::ResourceGroup::new(
            resources.lumber().saturating_sub(after_remove.lumber()),
            resources.clay().saturating_sub(after_remove.clay()),
            resources.iron().saturating_sub(after_remove.iron()),
            resources.crop().saturating_sub(after_remove.crop()),
        );
        self.village.store_resources(&to_add);
    }

    pub fn detach_army(&mut self, army: &Army) {
        let mut next = self
            .village
            .army()
            .cloned()
            .unwrap_or_else(|| Army::new_village_army(&self.village));
        let hero_id = army.hero().map(|hero| hero.id);
        let _ = next.split_units(army.units().clone(), hero_id, self.village.id);
        let next = if next.immensity() == 0 {
            None
        } else {
            Some(next)
        };
        let _ = self.village.set_army(next.as_ref());
    }

    pub fn set_building_level(
        &mut self,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    ) {
        if level == 0 {
            let _ = self.village.remove_building_at_slot(slot_id, speed);
            return;
        }
        if self.village.get_building_by_slot_id(slot_id).is_none()
            && let Ok(building) = Building::new(building_name.clone(), speed).at_level(level, speed)
        {
            let _ = self.village.add_building_at_slot(building, slot_id);
            return;
        }
        let _ = self
            .village
            .set_building_level_at_slot(slot_id, level, speed);
    }

    pub fn schedule_add_building(
        &self,
        slot_id: u8,
        building_name: BuildingName,
        speed: i8,
    ) -> Result<(i64, ResourceGroup), ApplicationError> {
        self.enforce_building_queue_capacity()?;
        self.ensure_add_queue_allows_building(&building_name)?;
        let mut village = self.village.clone();
        let before = village.stored_resources();
        village
            .init_building_construction(slot_id, building_name, speed)
            .map(|secs| {
                let after = village.stored_resources();
                (secs as i64, Self::deducted_cost(before, after))
            })
            .map_err(Into::into)
    }

    pub fn schedule_upgrade_building(
        &self,
        slot_id: u8,
        speed: i8,
    ) -> Result<(BuildingName, u8, i64, ResourceGroup), ApplicationError> {
        self.enforce_building_queue_capacity()?;
        self.enforce_no_pending_downgrade_for_slot(slot_id)?;
        let queued_for_slot: Vec<&PendingBuildingAction> = self
            .pending_building_actions
            .iter()
            .filter(|action| action.slot_id == slot_id)
            .collect();
        let mut current = self.find_building_by_slot(slot_id);
        if current.is_none() {
            let Some(first_queued) = queued_for_slot
                .iter()
                .min_by_key(|action| action.execute_at)
            else {
                return Err(GameError::EmptySlot { slot_id }.into());
            };
            let queued_level = queued_for_slot
                .iter()
                .filter(|action| action.building_name == first_queued.building_name)
                .count() as u8;
            if queued_level == 0 {
                return Err(GameError::EmptySlot { slot_id }.into());
            }
            current = Some(VillageBuilding {
                slot_id,
                building: Building::new(first_queued.building_name.clone(), speed)
                    .at_level(queued_level, speed)
                    .map_err(ApplicationError::from)?,
            });
        }
        let current = current.expect("checked above");
        let queued_changes_for_slot = (self
            .pending_building_actions
            .iter()
            .filter(|action| action.slot_id == slot_id)
            .count() as u8)
            .saturating_sub(if self.find_building_by_slot(slot_id).is_some() {
                0
            } else {
                current.building.level
            });
        let effective_level = current
            .building
            .level
            .saturating_add(queued_changes_for_slot);
        let effective_building = Building::new(current.building.name.clone(), speed)
            .at_level(effective_level, speed)
            .map_err(ApplicationError::from)?;
        let mut village = self.village.clone();
        if village.get_building_by_slot_id(slot_id).is_some() {
            village
                .set_building_level_at_slot(slot_id, effective_level, speed)
                .map_err(ApplicationError::from)?;
        } else {
            village
                .add_building_at_slot(effective_building, slot_id)
                .map_err(ApplicationError::from)?;
        }
        let before = village.stored_resources();
        let (building_name, next_level, duration_secs) = village
            .init_building_upgrade(slot_id, speed)
            .map_err(ApplicationError::from)?;
        let after = village.stored_resources();
        Ok((
            building_name,
            next_level,
            duration_secs as i64,
            Self::deducted_cost(before, after),
        ))
    }

    pub fn schedule_downgrade_building(
        &self,
        slot_id: u8,
        speed: i8,
    ) -> Result<(BuildingName, u8, i64), ApplicationError> {
        self.enforce_building_queue_capacity()?;
        self.enforce_downgrade_queue_capacity()?;
        self.enforce_no_pending_building_action_for_slot(slot_id)?;
        self.village
            .init_building_downgrade(slot_id, speed)
            .map(|(building_name, next_level, duration_secs)| {
                (building_name, next_level, duration_secs as i64)
            })
            .map_err(Into::into)
    }

    pub fn record_building_action_scheduled(
        &mut self,
        action_id: Uuid,
        kind: BuildingWorkflowKind,
        slot_id: u8,
        building_name: BuildingName,
        execute_at: chrono::DateTime<Utc>,
    ) {
        self.pending_building_actions.push(PendingBuildingAction {
            action_id,
            kind,
            slot_id,
            building_name,
            execute_at,
        });
    }

    pub fn mark_building_action_consumed(&mut self, action_id: Uuid) {
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

    pub fn schedule_train_units(
        &self,
        unit_idx: u8,
        building_name: BuildingName,
        quantity: i32,
        speed: i8,
    ) -> Result<(u8, UnitName, i32, ResourceGroup), ApplicationError> {
        self.validate_expansion_unit_training(unit_idx, quantity)?;
        let mut village = self.village.clone();
        let before = village.stored_resources();
        village
            .init_unit_training(unit_idx, &building_name, quantity, speed)
            .map(|(slot_id, unit_name, time_per_unit)| {
                let after = village.stored_resources();
                (
                    slot_id,
                    unit_name,
                    time_per_unit as i32,
                    Self::deducted_cost(before, after),
                )
            })
            .map_err(Into::into)
    }

    fn validate_expansion_unit_training(
        &self,
        unit_idx: u8,
        quantity: i32,
    ) -> Result<(), ApplicationError> {
        let unit = self
            .village
            .tribe
            .units()
            .get(unit_idx as usize)
            .ok_or(GameError::InvalidUnitIndex(unit_idx))?;
        if !unit.role.is_expansion() {
            return Ok(());
        }

        let commitments = self
            .pending_training_actions
            .iter()
            .map(|action| ExpansionTrainingCommitment {
                unit: action.unit.clone(),
                quantity_remaining: action.quantity_remaining,
            })
            .collect::<Vec<_>>();
        ExpansionSlotUsage::from_local_village(&self.village, &commitments)
            .validate_training(unit.role, quantity)
            .map_err(Into::into)
    }

    pub fn schedule_send_resources(
        &self,
        resources: ResourceGroup,
        server_speed: i8,
    ) -> Result<u8, ApplicationError> {
        self.village
            .validate_merchant_transfer(&resources, server_speed)
            .map_err(Into::into)
    }

    pub fn apply_merchant_departure(
        &mut self,
        resources: &ResourceGroup,
        merchants_used: u8,
    ) -> Result<(), ApplicationError> {
        self.village
            .reserve_merchant_transfer(resources, merchants_used)
            .map_err(ApplicationError::from)
    }

    pub fn apply_merchant_return(&mut self, merchants_used: u8) {
        self.village.return_merchants(merchants_used);
    }

    pub fn record_training_action_scheduled(
        &mut self,
        action_id: Uuid,
        slot_id: u8,
        unit: UnitName,
        time_per_unit: i32,
        quantity_remaining: i32,
        execute_at: chrono::DateTime<Utc>,
    ) {
        self.pending_training_actions.push(PendingTrainingAction {
            action_id,
            slot_id,
            unit,
            time_per_unit: time_per_unit.max(1),
            quantity_remaining,
            execute_at,
        });
    }

    pub fn mark_training_action_consumed(&mut self, action_id: Uuid) {
        self.pending_training_actions
            .retain(|action| action.action_id != action_id);
    }

    pub fn next_execution_time_for_training_slot(
        &self,
        slot_id: u8,
        duration_secs: i64,
    ) -> chrono::DateTime<Utc> {
        let now = Utc::now();
        let ready_at = self
            .pending_training_actions
            .iter()
            .filter(|action| action.slot_id == slot_id)
            .map(|action| {
                let remaining_after_next = action.quantity_remaining.saturating_sub(1) as i64;
                action.execute_at
                    + chrono::Duration::seconds(
                        remaining_after_next * i64::from(action.time_per_unit.max(1)),
                    )
            })
            .max()
            .filter(|time| *time > now)
            .unwrap_or(now);
        ready_at + chrono::Duration::seconds(duration_secs.max(1))
    }

    pub fn train_units(&mut self, unit: UnitName, quantity: u32) -> Result<(), ApplicationError> {
        let mut village_army = self
            .village
            .army()
            .map_or(Army::new_village_army(&self.village), |a| a.clone());
        village_army
            .add_unit(unit, quantity)
            .map_err(ApplicationError::from)?;
        self.village
            .set_army(Some(&village_army))
            .map_err(Into::into)
    }

    pub fn merge_units_home(&mut self, units: &TroopSet) -> Result<(), ApplicationError> {
        let mut village_army = self
            .village
            .army()
            .map_or(Army::new_village_army(&self.village), |a| a.clone());
        let mut next_units = village_army.units().clone();
        for idx in 0..10 {
            next_units.add(idx, units.get(idx));
        }
        village_army.update_units(&next_units);
        self.village
            .set_army(Some(&village_army))
            .map_err(Into::into)
    }

    pub fn schedule_academy_research(
        &self,
        unit: UnitName,
        speed: i8,
    ) -> Result<(i64, ResourceGroup), ApplicationError> {
        if self.pending_academy_actions.len() >= 2 {
            return Err(AppError::QueueLimitReached { queue: "academy" }.into());
        }
        if self
            .pending_academy_actions
            .iter()
            .any(|action| action.unit == unit)
        {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "academy",
                item: format!("{unit:?}"),
            }
            .into());
        }

        let mut village = self.village.clone();
        let before = village.stored_resources();
        village
            .init_academy_research(&unit, speed)
            .map(|secs| {
                let after = village.stored_resources();
                (secs as i64, Self::deducted_cost(before, after))
            })
            .map_err(Into::into)
    }

    pub fn record_academy_action_scheduled(
        &mut self,
        action_id: Uuid,
        unit: UnitName,
        execute_at: chrono::DateTime<Utc>,
    ) {
        self.pending_academy_actions.push(PendingAcademyAction {
            action_id,
            unit,
            execute_at,
        });
    }

    pub fn mark_academy_action_consumed(&mut self, action_id: Uuid) {
        self.pending_academy_actions
            .retain(|action| action.action_id != action_id);
    }

    pub fn next_execution_time_for_academy(&self, duration_secs: i64) -> chrono::DateTime<Utc> {
        let now = Utc::now();
        let ready_at = self
            .pending_academy_actions
            .iter()
            .map(|action| action.execute_at)
            .max()
            .filter(|time| *time > now)
            .unwrap_or(now);
        ready_at + chrono::Duration::seconds(duration_secs.max(1))
    }

    pub fn apply_academy_research_completed(
        &mut self,
        unit: UnitName,
    ) -> Result<(), ApplicationError> {
        self.village.research_academy(unit).map_err(Into::into)
    }

    pub fn schedule_smithy_research(
        &self,
        unit: UnitName,
        speed: i8,
    ) -> Result<(i64, ResourceGroup), ApplicationError> {
        if self.pending_smithy_actions.len() >= 2 {
            return Err(AppError::QueueLimitReached { queue: "smithy" }.into());
        }
        if self
            .pending_smithy_actions
            .iter()
            .any(|action| action.unit == unit)
        {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "smithy",
                item: format!("{unit:?}"),
            }
            .into());
        }

        let mut village = self.village.clone();
        let before = village.stored_resources();
        village
            .init_smithy_research(&unit, speed)
            .map(|secs| {
                let after = village.stored_resources();
                (secs as i64, Self::deducted_cost(before, after))
            })
            .map_err(Into::into)
    }

    pub fn record_smithy_action_scheduled(
        &mut self,
        action_id: Uuid,
        unit: UnitName,
        execute_at: chrono::DateTime<Utc>,
    ) {
        self.pending_smithy_actions.push(PendingSmithyAction {
            action_id,
            unit,
            execute_at,
        });
    }

    pub fn mark_smithy_action_consumed(&mut self, action_id: Uuid) {
        self.pending_smithy_actions
            .retain(|action| action.action_id != action_id);
    }

    pub fn next_execution_time_for_smithy(&self, duration_secs: i64) -> chrono::DateTime<Utc> {
        let now = Utc::now();
        let ready_at = self
            .pending_smithy_actions
            .iter()
            .map(|action| action.execute_at)
            .max()
            .filter(|time| *time > now)
            .unwrap_or(now);
        ready_at + chrono::Duration::seconds(duration_secs.max(1))
    }

    pub fn apply_smithy_research_completed(
        &mut self,
        unit: UnitName,
    ) -> Result<(), ApplicationError> {
        self.village.upgrade_smithy(unit).map_err(Into::into)
    }

    fn enforce_building_queue_capacity(&self) -> Result<(), ApplicationError> {
        let limit = if matches!(self.village.tribe, Tribe::Roman) {
            3usize
        } else {
            2usize
        };
        if self.pending_building_actions.len() >= limit {
            return Err(AppError::QueueLimitReached { queue: "building" }.into());
        }
        Ok(())
    }

    fn enforce_downgrade_queue_capacity(&self) -> Result<(), ApplicationError> {
        let queued_downgrades = self
            .pending_building_actions
            .iter()
            .filter(|action| matches!(action.kind, BuildingWorkflowKind::Downgrade))
            .count();
        if queued_downgrades >= 2 {
            return Err(AppError::QueueLimitReached {
                queue: "building_downgrade",
            }
            .into());
        }
        Ok(())
    }

    fn enforce_no_pending_downgrade_for_slot(&self, slot_id: u8) -> Result<(), ApplicationError> {
        if self.pending_building_actions.iter().any(|action| {
            action.slot_id == slot_id && matches!(action.kind, BuildingWorkflowKind::Downgrade)
        }) {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "building",
                item: slot_id.to_string(),
            }
            .into());
        }
        Ok(())
    }

    fn enforce_no_pending_building_action_for_slot(
        &self,
        slot_id: u8,
    ) -> Result<(), ApplicationError> {
        if self
            .pending_building_actions
            .iter()
            .any(|action| action.slot_id == slot_id)
        {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "building",
                item: slot_id.to_string(),
            }
            .into());
        }
        Ok(())
    }

    fn ensure_add_queue_allows_building(
        &self,
        candidate: &BuildingName,
    ) -> Result<(), ApplicationError> {
        if self.pending_building_actions.is_empty() {
            return Ok(());
        }

        let candidate_data = get_building_data(candidate).map_err(ApplicationError::from)?;
        for action in &self.pending_building_actions {
            let queued_name = action.building_name.clone();

            if candidate_data
                .rules
                .conflicts
                .iter()
                .any(|conflict| conflict.0 == queued_name)
            {
                return Err(GameError::BuildingConflict(candidate.clone(), queued_name).into());
            }

            if let Ok(queued_data) = get_building_data(&queued_name)
                && queued_data
                    .rules
                    .conflicts
                    .iter()
                    .any(|conflict| conflict.0 == *candidate)
            {
                return Err(GameError::BuildingConflict(candidate.clone(), queued_name).into());
            }
        }
        Ok(())
    }
}
