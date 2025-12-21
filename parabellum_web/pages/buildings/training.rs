use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{
    components::UpgradeBlock,
    view_helpers::{building_description_paragraphs, format_duration},
};

/// Unit training option
#[derive(Clone, PartialEq)]
pub struct UnitTrainingOption {
    pub unit_idx: u8,
    pub name: String,
    pub cost: ResourceGroup,
    pub upkeep: u32,
    pub time_secs: u32,
    pub max_quantity: Option<u32>,
}

/// Training queue item
#[derive(Clone, PartialEq)]
pub struct TrainingQueueItem {
    pub quantity: u32,
    pub unit_name: String,
    pub time_per_unit: u32,
    pub time_remaining_secs: u32,
}

/// Training building page - for Barracks, Stable, Workshop
#[component]
pub fn TrainingBuildingPage(
    village: Village,
    slot_id: u8,
    building_name: BuildingName,
    current_level: u8,
    current_value: u32,
    population: u32,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    current_upkeep: u32,
    next_upkeep: u32,
    queue_full: bool,
    training_units: Vec<UnitTrainingOption>,
    training_queue: Vec<TrainingQueueItem>,
    csrf_token: String,
    flash_error: Option<String>,
    #[props(default = None)] next_value: Option<String>,
) -> Element {
    let description_paragraphs = building_description_paragraphs(&building_name);
    let training_speed_percent = (current_value as f32 / 10.0) as u32;

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
                    if !description_paragraphs.is_empty() {
                        div { class: "mt-2 text-gray-700 text-sm space-y-2",
                            for paragraph in description_paragraphs.iter() {
                                p { "{paragraph}" }
                            }
                        }
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
                    div { class: "p-3 border rounded-md bg-blue-50 border-blue-200",
                        div { class: "text-gray-500", "{t!(\"game.building.training_multiplier\")}" }
                        div { class: "text-lg font-bold text-blue-700", "{training_speed_percent}%" }
                    }
                }

                // Upgrade block
                UpgradeBlock {
                    village: village.clone(),
                    building_name: building_name.clone(),
                    current_level: current_level,
                    next_level: next_level,
                    cost: cost,
                    time_secs: time_secs,
                    current_upkeep: current_upkeep,
                    next_upkeep: next_upkeep,
                    queue_full: queue_full,
                    slot_id: slot_id,
                    csrf_token: csrf_token.clone(),
                    next_value: next_value.clone(),
                }

                // Training units
                div { class: "space-y-3",
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.train_units\")}" }
                    if training_units.is_empty() {
                        p { class: "text-sm text-gray-500", "{t!(\"game.building.no_units_available\")}" }
                    } else {
                        div { class: "space-y-4",
                            for unit in training_units.iter() {
                                TrainingUnitCard {
                                    unit: unit.clone(),
                                    slot_id: slot_id,
                                    building_name: building_name.clone(),
                                    csrf_token: csrf_token.clone()
                                }
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
    }
}

/// Training unit card with cost and train button
#[component]
pub fn TrainingUnitCard(
    unit: UnitTrainingOption,
    slot_id: u8,
    building_name: BuildingName,
    csrf_token: String,
) -> Element {
    let time_formatted = format_duration(unit.time_secs);

    rsx! {
        div { class: "border rounded-md p-4 bg-white space-y-3",
            div { class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2",
                div {
                    div { class: "text-lg font-semibold text-gray-900", "{unit.name}" }
                    div { class: "text-xs text-gray-500",
                        "{t!(\"game.building.training_time\")}: {time_formatted}"
                    }
                }
                div { class: "text-xs text-gray-500",
                    "{t!(\"game.building.training_upkeep\")} {unit.upkeep}"
                }
            }

            div {
                div { class: "text-xs uppercase text-gray-500", "{t!(\"game.building.training_cost\")}" }
                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-2 mt-2 text-sm",
                    div { class: "flex items-center justify-between",
                        span { "🌲 {t!(\"game.village.resources.lumber\")}" }
                        span { class: "font-semibold", "{unit.cost.lumber()}" }
                    }
                    div { class: "flex items-center justify-between",
                        span { "🧱 {t!(\"game.village.resources.clay\")}" }
                        span { class: "font-semibold", "{unit.cost.clay()}" }
                    }
                    div { class: "flex items-center justify-between",
                        span { "⚒️ {t!(\"game.village.resources.iron\")}" }
                        span { class: "font-semibold", "{unit.cost.iron()}" }
                    }
                    div { class: "flex items-center justify-between",
                        span { "🌾 {t!(\"game.village.resources.crop\")}" }
                        span { class: "font-semibold", "{unit.cost.crop()}" }
                    }
                }
            }

            form {
                action: "/army/train?s={slot_id}",
                method: "post",
                class: "flex flex-col sm:flex-row sm:items-end gap-3",
                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                input { r#type: "hidden", name: "unit_idx", value: "{unit.unit_idx}" }
                input { r#type: "hidden", name: "building_name", value: "{building_name}" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                label { class: "flex-1 text-sm text-gray-600",
                    "{t!(\"game.building.training_quantity\")}"
                    input {
                        r#type: "number",
                        min: "1",
                        max: if let Some(max) = unit.max_quantity { "{max}" } else { "" },
                        name: "quantity",
                        value: "1",
                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                    }
                }
                button {
                    r#type: "submit",
                    class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                    "{t!(\"game.building.train_action\")}"
                }
            }
        }
    }
}

/// Training queue display
#[component]
pub fn TrainingQueue(queue: Vec<TrainingQueueItem>) -> Element {
    rsx! {
        div { class: "border rounded-md p-4 bg-gray-50 space-y-2",
            div { class: "text-sm text-gray-500 uppercase",
                "{t!(\"game.building.training_queue_title\")}"
            }
            for job in queue.iter() {
                {
                    let time_remaining = format_duration(job.time_remaining_secs);
                    rsx! {
                        div { class: "p-3 bg-white border rounded-md space-y-1 text-sm",
                            div { class: "flex items-center justify-between font-semibold text-gray-800",
                                span { "{job.quantity} × {job.unit_name}" }
                                span { class: "text-xs text-gray-500",
                                    "{t!(\"game.building.training_time\")} {job.time_per_unit}s"
                                }
                            }
                            div { class: "flex items-center justify-between text-xs text-gray-600",
                                span { "{t!(\"game.building.training_remaining\")}" }
                                span {
                                    class: "font-mono countdown-timer",
                                    "data-seconds": "{job.time_remaining_secs}",
                                    "{time_remaining}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
