use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

/// Resource cost display component
#[component]
pub fn ResourceCost(cost: ResourceGroup) -> Element {
    rsx! {
        div { class: "flex gap-3 text-sm",
            span { class: "text-gray-700", "🌲 {cost.lumber()}" }
            span { class: "text-gray-700", "🧱 {cost.clay()}" }
            span { class: "text-gray-700", "⛏️ {cost.iron()}" }
            span { class: "text-gray-700", "🌾 {cost.crop()}" }
        }
    }
}

/// Building upgrade block component
/// Shows upgrade info with cost, time, and action button
#[component]
pub fn UpgradeBlock(
    village: Village,
    building_name: BuildingName,
    current_level: u8,
    next_level: u8,
    cost: ResourceGroup,
    time_secs: u32,
    current_upkeep: u32,
    next_upkeep: u32,
    queue_full: bool,
    slot_id: u8,
    csrf_token: String,
) -> Element {
    // Format time in component
    let time_formatted = format_duration(time_secs);

    // Check if can afford
    let stored = village.stored_resources();
    let can_afford = stored.lumber() >= cost.lumber()
        && stored.clay() >= cost.clay()
        && stored.iron() >= cost.iron()
        && stored.crop() >= cost.crop();

    let can_upgrade = can_afford && !queue_full;

    rsx! {
        div { class: "border rounded-lg p-4 bg-white shadow-sm",
            h3 { class: "text-lg font-bold text-gray-800 mb-3",
                "{t!(\"game.building.upgrade_to_level\", level = next_level)}"
            }

            div { class: "mb-3",
                p { class: "text-sm text-gray-600 mb-2", "{t!(\"game.building.cost\")}:" }
                ResourceCost { cost: cost }
            }

            div { class: "grid grid-cols-2 gap-2 text-sm mb-3",
                div {
                    span { class: "text-gray-600", "{t!(\"game.building.duration\")}: " }
                    span { class: "font-semibold", "{time_formatted}" }
                }
                div {
                    span { class: "text-gray-600", "{t!(\"game.building.upkeep\")}: " }
                    span { class: "font-semibold", "{current_upkeep} → {next_upkeep}" }
                }
            }

            if queue_full {
                p { class: "text-sm text-yellow-600 mb-2",
                    "⚠️ {t!(\"game.building.queue_full\")}"
                }
            }

            if !can_afford {
                p { class: "text-sm text-red-600 mb-2",
                    "❌ {t!(\"game.building.insufficient_resources\")}"
                }
            }

            form {
                method: "post",
                action: "/dioxus/build/{slot_id}",
                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                input { r#type: "hidden", name: "action", value: "upgrade" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                button {
                    r#type: "submit",
                    class: if can_upgrade {
                        "w-full bg-green-600 hover:bg-green-700 text-white font-semibold py-2 px-4 rounded"
                    } else {
                        "w-full bg-gray-300 text-gray-500 font-semibold py-2 px-4 rounded cursor-not-allowed"
                    },
                    disabled: !can_upgrade,
                    "{t!(\"game.building.upgrade\")}"
                }
            }
        }
    }
}

/// Missing requirements display component
#[component]
pub fn MissingRequirements(requirements: Vec<(BuildingName, u8)>) -> Element {
    if requirements.is_empty() {
        return rsx! { Fragment {} };
    }

    rsx! {
        div { class: "mt-2 text-sm text-red-600",
            p { class: "font-semibold", "❌ {t!(\"game.building.requirements\")}:" }
            ul { class: "list-disc list-inside",
                for (building_name , required_level) in requirements {
                    li { "{building_name:?} (Level {required_level})" }
                }
            }
        }
    }
}

/// Generic building page - shows upgrade block only
#[component]
pub fn GenericBuildingPage(
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
    // Get description based on building type
    let description = match building_name {
        BuildingName::Woodcutter => t!("game.buildings.woodcutter.description"),
        BuildingName::ClayPit => t!("game.buildings.clay_pit.description"),
        BuildingName::IronMine => t!("game.buildings.iron_mine.description"),
        BuildingName::Cropland => t!("game.buildings.cropland.description"),
        _ => t!(""),
    };

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

/// Building option for empty slot selection
#[derive(Clone, PartialEq)]
pub struct BuildingOption {
    pub name: BuildingName,
    pub cost: ResourceGroup,
    pub time_secs: u32,
    pub missing_requirements: Vec<(BuildingName, u8)>,
}

/// Unit training option
#[derive(Clone, PartialEq)]
pub struct UnitTrainingOption {
    pub unit_idx: u8,
    pub name: String,
    pub cost: ResourceGroup,
    pub upkeep: u32,
    pub time_secs: u32,
}

/// Training queue item
#[derive(Clone, PartialEq)]
pub struct TrainingQueueItem {
    pub quantity: u32,
    pub unit_name: String,
    pub time_per_unit: u32,
    pub time_remaining_secs: u32,
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
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    let stored = village.stored_resources();

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4",
                "{t!(\"game.building.empty_slot\")}"
            }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    "{error}"
                }
            }

            div { class: "space-y-6 py-6",
                div { class: "text-center",
                    p { class: "text-lg text-gray-600", "{t!(\"game.building.empty_slot\")}" }
                    p { class: "text-sm text-gray-500 mt-2", "{t!(\"game.building.empty_slot_hint\")}" }
                }

                if buildable_buildings.is_empty() && locked_buildings.is_empty() {
                    div { class: "text-center text-sm text-gray-500",
                        if has_queue_for_slot {
                            "{t!(\"game.building.queue_locked\")}"
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

/// Building option card component
#[component]
fn BuildingOptionCard(
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

    rsx! {
        div { class: "p-3 border rounded-md bg-gray-50 text-left text-sm text-gray-700 space-y-3 {opacity_class}",
            div { class: "text-base font-semibold text-gray-800", "{option.name:?}" }

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
                    action: "/dioxus/build/{slot_id}",
                    input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                    input { r#type: "hidden", name: "action", value: "build" }
                    input { r#type: "hidden", name: "building_name", value: "{option.name}" }
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

/// Training building page - for Barracks, Stable, Workshop
#[component]
pub fn TrainingBuildingPage(
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
    training_units: Vec<UnitTrainingOption>,
    training_queue: Vec<TrainingQueueItem>,
    csrf_token: String,
    flash_error: Option<String>,
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

            div { class: "space-y-6",
                // Building description
                div {
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.existing\")}" }
                    div { class: "text-2xl font-semibold", "{building_name:?}" }
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
                    csrf_token: csrf_token.clone()
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
fn TrainingUnitCard(
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
fn TrainingQueue(queue: Vec<TrainingQueueItem>) -> Element {
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

/// Helper function to format duration (copied from view_helpers for now)
fn format_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}
