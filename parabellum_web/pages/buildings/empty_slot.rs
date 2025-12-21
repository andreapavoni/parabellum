use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::view_helpers::{building_description_paragraphs, format_duration};

/// Building option card component
#[component]
pub fn BuildingOptionCard(
    option: BuildingOption,
    can_start: bool,
    can_afford: bool,
    queue_full: bool,
    slot_id: u8,
    csrf_token: String,
    locked: bool,
    time_formatted: String,
) -> Element {
    let opacity_class = if locked { "opacity-70" } else { "" };
    let description_paragraphs = building_description_paragraphs(&option.name);

    rsx! {
        div { class: "p-3 border rounded-md bg-gray-50 text-left text-sm text-gray-700 space-y-3 {opacity_class}",
            div { class: "text-base font-semibold text-gray-800", "{option.name:?}" }

            // Building description
            if !description_paragraphs.is_empty() {
                div { class: "text-xs text-gray-600 leading-relaxed space-y-1",
                    for paragraph in description_paragraphs.iter() {
                        p { "{paragraph}" }
                    }
                }
            }

            // Requirements
            if !option.missing_requirements.is_empty() {
                div { class: "text-xs bg-amber-50 border border-amber-200 text-amber-800 rounded-md p-2",
                    div { class: "font-semibold tracking-wide uppercase", "{t!(\"game.building.requirements_title\")}" }
                    ul { class: "list-disc ml-4 mt-1 space-y-0.5",
                        for (building_name , level) in option.missing_requirements.iter() {
                            li { "{building_name:?} – Lv {level}" }
                        }
                    }
                }
            }

            // Cost
            div {
                div { class: "text-xs uppercase text-gray-500", "{t!(\"game.building.cost\")}" }
                div { class: "grid grid-cols-2 gap-2 mt-1",
                    div { class: "flex items-center justify-between gap-2",
                        span { "🌲 {t!(\"game.village.resources.lumber\")}" }
                        span { class: "font-semibold text-gray-900", "{option.cost.lumber()}" }
                    }
                    div { class: "flex items-center justify-between gap-2",
                        span { "🧱 {t!(\"game.village.resources.clay\")}" }
                        span { class: "font-semibold text-gray-900", "{option.cost.clay()}" }
                    }
                    div { class: "flex items-center justify-between gap-2",
                        span { "⚒️ {t!(\"game.village.resources.iron\")}" }
                        span { class: "font-semibold text-gray-900", "{option.cost.iron()}" }
                    }
                    div { class: "flex items-center justify-between gap-2",
                        span { "🌾 {t!(\"game.village.resources.crop\")}" }
                        span { class: "font-semibold text-gray-900", "{option.cost.crop()}" }
                    }
                }
            }

            // Time
            div { class: "flex items-center justify-between text-xs text-gray-500 pt-1",
                span { "{t!(\"game.building.time\")}" }
                span { class: "font-semibold text-gray-900", "{time_formatted}" }
            }

            // Build button
            if !locked {
                form {
                    method: "post",
                    action: "/build/{slot_id}",
                    input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                    input { r#type: "hidden", name: "action", value: "build" }
                    input { r#type: "hidden", name: "building_name", value: "{option.name:?}" }
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                    button {
                        r#type: "submit",
                        class: if can_start && !queue_full {
                            "w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-4 rounded"
                        } else {
                            "w-full bg-blue-600 text-white font-semibold py-2 px-4 rounded opacity-60 cursor-not-allowed"
                        },
                        disabled: !can_start || queue_full,
                        "{t!(\"game.building.construct_action\")}"
                    }
                }

                if !can_afford {
                    div { class: "text-xs text-red-600",
                        "{t!(\"game.building.insufficient_resources\")}"
                    }
                } else if !can_start {
                    div { class: "text-xs text-red-600",
                        "{t!(\"game.building.missing_requirements_hint\")}"
                    }
                } else if queue_full {
                    div { class: "text-xs text-amber-700",
                        "{t!(\"game.building.construction_queue_full_short\")}"
                    }
                }
            } else {
                div { class: "text-xs text-amber-700 font-semibold uppercase",
                    "{t!(\"game.building.missing_requirements_hint\")}"
                }
            }
        }
    }
}

/// Building option for empty slot selection
#[derive(Clone, PartialEq)]
pub struct BuildingOption {
    pub name: BuildingName,
    pub cost: ResourceGroup,
    pub time_secs: u32,
    pub missing_requirements: Vec<(BuildingName, u8)>,
}

/// Empty slot page - shows available buildings to construct
#[component]
pub fn EmptySlotPage(
    village: Village,
    slot_id: u8,
    buildable_buildings: Vec<BuildingOption>,
    locked_buildings: Vec<BuildingOption>,
    queue_full: bool,
    has_queue_for_slot: bool,
    queued_building: Option<(String, u8)>,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    let stored = village.stored_resources();

    let title = if has_queue_for_slot {
        t!("game.building.construction_in_progress_short")
    } else {
        t!("game.building.empty_slot")
    };
    let construction_label = queued_building
        .as_ref()
        .map(|(name, level)| {
            t!(
                "game.building.construction_in_progress",
                name = name,
                level = level
            )
            .to_string()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4",
                "{title}"
            }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    "{error}"
                }
            }

            div { class: "space-y-6 py-6",
                div { class: "text-center",
                    if has_queue_for_slot {
                        div { class: "inline-flex items-center gap-2 px-4 py-2 bg-amber-100 text-amber-800 rounded-md text-sm font-semibold",
                            "{t!(\"game.building.queue_locked\")}"
                        }
                        if queued_building.is_some() {
                            p { class: "text-sm text-gray-600 mt-2",
                                "{construction_label}"
                            }
                        }
                    } else {
                        p { class: "text-lg text-gray-600", "{t!(\"game.building.empty_slot\")}" }
                        p { class: "text-sm text-gray-500 mt-2", "{t!(\"game.building.empty_slot_hint\")}" }
                    }
                }

                if buildable_buildings.is_empty() && locked_buildings.is_empty() {
                    div { class: "text-center text-sm text-gray-500",
                        if has_queue_for_slot {
                            ""
                        } else {
                            "{t!(\"game.building.no_available\")}"
                        }
                    }
                } else {
                    div {
                        // Buildable buildings
                        if !buildable_buildings.is_empty() {
                            div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.available_ready\")}" }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3 mt-3",
                                for option in buildable_buildings.iter() {
                                    {
                                        let can_afford = stored.lumber() >= option.cost.lumber()
                                            && stored.clay() >= option.cost.clay()
                                            && stored.iron() >= option.cost.iron()
                                            && stored.crop() >= option.cost.crop();
                                        let can_start = can_afford && option.missing_requirements.is_empty();
                                        let time_formatted = format_duration(option.time_secs);

                                        rsx! {
                                            BuildingOptionCard {
                                                option: option.clone(),
                                                can_start: can_start,
                                                can_afford: can_afford,
                                                queue_full: queue_full,
                                                slot_id: slot_id,
                                                csrf_token: csrf_token.clone(),
                                                locked: false,
                                                time_formatted: time_formatted
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Locked buildings
                        if !locked_buildings.is_empty() {
                            div { class: "text-sm text-gray-500 uppercase mt-6", "{t!(\"game.building.available_locked\")}" }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3 mt-3",
                                for option in locked_buildings.iter() {
                                    {
                                        let time_formatted = format_duration(option.time_secs);

                                        rsx! {
                                            BuildingOptionCard {
                                                option: option.clone(),
                                                can_start: false,
                                                can_afford: false,
                                                queue_full: queue_full,
                                                slot_id: slot_id,
                                                csrf_token: csrf_token.clone(),
                                                locked: true,
                                                time_formatted: time_formatted
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
