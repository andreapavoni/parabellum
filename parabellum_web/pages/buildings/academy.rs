use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{
    components::{MissingRequirements, ResourceCost, UpgradeBlock},
    view_helpers::{building_description_paragraphs, format_duration},
};

/// Academy research option
#[derive(Clone, PartialEq)]
pub struct AcademyResearchOption {
    pub unit_name: String,
    pub unit_value: String,
    pub cost: ResourceGroup,
    pub time_secs: u32,
    pub missing_requirements: Vec<(BuildingName, u8)>,
}

/// Academy queue item
#[derive(Clone, PartialEq)]
pub struct AcademyQueueItem {
    pub unit_name: String,
    pub time_remaining_secs: u32,
    pub is_processing: bool,
}

/// Academy page - research units
#[component]
pub fn AcademyPage(
    village: Village,
    slot_id: u8,
    building_name: BuildingName,
    current_level: u8,
    population: u32,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    current_upkeep: u32,
    next_upkeep: u32,
    queue_full: bool,
    ready_units: Vec<AcademyResearchOption>,
    locked_units: Vec<AcademyResearchOption>,
    researched_units: Vec<String>,
    academy_queue: Vec<AcademyQueueItem>,
    academy_queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
    #[props(default = None)] next_value: Option<String>,
) -> Element {
    let stored = village.stored_resources();
    let description_paragraphs = building_description_paragraphs(&building_name);

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4", "{building_name} (Level {current_level})" }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4", "{error}" }
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
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm",
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.level\")}" }
                        div { class: "text-lg font-bold", "{current_level}" }
                    }
                    div { class: "p-3 border rounded-md bg-gray-50",
                        div { class: "text-gray-500", "{t!(\"game.building.population\")}" }
                        div { class: "text-lg font-bold", "{population}" }
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

                // Research queue
                if !academy_queue.is_empty() {
                    div { class: "border rounded-md p-4 bg-gray-50 space-y-3",
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.research_queue_title\")}" }
                        for job in academy_queue.iter() {
                            {
                                let time_formatted = format_duration(job.time_remaining_secs);
                                rsx! {
                                    div { class: "bg-white border rounded-md p-3 text-sm space-y-1",
                                        div { class: "flex items-center justify-between",
                                            span { class: "font-semibold text-gray-900", "{job.unit_name}" }
                                            span {
                                                class: if job.is_processing { "text-xs font-semibold text-emerald-600" } else { "text-xs font-semibold text-gray-500" },
                                                if job.is_processing { "{t!(\"game.building.research_in_progress\")}" } else { "{t!(\"game.building.research_pending\")}" }
                                            }
                                        }
                                        div { class: "flex items-center justify-between text-xs text-gray-600",
                                            span { "{t!(\"game.building.time_remaining\")}" }
                                            span { class: "font-mono countdown-timer", "data-seconds": "{job.time_remaining_secs}", "{time_formatted}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Ready to research
                div {
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.research_available\")}" }
                    if ready_units.is_empty() {
                        p { class: "text-sm text-gray-500 mt-2", "{t!(\"game.building.no_research_available\")}" }
                    } else {
                        if academy_queue_full {
                            div { class: "text-xs text-amber-700 border border-amber-200 bg-amber-50 rounded-md p-2 mt-2", "{t!(\"game.building.academy_queue_full_hint\")}" }
                        }
                        div { class: "space-y-4 mt-3",
                            for option in ready_units.iter() {
                                {
                                    let can_afford = stored.lumber() >= option.cost.lumber()
                                        && stored.clay() >= option.cost.clay()
                                        && stored.iron() >= option.cost.iron()
                                        && stored.crop() >= option.cost.crop();
                                    let can_research = can_afford && !academy_queue_full;
                                    let time_formatted = format_duration(option.time_secs);

                                    rsx! {
                                        div { class: "border rounded-md p-4 bg-white space-y-3",
                                            div { class: "flex items-center justify-between",
                                                div {
                                                    div { class: "text-lg font-semibold text-gray-900", "{option.unit_name}" }
                                                    div { class: "text-xs text-gray-500", "{t!(\"game.building.research_time\")}: {time_formatted}" }
                                                }
                                            }
                                            ResourceCost { cost: option.cost.clone() }
                                            form {
                                                action: "/academy/research",
                                                method: "post",
                                                class: "flex flex-col sm:flex-row sm:items-end gap-3",
                                                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                                                input { r#type: "hidden", name: "unit_name", value: "{option.unit_value}" }
                                                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                                button {
                                                    r#type: "submit",
                                                    class: if can_research { "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded" } else { "bg-emerald-600 text-white font-semibold px-4 py-2 rounded opacity-60 cursor-not-allowed" },
                                                    disabled: !can_research,
                                                    "{t!(\"game.building.research_action\")}"
                                                }
                                            }
                                            if !can_afford {
                                                div { class: "text-xs text-red-600", "{t!(\"game.building.not_enough_resources\")}" }
                                            } else if academy_queue_full {
                                                div { class: "text-xs text-amber-700", "{t!(\"game.building.academy_queue_full_hint\")}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Locked units
                if !locked_units.is_empty() {
                    div {
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.research_locked\")}" }
                        div { class: "space-y-4 mt-3",
                            for option in locked_units.iter() {
                                {
                                    let time_formatted = format_duration(option.time_secs);
                                    rsx! {
                                        div { class: "border rounded-md p-4 bg-white space-y-3 opacity-70",
                                            div { class: "flex items-center justify-between",
                                                div {
                                                    div { class: "text-lg font-semibold text-gray-900", "{option.unit_name}" }
                                                    div { class: "text-xs text-gray-500", "{t!(\"game.building.research_time\")}: {time_formatted}" }
                                                }
                                                span { class: "text-xs text-amber-700 font-semibold uppercase", "{t!(\"game.building.missing_requirements_hint\")}" }
                                            }
                                            MissingRequirements { requirements: option.missing_requirements.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Researched units
                if !researched_units.is_empty() {
                    div {
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.research_completed\")}" }
                        div { class: "flex flex-wrap gap-2 mt-2",
                            for unit_name in researched_units.iter() {
                                span { class: "px-3 py-1 bg-emerald-50 text-emerald-700 text-xs font-semibold rounded-full border border-emerald-200", "{unit_name}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
