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
    // Check if at max level - time_secs will be 0 when we can't upgrade
    // (handler sets it to 0 when upgrade_info is None)
    let at_max_level = time_secs == 0 && next_level == current_level + 1;

    if at_max_level {
        return rsx! {
            div { class: "border rounded-lg p-4 bg-gray-100 shadow-sm",
                h3 { class: "text-lg font-bold text-gray-800 mb-3",
                    "{t!(\"game.building.max_level_reached\")}"
                }
                p { class: "text-sm text-gray-600",
                    "{building_name:?} is at maximum level ({current_level})"
                }
            }
        };
    }

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
                "Upgrade to level {next_level}"
            }

            div { class: "mb-3",
                p { class: "text-sm text-gray-600 mb-2", "Cost:" }
                ResourceCost { cost: cost }
            }

            div { class: "grid grid-cols-2 gap-2 text-sm mb-3",
                div {
                    span { class: "text-gray-600", "Duration: " }
                    span { class: "font-semibold", "{time_formatted}" }
                }
                div {
                    span { class: "text-gray-600", "Upkeep: " }
                    span { class: "font-semibold", "{current_upkeep} → {next_upkeep}" }
                }
            }

            if queue_full {
                p { class: "text-sm text-yellow-600 mb-2",
                    "⚠️ Queue is full"
                }
            }

            if !can_afford {
                p { class: "text-sm text-red-600 mb-2",
                    "❌ Insufficient resources"
                }
            }

            form {
                method: "post",
                action: "/build/{slot_id}",
                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                input { r#type: "hidden", name: "action", value: "upgrade" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                button {
                    r#type: "submit",
                    class: "w-full text-white font-semibold py-2 px-4 rounded",
                    style: if can_upgrade {
                        "background-color: #16a34a;"
                    } else {
                        "background-color: #9ca3af; cursor: not-allowed; opacity: 0.7;"
                    },
                    disabled: !can_upgrade,
                    "Upgrade"
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

/// Smithy upgrade option
#[derive(Clone, PartialEq)]
pub struct SmithyUpgradeOption {
    pub unit_name: String,
    pub unit_value: String,
    pub current_level: u8,
    pub max_level: u8,
    pub cost: ResourceGroup,
    pub time_secs: u32,
    pub can_upgrade: bool,
}

/// Smithy queue item
#[derive(Clone, PartialEq)]
pub struct SmithyQueueItem {
    pub unit_name: String,
    pub target_level: u8,
    pub time_remaining_secs: u32,
    pub is_processing: bool,
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
                    action: "/build/{slot_id}",
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

/// Academy page - research units
#[component]
pub fn AcademyPage(
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
    ready_units: Vec<AcademyResearchOption>,
    locked_units: Vec<AcademyResearchOption>,
    researched_units: Vec<String>,
    academy_queue: Vec<AcademyQueueItem>,
    academy_queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    let stored = village.stored_resources();

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4", "{building_name:?} (Level {current_level})" }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4", "{error}" }
            }

            div { class: "space-y-6",
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

/// Smithy page - upgrade units
#[component]
pub fn SmithyPage(
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
    smithy_units: Vec<SmithyUpgradeOption>,
    smithy_queue: Vec<SmithyQueueItem>,
    smithy_queue_full: bool,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    let stored = village.stored_resources();

    rsx! {
        div { class: "container mx-auto p-4 max-w-4xl",
            h1 { class: "text-2xl font-bold mb-4", "{building_name:?} (Level {current_level})" }

            if let Some(error) = flash_error {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4", "{error}" }
            }

            div { class: "space-y-6",
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

                // Smithy queue
                if !smithy_queue.is_empty() {
                    div { class: "border rounded-md p-4 bg-gray-50 space-y-3",
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.smithy_queue_title\")}" }
                        for job in smithy_queue.iter() {
                            {
                                let time_formatted = format_duration(job.time_remaining_secs);
                                rsx! {
                                    div { class: "bg-white border rounded-md p-3 text-sm space-y-1",
                                        div { class: "flex items-center justify-between",
                                            span { class: "font-semibold text-gray-900", "{job.unit_name} → Lv {job.target_level}" }
                                            span {
                                                class: if job.is_processing { "text-xs font-semibold text-emerald-600" } else { "text-xs font-semibold text-gray-500" },
                                                if job.is_processing { "{t!(\"game.building.upgrade_in_progress\")}" } else { "{t!(\"game.building.upgrade_pending\")}" }
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

                // Upgradeable units
                div {
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.building.smithy_upgrades\")}" }
                    if smithy_units.is_empty() {
                        p { class: "text-sm text-gray-500 mt-2", "{t!(\"game.building.no_units_to_upgrade\")}" }
                    } else {
                        if smithy_queue_full {
                            div { class: "text-xs text-amber-700 border border-amber-200 bg-amber-50 rounded-md p-2 mt-2", "{t!(\"game.building.smithy_queue_full_hint\")}" }
                        }
                        div { class: "space-y-4 mt-3",
                            for option in smithy_units.iter() {
                                {
                                    let can_afford = stored.lumber() >= option.cost.lumber()
                                        && stored.clay() >= option.cost.clay()
                                        && stored.iron() >= option.cost.iron()
                                        && stored.crop() >= option.cost.crop();
                                    let can_upgrade = option.can_upgrade && can_afford && !smithy_queue_full;
                                    let time_formatted = format_duration(option.time_secs);

                                    rsx! {
                                        div { class: "border rounded-md p-4 bg-white space-y-3",
                                            div { class: "flex items-center justify-between",
                                                div {
                                                    div { class: "text-lg font-semibold text-gray-900", "{option.unit_name}" }
                                                    div { class: "text-xs text-gray-500",
                                                        "Level {option.current_level} → {option.current_level + 1} (Max: {option.max_level})"
                                                    }
                                                    div { class: "text-xs text-gray-500", "{t!(\"game.building.upgrade_time\")}: {time_formatted}" }
                                                }
                                            }
                                            ResourceCost { cost: option.cost.clone() }
                                            form {
                                                action: "/smithy/research",
                                                method: "post",
                                                class: "flex flex-col sm:flex-row sm:items-end gap-3",
                                                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                                                input { r#type: "hidden", name: "unit_name", value: "{option.unit_value}" }
                                                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                                button {
                                                    r#type: "submit",
                                                    class: if can_upgrade { "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded" } else { "bg-blue-600 text-white font-semibold px-4 py-2 rounded opacity-60 cursor-not-allowed" },
                                                    disabled: !can_upgrade,
                                                    "{t!(\"game.building.upgrade_action\")}"
                                                }
                                            }
                                            if !can_afford {
                                                div { class: "text-xs text-red-600", "{t!(\"game.building.not_enough_resources\")}" }
                                            } else if smithy_queue_full {
                                                div { class: "text-xs text-amber-700", "{t!(\"game.building.smithy_queue_full_hint\")}" }
                                            } else if !option.can_upgrade {
                                                div { class: "text-xs text-gray-500", "{t!(\"game.building.max_level_reached\")}" }
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

/// Rally Point page - send troops and view movements
#[component]
pub fn RallyPointPage(
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
    home_troops: Vec<super::common::TroopCount>,
    reinforcements: Vec<super::common::TroopCount>,
    sendable_units: Vec<super::common::RallyPointUnit>,
    incoming_movements: Vec<super::common::TroopMovement>,
    outgoing_movements: Vec<super::common::TroopMovement>,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    use super::common::{MovementDirection, MovementKind};

    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-6xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{building_name:?} (Level {current_level})"
            }
            p { class: "text-gray-600 mb-6",
                "{village.name} ({village.position.x}|{village.position.y})"
            }

            if let Some(error) = flash_error {
                div { class: "mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded",
                    "{error}"
                }
            }

            div { class: "space-y-6",
                // Upgrade block
                UpgradeBlock {
                    village: village.clone(),
                    building_name: building_name,
                    current_level: current_level,
                    next_level: next_level,
                    cost: cost,
                    time_secs: time_secs,
                    current_upkeep: current_upkeep,
                    next_upkeep: next_upkeep,
                    queue_full: queue_full,
                    slot_id: slot_id,
                    csrf_token: csrf_token.clone(),
                }

                // Troops overview - full army table
                div { class: "border rounded-md p-4 bg-white space-y-3",
                    div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.rally_point.troops_overview\")}" }

                    // Table showing all units with icons
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b",
                                    th { class: "text-left p-2 text-xs font-semibold text-gray-600", "Unit" }
                                    th { class: "text-right p-2 text-xs font-semibold text-gray-600", "Home" }
                                    th { class: "text-right p-2 text-xs font-semibold text-gray-600", "Support" }
                                    th { class: "text-right p-2 text-xs font-semibold text-gray-600", "Total" }
                                }
                            }
                            tbody {
                                for unit in sendable_units.iter() {
                                    {
                                        // Find reinforcement count for this unit
                                        let support_count = reinforcements.iter()
                                            .find(|r| r.name == unit.name)
                                            .map(|r| r.count)
                                            .unwrap_or(0);
                                        let total = unit.available + support_count;

                                        // Only show row if there are any troops
                                        if total > 0 {
                                            rsx! {
                                                tr { class: "border-b hover:bg-gray-50",
                                                    td { class: "p-2 font-medium text-gray-800", "{unit.name}" }
                                                    td { class: "p-2 text-right tabular-nums",
                                                        if unit.available > 0 {
                                                            span { class: "text-gray-900", "{unit.available}" }
                                                        } else {
                                                            span { class: "text-gray-400", "—" }
                                                        }
                                                    }
                                                    td { class: "p-2 text-right tabular-nums",
                                                        if support_count > 0 {
                                                            span { class: "text-blue-600", "{support_count}" }
                                                        } else {
                                                            span { class: "text-gray-400", "—" }
                                                        }
                                                    }
                                                    td { class: "p-2 text-right tabular-nums font-semibold", "{total}" }
                                                }
                                            }
                                        } else {
                                            rsx! { }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Summary
                    div { class: "pt-2 border-t text-xs text-gray-500",
                        p { "🏠 Home troops  •  🛡️ Support troops (reinforcements from other villages)" }
                    }
                }

                // Incoming and outgoing movements
                div { class: "grid gap-4 md:grid-cols-2",
                    // Incoming movements
                    div { class: "border rounded-md p-4 bg-white space-y-3",
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.rally_point.incoming_movements\")}" }
                        if incoming_movements.is_empty() {
                            p { class: "text-sm text-gray-500", "{t!(\"game.rally_point.no_movements\")}" }
                        } else {
                            div { class: "space-y-3",
                                for movement in incoming_movements.iter() {
                                    {
                                        let direction_text = match movement.direction {
                                            MovementDirection::Incoming => t!("game.rally_point.direction.incoming"),
                                            MovementDirection::Outgoing => t!("game.rally_point.direction.outgoing"),
                                        };
                                        let kind_text = match movement.kind {
                                            MovementKind::Attack => t!("game.rally_point.movement.attack"),
                                            MovementKind::Raid => t!("game.rally_point.movement.raid"),
                                            MovementKind::Reinforcement => t!("game.rally_point.movement.reinforcement"),
                                            MovementKind::Return => t!("game.rally_point.movement.return"),
                                        };
                                        rsx! {
                                            div { class: "border rounded-md p-3 bg-gray-50 space-y-1 text-sm",
                                                div { class: "font-semibold text-gray-800",
                                                    "{direction_text} — {kind_text}"
                                                }
                                                div { class: "text-xs text-gray-600",
                                                    "{movement.origin_name} ({movement.origin_x}|{movement.origin_y}) → {movement.destination_name} ({movement.destination_x}|{movement.destination_y})"
                                                }
                                                div { class: "text-xs text-gray-500 flex items-center justify-between",
                                                    span { "{t!(\"game.rally_point.arrives_in\")}" }
                                                    span {
                                                        class: "font-mono countdown-timer",
                                                        "data-seconds": "{movement.time_seconds}",
                                                        "{movement.time_remaining}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Outgoing movements
                    div { class: "border rounded-md p-4 bg-white space-y-3",
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.rally_point.outgoing_movements\")}" }
                        if outgoing_movements.is_empty() {
                            p { class: "text-sm text-gray-500", "{t!(\"game.rally_point.no_movements\")}" }
                        } else {
                            div { class: "space-y-3",
                                for movement in outgoing_movements.iter() {
                                    {
                                        let direction_text = match movement.direction {
                                            MovementDirection::Incoming => t!("game.rally_point.direction.incoming"),
                                            MovementDirection::Outgoing => t!("game.rally_point.direction.outgoing"),
                                        };
                                        let kind_text = match movement.kind {
                                            MovementKind::Attack => t!("game.rally_point.movement.attack"),
                                            MovementKind::Raid => t!("game.rally_point.movement.raid"),
                                            MovementKind::Reinforcement => t!("game.rally_point.movement.reinforcement"),
                                            MovementKind::Return => t!("game.rally_point.movement.return"),
                                        };
                                        rsx! {
                                            div { class: "border rounded-md p-3 bg-gray-50 space-y-1 text-sm",
                                                div { class: "font-semibold text-gray-800",
                                                    "{direction_text} — {kind_text}"
                                                }
                                                div { class: "text-xs text-gray-600",
                                                    "{movement.origin_name} ({movement.origin_x}|{movement.origin_y}) → {movement.destination_name} ({movement.destination_x}|{movement.destination_y})"
                                                }
                                                div { class: "text-xs text-gray-500 flex items-center justify-between",
                                                    span { "{t!(\"game.rally_point.arrives_in\")}" }
                                                    span {
                                                        class: "font-mono countdown-timer",
                                                        "data-seconds": "{movement.time_seconds}",
                                                        "{movement.time_remaining}"
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

                // Send troops form
                div { class: "border rounded-md p-4 bg-white space-y-4",
                    div {
                        div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.rally_point.send_troops\")}" }
                        p { class: "text-sm text-gray-500", "{t!(\"game.rally_point.send_hint\")}" }
                    }
                    form {
                        action: "/army/send?s={slot_id}",
                        method: "post",
                        class: "space-y-4",
                        input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                        input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                        div { class: "grid gap-3 sm:grid-cols-3",
                            label { class: "text-sm text-gray-600",
                                "{t!(\"game.rally_point.target_x\")}"
                                input {
                                    r#type: "number",
                                    name: "target_x",
                                    required: true,
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "{t!(\"game.rally_point.target_y\")}"
                                input {
                                    r#type: "number",
                                    name: "target_y",
                                    required: true,
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "{t!(\"game.rally_point.movement_type\")}"
                                select {
                                    name: "movement",
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700",
                                    option { value: "attack", "{t!(\"game.rally_point.movement.attack\")}" }
                                    option { value: "raid", "{t!(\"game.rally_point.movement.raid\")}" }
                                    option { value: "reinforcement", "{t!(\"game.rally_point.movement.reinforcement\")}" }
                                }
                            }
                        }

                        div { class: "space-y-2",
                            div { class: "text-sm text-gray-500 uppercase", "{t!(\"game.rally_point.select_units\")}" }
                            for unit in sendable_units.iter() {
                                label {
                                    class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-sm text-gray-700 border rounded-md px-3 py-2",
                                    span { class: "font-semibold", "{unit.name}" }
                                    span { class: "text-xs text-gray-500", "{t!(\"game.rally_point.available\")}: {unit.available}" }
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        max: "{unit.available}",
                                        name: "units[]",
                                        value: "0",
                                        class: "w-full sm:w-32 border rounded px-2 py-1 text-gray-700"
                                    }
                                }
                            }
                        }

                        button {
                            r#type: "submit",
                            class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                            "{t!(\"game.rally_point.send_button\")}"
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
