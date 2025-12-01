use chrono::Utc;
use parabellum_app::{cqrs::queries::BuildingQueueItem, jobs::JobStatus};
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::buildings::BuildingName;

use crate::templates::{BuildingQueueItemView, ServerTimeContext};

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

/// Converts queue items into view representations with formatted timers.
pub fn building_queue_to_views(items: &[BuildingQueueItem]) -> Vec<BuildingQueueItemView> {
    let now = Utc::now();
    items
        .iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            BuildingQueueItemView {
                job_id: item.job_id,
                slot_id: item.slot_id,
                building_name: item.building_name.clone(),
                target_level: item.target_level,
                is_processing: matches!(item.status, JobStatus::Processing),
                time_remaining: format_duration(remaining),
                time_seconds: remaining,
                queue_class: None,
            }
        })
        .collect()
}

/// Returns the current server time information for the UI.
pub fn server_time_context() -> ServerTimeContext {
    let now = Utc::now();
    ServerTimeContext {
        formatted: now.format("%H:%M:%S").to_string(),
        timestamp: now.timestamp(),
    }
}
