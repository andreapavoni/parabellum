//! Common building preview policy; domain models own costs and prerequisites.
use crate::villages::{
    models::BuildingWorkflowKind,
    policies::building_queue::queued_building_conflicts,
    read_models::{BuildingQueueItem, buildings::*},
};
use parabellum_game::models::{
    buildings::{Building, get_building_data},
    village::{Village, VillageBuilding},
};
use parabellum_types::{
    buildings::{BuildingName, BuildingRequirement},
    common::ResourceGroup,
};

/// Computes the next upgrade without applying command side effects or reading time.
pub fn occupied_preview(
    village: &Village,
    slot: &VillageBuilding,
    queue: &[BuildingQueueItem],
    speed: i8,
) -> BuildingUpgradePreview {
    let slot_id = slot.slot_id;
    let main_building_level = village.main_building_level();
    let current_level = slot.building.level;
    let queued_upgrades = queue
        .iter()
        .filter(|item| {
            item.slot_id == slot_id
                && matches!(
                    item.kind,
                    BuildingWorkflowKind::Add | BuildingWorkflowKind::Upgrade
                )
        })
        .count() as u8;

    let max_level = get_building_data(&slot.building.name)
        .map(|data| data.rules.max_level)
        .unwrap_or(current_level);
    let pending_level = current_level.saturating_add(queued_upgrades);
    let at_max_level = pending_level >= max_level;
    let next_level = pending_level.saturating_add(1).min(max_level);

    let upgrade_info = if at_max_level {
        None
    } else {
        slot.building.clone().at_level(next_level, speed).ok()
    };
    let (cost, time_secs, next_upkeep) = if let Some(ref upgraded) = upgrade_info {
        let computed = upgraded.cost();
        (
            computed.resources,
            upgraded.calculate_build_time_secs(&speed, &main_building_level),
            computed.upkeep,
        )
    } else {
        let current_cost = slot.building.cost();
        (current_cost.resources, 0, current_cost.upkeep)
    };

    let current_value = if slot.building.value == 0 {
        None
    } else {
        Some(slot.building.value)
    };
    let next_value = upgrade_info.as_ref().map(|upgraded| upgraded.value);

    BuildingUpgradePreview {
        building_name: slot.building.name.clone(),
        current_level,
        next_level,
        current_upkeep: slot.building.cost().upkeep,
        next_upkeep,
        time_secs,
        at_max_level,
        current_value,
        next_value,
        cost,
    }
}

/// Describes empty-slot choices, or the next upgrade of already queued construction.
pub fn empty_preview(
    village: &Village,
    slot_id: u8,
    queue: &[BuildingQueueItem],
    speed: i8,
) -> EmptyBuildingSlot {
    let queued_for_slot: Vec<&BuildingQueueItem> = queue
        .iter()
        .filter(|item| item.slot_id == slot_id)
        .collect();
    let queued = queued_for_slot.last().copied();
    let has_queue_for_slot = !queued_for_slot.is_empty();
    let (buildable_buildings, locked_buildings) = if has_queue_for_slot {
        (vec![], vec![])
    } else {
        build_options_for_slot(&village, slot_id, queue, speed)
    };
    let queued_target_level = queued.map(|item| item.target_level);
    let queued_next_level = queued_target_level.map(|level| level.saturating_add(1));
    let queued_can_upgrade = queued.and_then(|item| {
        get_building_data(&item.building_name)
            .ok()
            .map(|data| item.target_level < data.rules.max_level)
    });
    let queued_upgrade_preview = queued.map(|item| {
        let current_level = item.target_level;
        let building_name = item.building_name.clone();
        let template = Building::new(building_name.clone(), speed);
        let current_building = template.at_level(current_level, speed).ok();
        let current_upkeep = current_building
            .as_ref()
            .map(|b| b.cost().upkeep)
            .unwrap_or(template.cost().upkeep);
        let current_value = current_building
            .as_ref()
            .and_then(|building| (building.value > 0).then_some(building.value));
        let max_level = get_building_data(&building_name)
            .map(|data| data.rules.max_level)
            .unwrap_or(current_level);
        let at_max_level = current_level >= max_level;
        let next_level = current_level.saturating_add(1).min(max_level);
        let main_building_level = village.main_building_level();
        let next_building = if at_max_level {
            None
        } else {
            template.at_level(next_level, speed).ok()
        };
        let (next_upkeep, time_secs, cost, next_value) = if let Some(ref upgraded) = next_building {
            let computed = upgraded.cost();
            (
                computed.upkeep,
                upgraded.calculate_build_time_secs(&speed, &main_building_level),
                computed.resources,
                Some(upgraded.value),
            )
        } else {
            (current_upkeep, 0, ResourceGroup::new(0, 0, 0, 0), None)
        };

        BuildingUpgradePreview {
            building_name: building_name,
            current_level,
            next_level,
            current_upkeep,
            next_upkeep,
            time_secs,
            at_max_level,
            current_value,
            next_value,
            cost,
        }
    });

    EmptyBuildingSlot {
        buildable_buildings,
        locked_buildings,
        has_queue_for_slot,
        queued_building_name: queued.map(|item| item.building_name.clone()),
        queued_target_level,
        queued_next_level,
        queued_can_upgrade,
        queued_upgrade_preview,
    }
}

fn build_options_for_slot(
    village: &parabellum_game::models::village::Village,
    slot_id: u8,
    queue: &[BuildingQueueItem],
    server_speed: i8,
) -> (Vec<BuildingOption>, Vec<BuildingOption>) {
    let mut buildable = Vec::new();
    let mut locked = Vec::new();
    let main_building_level = village.main_building_level();

    for name in village.candidate_buildings_for_slot(slot_id) {
        if queue
            .iter()
            .any(|item| queued_building_conflicts(&name, &item.building_name))
        {
            continue;
        }

        let building = Building::new(name.clone(), server_speed);
        let validation_ok = village.validate_building_construction(&building).is_ok();
        let missing_requirements = missing_building_requirements(village, &name);

        if !validation_ok && missing_requirements.is_empty() {
            continue;
        }

        let cost = building.cost();
        let time_secs = building.calculate_build_time_secs(&server_speed, &main_building_level);
        let option = BuildingOption {
            building_name: name,
            cost: cost.resources,
            upkeep: cost.upkeep,
            time_secs,
            missing_requirements,
        };

        if validation_ok {
            buildable.push(option);
        } else {
            locked.push(option);
        }
    }

    (buildable, locked)
}

fn missing_building_requirements(
    village: &parabellum_game::models::village::Village,
    name: &BuildingName,
) -> Vec<BuildingRequirement> {
    let Ok(data) = get_building_data(name) else {
        return vec![];
    };

    data.rules
        .requirements
        .iter()
        .filter_map(|req| {
            let level = village
                .buildings()
                .iter()
                .find(|vb| vb.building.name == req.0)
                .map(|vb| vb.building.level)
                .unwrap_or(0);

            if level >= req.1 {
                None
            } else {
                Some(req.clone())
            }
        })
        .collect()
}
