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
