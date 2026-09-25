//! Transport conversion for application-owned common building previews.
use super::{
    BuildOptionDto, EmptySlotDetailDto, QueuedUpgradePreviewDto, RequirementDto, building_key,
    resource_group_to_dto,
};
use parabellum_app::villages::read_models::buildings::{
    BuildingOption, BuildingUpgradePreview, EmptyBuildingSlot,
};

pub(super) fn empty_slot_to_dto(empty: EmptyBuildingSlot) -> EmptySlotDetailDto {
    EmptySlotDetailDto {
        buildable_buildings: empty
            .buildable_buildings
            .into_iter()
            .map(option_to_dto)
            .collect(),
        locked_buildings: empty
            .locked_buildings
            .into_iter()
            .map(option_to_dto)
            .collect(),
        has_queue_for_slot: empty.has_queue_for_slot,
        queued_building_name: empty.queued_building_name.as_ref().map(building_key),
        queued_target_level: empty.queued_target_level,
        queued_next_level: empty.queued_next_level,
        queued_can_upgrade: empty.queued_can_upgrade,
        queued_upgrade_preview: empty.queued_upgrade_preview.map(upgrade_to_dto),
    }
}

fn option_to_dto(option: BuildingOption) -> BuildOptionDto {
    BuildOptionDto {
        building_name: building_key(&option.building_name),
        cost: resource_group_to_dto(&option.cost),
        next_upkeep: option.upkeep,
        upkeep: option.upkeep,
        time_secs: option.time_secs,
        missing_requirements: option
            .missing_requirements
            .into_iter()
            .map(|requirement| RequirementDto {
                building_name: building_key(&requirement.0),
                required_level: requirement.1,
            })
            .collect(),
    }
}

fn upgrade_to_dto(preview: BuildingUpgradePreview) -> QueuedUpgradePreviewDto {
    QueuedUpgradePreviewDto {
        building_name: building_key(&preview.building_name),
        current_level: preview.current_level,
        next_level: preview.next_level,
        current_upkeep: preview.current_upkeep,
        next_upkeep: preview.next_upkeep,
        time_secs: preview.time_secs,
        at_max_level: preview.at_max_level,
        current_value: preview.current_value,
        next_value: preview.next_value,
        cost: resource_group_to_dto(&preview.cost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_types::{
        buildings::{BuildingName, BuildingRequirement},
        common::ResourceGroup,
    };
    use serde_json::json;

    #[test]
    fn empty_slot_preserves_option_fields_and_omits_absent_queue_fields() {
        let dto = empty_slot_to_dto(EmptyBuildingSlot {
            buildable_buildings: vec![BuildingOption {
                building_name: BuildingName::Cranny,
                cost: ResourceGroup::new(10, 20, 30, 40),
                upkeep: 2,
                time_secs: 90,
                missing_requirements: vec![BuildingRequirement(BuildingName::MainBuilding, 3)],
            }],
            locked_buildings: vec![],
            has_queue_for_slot: false,
            queued_building_name: None,
            queued_target_level: None,
            queued_next_level: None,
            queued_can_upgrade: None,
            queued_upgrade_preview: None,
        });
        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            json!({
                "buildableBuildings": [{"buildingName":"Cranny", "cost":{"lumber":10,"clay":20,"iron":30,"crop":40}, "nextUpkeep":2,"upkeep":2,"timeSecs":90,"missingRequirements":[{"buildingName":"MainBuilding","requiredLevel":3}]}],
                "lockedBuildings":[], "hasQueueForSlot":false
            })
        );
    }

    #[test]
    fn queued_preview_preserves_null_values_and_resource_shape() {
        let dto = upgrade_to_dto(BuildingUpgradePreview {
            building_name: BuildingName::Cranny,
            current_level: 10,
            next_level: 10,
            current_upkeep: 1,
            next_upkeep: 1,
            time_secs: 0,
            at_max_level: true,
            current_value: None,
            next_value: None,
            cost: ResourceGroup::default(),
        });
        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            json!({
                "buildingName":"Cranny","currentLevel":10,"nextLevel":10,"currentUpkeep":1,"nextUpkeep":1,"timeSecs":0,"atMaxLevel":true,
                "currentValue":null,"nextValue":null,"cost":{"lumber":0,"clay":0,"iron":0,"crop":0}
            })
        );
    }
}
