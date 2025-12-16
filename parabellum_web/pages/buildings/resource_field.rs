use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{components::UpgradeBlock, view_helpers::building_description};

/// Resource field page - shows production stats and upgrade block
#[component]
pub fn ResourceFieldPage(
    village: Village,
    slot_id: u8,
    building_name: BuildingName,
    current_level: u8,
    production_value: u32,
    population: u32,
    current_upkeep: u32,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    next_upkeep: u32,
    queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    // Get description using the centralized helper
    let description = building_description(&building_name);

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4",
                "{building_name:?} (Level {current_level})"
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
                    div { class: "text-2xl font-semibold", "{building_name:?}" }
                    p { class: "mt-2 text-gray-700 text-sm", "{description}" }
                }

                // Stats grid
                div { class: "grid grid-cols-1 sm:grid-cols-4 gap-4 text-sm",
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.level\")}" }
                        div { class: "text-lg font-bold", "{current_level}" }
                    }
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.population\")}" }
                        div { class: "text-lg font-bold", "{population}" }
                    }
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.village.resources.title\")}" }
                        div { class: "text-lg font-bold", "{production_value}" }
                    }
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.upkeep\")}" }
                        div { class: "text-lg font-bold", "{current_upkeep}" }
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
