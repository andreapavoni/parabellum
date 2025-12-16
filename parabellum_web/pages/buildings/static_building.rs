use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{components::UpgradeBlock, view_helpers::building_description};

/// Value display type for static buildings
#[derive(Clone, PartialEq)]
pub enum BuildingValueType {
    /// Capacity (e.g., Warehouse, Granary) - shows storage capacity
    Capacity,
    /// Percentage bonus (e.g., Sawmill, Brickyard) - shows production bonus as percentage
    ProductionBonus { resource_type: &'static str },
    /// Defense bonus (e.g., Wall) - shows defensive value
    DefenseBonus,
    /// Construction speed bonus (e.g., Main Building) - shows build time reduction
    ConstructionSpeedBonus,
    /// Hidden capacity (e.g., Cranny) - shows how many resources can be hidden
    HiddenCapacity,
}

/// Static building page - shows building value and upgrade block
#[component]
pub fn StaticBuildingPage(
    village: Village,
    slot_id: u8,
    building_name: BuildingName,
    current_level: u8,
    current_value: u32,
    next_value: Option<u32>,
    value_type: BuildingValueType,
    population: u32,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    current_upkeep: u32,
    next_upkeep: u32,
    queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    // Get description from i18n using the helper
    let description = building_description(&building_name);

    // Format value display based on type
    let (value_label, current_value_display) = match value_type {
        BuildingValueType::Capacity => (
            t!("game.building.current_capacity").to_string(),
            format!("{}", current_value),
        ),
        BuildingValueType::ProductionBonus { resource_type } => (
            format!("{} ({})", t!("game.building.current_bonus"), resource_type),
            format!("{}%", current_value),
        ),
        BuildingValueType::DefenseBonus => (
            t!("game.building.current_bonus").to_string(),
            format!("{}%", current_value),
        ),
        BuildingValueType::ConstructionSpeedBonus => (
            t!("game.building.current_construction_time").to_string(),
            format!("{:.1}%", current_value as f32 / 10.0),
        ),
        BuildingValueType::HiddenCapacity => (
            t!("game.building.current_capacity").to_string(),
            format!("{}", current_value),
        ),
    };

    // Format next value display if available
    let next_value_display = next_value.map(|nv| match value_type {
        BuildingValueType::Capacity | BuildingValueType::HiddenCapacity => format!("{}", nv),
        BuildingValueType::ProductionBonus { .. } | BuildingValueType::DefenseBonus => {
            format!("{}%", nv)
        }
        BuildingValueType::ConstructionSpeedBonus => format!("{:.1}%", nv as f32 / 10.0),
    });

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4",
                "{building_name} (Level {current_level})"
            }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    "{error}"
                }
            }

            div { class: "space-y-6",
                // Building description
                div {
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.existing\")}" }
                    div { class: "text-2xl font-semibold", "{building_name}" }
                    if !description.is_empty() {
                        p { class: "mt-2 text-gray-700 text-sm", "{description}" }
                    }
                }

                // Stats grid
                div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm",
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.level\")}" }
                        div { class: "text-lg font-bold", "{current_level}" }
                    }
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.population\")}" }
                        div { class: "text-lg font-bold", "{population}" }
                    }
                    div { class: "p-3 border rounded-md bg-emerald-50 border-emerald-200",
                        div { class: "text-gray-500", "{value_label}" }
                        div { class: "text-lg font-bold text-emerald-700", "{current_value_display}" }
                        if let Some(next_display) = next_value_display {
                            div { class: "text-xs text-gray-500 mt-1",
                                "Next: {next_display}"
                            }
                        }
                    }
                }

                // Upgrade block
                UpgradeBlock {
                    village: village,
                    building_name: building_name,
                    current_level: current_level,
                    next_level: next_level,
                    cost: cost,
                    time_secs: time_secs,
                    current_upkeep: current_upkeep,
                    next_upkeep: next_upkeep,
                    queue_full: queue_full,
                    slot_id: slot_id,
                    csrf_token: csrf_token
                }
            }
        }
    }
}
