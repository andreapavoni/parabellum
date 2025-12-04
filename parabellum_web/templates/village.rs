use askama::Template;
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::buildings::BuildingName;
use rust_i18n::t;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{handlers::CurrentUser, view_helpers};

use super::shared::{BuildingQueueItemView, ResourceCostView, ServerTime};

/// Template for the village center page.
#[derive(Debug, Default, Template)]
#[template(path = "village/village.html")]
pub struct VillageTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub building_queue: Vec<BuildingQueueItemView>,
    pub slot_buildings: HashMap<u8, VillageBuilding>,
    pub server_time: ServerTime,
}

impl VillageTemplate {
    pub fn slot_classes(&self, slot_id: u8, base_classes: &str, include_occupied: bool) -> String {
        let mut classes = String::from(base_classes);

        if include_occupied && self.slot_buildings.get(&slot_id).is_some() {
            classes.push(' ');
            classes.push_str("occupied");
        }

        if let Some(queue_class) = queue_state_class(&self.building_queue, slot_id) {
            classes.push(' ');
            classes.push_str(queue_class);
        }

        classes
    }

    pub fn slot_title(&self, slot_id: u8) -> String {
        if let Some(slot) = self.slot_buildings.get(&slot_id) {
            t!(
                "game.village.slots.occupied",
                name = slot.building.name,
                level = slot.building.level
            )
            .to_string()
        } else {
            t!("game.village.slots.empty").to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceField {
    pub class: &'static str,
    pub name: BuildingName,
    pub level: u8,
}

/// Template for the resource fields page.
#[derive(Debug, Template)]
#[template(path = "village/resources.html")]
pub struct ResourcesTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub resource_slots: Vec<ResourceField>,
    pub building_queue: Vec<BuildingQueueItemView>,
    pub server_time: ServerTime,
}

impl ResourcesTemplate {
    pub fn slot_title(&self, slot_index: usize) -> String {
        let slot = &self.resource_slots[slot_index];
        t!(
            "game.village.slots.occupied",
            name = slot.name,
            level = slot.level
        )
        .to_string()
    }

    pub fn slot_classes(&self, slot_field: &ResourceField, slot_id: u8) -> String {
        let mut classes = format!("hex {} occupied", slot_field.class);
        if let Some(queue_class) = queue_state_class(&self.building_queue, slot_id) {
            classes.push(' ');
            classes.push_str(queue_class);
        }
        classes
    }
}

#[derive(Debug, Clone)]
pub struct BuildingOption {
    pub name: BuildingName,
    pub cost: ResourceCostView,
    pub upkeep: u32,
    pub time_formatted: String,
}

#[derive(Debug, Clone)]
pub struct BuildingUpgradeInfo {
    pub next_level: u8,
    pub cost: ResourceCostView,
    pub current_upkeep: u32,
    pub upkeep: u32,
    pub time_formatted: String,
}

#[derive(Debug, Clone)]
pub struct UnitTrainingOption {
    pub unit_idx: u8,
    pub name: String,
    pub cost: ResourceCostView,
    pub upkeep: u32,
    pub time_formatted: String,
}

#[derive(Debug, Clone)]
pub struct UnitTrainingQueueItemView {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit_name: String,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub time_remaining: String,
    pub time_seconds: u32,
}

/// Template for individual building page.
#[derive(Debug, Default, Template)]
#[template(path = "village/building.html")]
pub struct BuildingTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub slot_id: u8,
    pub slot_building: Option<VillageBuilding>,
    pub available_buildings: Vec<BuildingOption>,
    pub upgrade: Option<BuildingUpgradeInfo>,
    pub current_upkeep: Option<u32>,
    pub csrf_token: String,
    pub flash_error: Option<String>,
    pub current_construction: Option<BuildingQueueItemView>,
    pub available_resources: ResourceCostView,
    pub server_time: ServerTime,
    pub barracks_units: Vec<UnitTrainingOption>,
    pub stable_units: Vec<UnitTrainingOption>,
    pub workshop_units: Vec<UnitTrainingOption>,
    pub training_queue_for_slot: Vec<UnitTrainingQueueItemView>,
}

impl BuildingTemplate {
    #[allow(dead_code)]
    pub fn can_afford(&self, cost: &ResourceCostView) -> bool {
        self.available_resources.lumber >= cost.lumber
            && self.available_resources.clay >= cost.clay
            && self.available_resources.iron >= cost.iron
            && self.available_resources.crop >= cost.crop
    }

    pub fn building_description(&self, name: &BuildingName) -> String {
        view_helpers::building_description(name)
    }
}

fn queue_state_class(
    building_queue: &Vec<BuildingQueueItemView>,
    slot_id: u8,
) -> Option<&'static str> {
    building_queue
        .iter()
        .find(|item| item.slot_id == slot_id)
        .map(|item| {
            if item.is_processing {
                "construction-active"
            } else {
                "construction-pending"
            }
        })
}
