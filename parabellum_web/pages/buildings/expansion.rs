use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};

use crate::components::UpgradeBlock;

/// Expansion building page (Residence/Palace) - shows culture points and upgrade block
#[component]
pub fn ExpansionBuildingPage(
    village: Village,
    slot_id: u8,
    building_name: BuildingName,
    current_level: u8,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    current_upkeep: u32,
    next_upkeep: u32,
    queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
    // Culture points data
    village_culture_points_production: u32,
    account_culture_points_production: u32,
    account_culture_points: u32,
    #[props(default = None)] next_value: Option<String>,
) -> Element {
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

            // Culture Points Information
            div { class: "bg-white border border-gray-300 rounded-lg p-6 mb-6 shadow-sm",
                h2 { class: "text-xl font-semibold mb-4 text-gray-800", "Culture Points" }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    // Village CPP
                    div { class: "bg-blue-50 p-4 rounded-lg",
                        div { class: "text-sm text-gray-600 mb-1", "Village Production" }
                        div { class: "text-2xl font-bold text-blue-700",
                            "{village_culture_points_production}"
                            span { class: "text-sm font-normal text-gray-600 ml-1", "/ day" }
                        }
                    }

                    // Account CPP
                    div { class: "bg-green-50 p-4 rounded-lg",
                        div { class: "text-sm text-gray-600 mb-1", "Total Production" }
                        div { class: "text-2xl font-bold text-green-700",
                            "{account_culture_points_production}"
                            span { class: "text-sm font-normal text-gray-600 ml-1", "/ day" }
                        }
                    }

                    // Account CP
                    div { class: "bg-purple-50 p-4 rounded-lg",
                        div { class: "text-sm text-gray-600 mb-1", "Total Culture Points" }
                        div { class: "text-2xl font-bold text-purple-700",
                            "{account_culture_points}"
                        }
                    }
                }

                div { class: "mt-4 text-sm text-gray-600",
                    p {
                        "Culture Points are required to found or conquer new villages. "
                        "Each building in your empire produces Culture Points over time. "
                        "The higher the building level, the more Culture Points it generates per day."
                    }
                }
            }

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
                csrf_token: csrf_token,
                next_value: next_value,
            }
        }
    }
}
