use parabellum_game::models::village::VillageBuilding;
use parabellum_types::buildings::BuildingName;

/// Formats a duration in seconds to HH:MM:SS.
pub fn format_duration(total_seconds: u32) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Returns the CSS class for a resource slot based on the building placed in it.
pub fn resource_css_class(slot: Option<&VillageBuilding>) -> &'static str {
    match slot.map(|vb| &vb.building.name) {
        Some(BuildingName::Woodcutter) => "wood",
        Some(BuildingName::ClayPit) => "clay",
        Some(BuildingName::IronMine) => "iron",
        Some(BuildingName::Cropland) => "crop",
        _ => "wood",
    }
}

/// Helper to get the level of a resource field, falling back to 0 if empty.
pub fn building_level(slot: Option<&VillageBuilding>) -> u8 {
    slot.map(|vb| vb.building.level).unwrap_or(0)
}
