use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};

use crate::components::UpgradeBlock;
use crate::pages::buildings::{
    TrainingQueue, TrainingQueueItem, TrainingUnitCard, UnitTrainingOption,
};

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
    // Settler training data
    max_foundation_slots: u8,
    child_villages_count: u32,
    settlers_at_home: u32,
    settlers_deployed: u32,
    max_settlers_trainable: u32,
    training_units: Vec<UnitTrainingOption>,
    training_queue: Vec<TrainingQueueItem>,
    #[props(default = None)] next_value: Option<String>,
    #[props(default = None)] next_cp_required: Option<u32>,
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

                // Show next CP requirement if available
                if let Some(required_cp) = next_cp_required {
                    div { class: "mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded",
                        div { class: "text-sm font-medium text-gray-700",
                            "Next village requires: "
                            span { class: "font-bold text-yellow-700", "{required_cp}" }
                            " Culture Points"
                        }
                    }
                }
            }

            // Expansion Training Section (only show if building has foundation slots)
            if max_foundation_slots > 0 {
                div { class: "bg-white border border-gray-300 rounded-lg p-6 mb-6 shadow-sm",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-800", "Expansion" }

                    // Foundation Slots Status
                    div { class: "mb-4",
                        h3 { class: "text-lg font-medium mb-2 text-gray-700", "Foundation Slots" }
                        div { class: "flex items-center gap-2",
                            for i in 0..max_foundation_slots {
                                if (i as u32) < child_villages_count {
                                    // Occupied slot
                                    div { class: "w-16 h-16 bg-red-500 border-2 border-red-700 rounded flex items-center justify-center",
                                        span { class: "text-white font-bold", "✓" }
                                    }
                                } else {
                                    // Available slot
                                    div { class: "w-16 h-16 bg-green-500 border-2 border-green-700 rounded flex items-center justify-center",
                                        span { class: "text-white font-bold", "○" }
                                    }
                                }
                            }
                        }
                        div { class: "text-sm text-gray-600 mt-2",
                            "{child_villages_count} / {max_foundation_slots} slots used"
                        }
                    }

                    // Settlers Status
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",
                        div { class: "bg-blue-50 p-4 rounded-lg",
                            div { class: "text-sm text-gray-600 mb-1", "Settlers at Home" }
                            div { class: "text-2xl font-bold text-blue-700", "{settlers_at_home}" }
                        }
                        div { class: "bg-purple-50 p-4 rounded-lg",
                            div { class: "text-sm text-gray-600 mb-1", "Settlers Deployed" }
                            div { class: "text-2xl font-bold text-purple-700", "{settlers_deployed}" }
                        }
                    }

                    // Settler Training - using reusable training component
                    if !training_units.is_empty() {
                        div { class: "space-y-3 mt-4",
                            h3 { class: "text-lg font-medium mb-3 text-gray-700", "Train Settlers" }
                            for unit in training_units.iter() {
                                TrainingUnitCard {
                                    unit: unit.clone(),
                                    slot_id: slot_id,
                                    building_name: building_name.clone(),
                                    csrf_token: csrf_token.clone()
                                }
                            }
                        }
                    } else {
                        div { class: "bg-gray-100 border border-gray-300 rounded-lg p-4 mt-4",
                            div { class: "text-sm text-gray-600",
                                if max_foundation_slots == child_villages_count as u8 {
                                    "All foundation slots are in use. You cannot train more settlers until you have available slots."
                                } else if settlers_at_home + settlers_deployed >= max_settlers_trainable {
                                    "You have reached the maximum number of settlers for your available slots."
                                } else {
                                    "Settlers are not yet available for training."
                                }
                            }
                        }
                    }

                    // Training queue
                    if !training_queue.is_empty() {
                        TrainingQueue { queue: training_queue }
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
