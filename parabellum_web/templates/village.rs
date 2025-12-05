use askama::Template;
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::{army::UnitName, buildings::BuildingName};
use rust_i18n::t;
use std::collections::HashMap;
use uuid::Uuid;

use crate::view_helpers;

use super::shared::{BuildingQueueItemView, ResourceCostView, TemplateLayout};

/// Template for the village center page.
#[derive(Debug, Default, Template)]
#[template(path = "village/village.html")]
pub struct VillageTemplate {
    pub layout: TemplateLayout,
    pub building_queue: Vec<BuildingQueueItemView>,
    pub slot_buildings: HashMap<u8, VillageBuilding>,
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

#[derive(Debug, Clone)]
pub struct BuildingRequirementView {
    pub name: BuildingName,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct AcademyResearchOption {
    #[allow(dead_code)]
    pub unit_name: UnitName,
    pub unit_value: String,
    pub display_name: String,
    pub cost: ResourceCostView,
    pub time_formatted: String,
    pub missing_requirements: Vec<BuildingRequirementView>,
}

/// Template for the resource fields page.
#[derive(Debug, Template)]
#[template(path = "village/resources.html")]
pub struct ResourcesTemplate {
    pub layout: TemplateLayout,
    pub resource_slots: Vec<ResourceField>,
    pub building_queue: Vec<BuildingQueueItemView>,
    pub home_troops: Vec<TroopCountView>,
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
    pub time_formatted: String,
    pub missing_requirements: Vec<BuildingRequirementView>,
    pub can_start: bool,
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
pub struct TroopCountView {
    pub name: String,
    pub count: u32,
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

#[derive(Debug, Clone)]
pub struct AcademyResearchQueueItemView {
    pub job_id: Uuid,
    pub unit_name: String,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone)]
pub struct SmithyQueueItemView {
    pub job_id: Uuid,
    pub unit_name: String,
    pub target_level: u8,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone)]
pub struct SmithyUpgradeOption {
    pub unit_value: String,
    pub display_name: String,
    pub current_level: u8,
    pub queued_levels: u8,
    pub max_level: u8,
    pub cost: Option<ResourceCostView>,
    pub time_formatted: Option<String>,
    pub can_upgrade: bool,
    pub is_researched: bool,
}

#[derive(Debug, Clone)]
pub struct BuildingPageContext {
    pub slot_id: u8,
    pub slot_building: Option<VillageBuilding>,
    pub building_queue_full: bool,
    pub upgrade: Option<BuildingUpgradeInfo>,
    pub current_upkeep: Option<u32>,
    pub csrf_token: String,
    pub flash_error: Option<String>,
    pub current_construction: Option<BuildingQueueItemView>,
    pub available_resources: ResourceCostView,
}

impl BuildingPageContext {
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

#[derive(Debug, Template)]
#[template(path = "village/buildings/empty_slot.html")]
pub struct EmptySlotTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub buildable_buildings: Vec<BuildingOption>,
    pub locked_buildings: Vec<BuildingOption>,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/resource.html")]
pub struct ResourceFieldTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/barracks.html")]
pub struct BarracksTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub barracks_units: Vec<UnitTrainingOption>,
    pub training_queue_for_slot: Vec<UnitTrainingQueueItemView>,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/stable.html")]
pub struct StableTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub stable_units: Vec<UnitTrainingOption>,
    pub training_queue_for_slot: Vec<UnitTrainingQueueItemView>,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/workshop.html")]
pub struct WorkshopTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub workshop_units: Vec<UnitTrainingOption>,
    pub training_queue_for_slot: Vec<UnitTrainingQueueItemView>,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/academy.html")]
pub struct AcademyTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub academy_ready_units: Vec<AcademyResearchOption>,
    pub academy_locked_units: Vec<AcademyResearchOption>,
    pub academy_researched_units: Vec<AcademyResearchOption>,
    pub academy_queue: Vec<AcademyResearchQueueItemView>,
    pub academy_queue_full: bool,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/smithy.html")]
pub struct SmithyTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
    pub smithy_units: Vec<SmithyUpgradeOption>,
    pub smithy_queue: Vec<SmithyQueueItemView>,
    pub smithy_queue_full: bool,
}

#[derive(Debug, Template)]
#[template(path = "village/buildings/generic.html")]
pub struct GenericBuildingTemplate {
    pub layout: TemplateLayout,
    pub ctx: BuildingPageContext,
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
