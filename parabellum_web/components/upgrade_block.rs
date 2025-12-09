use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{components::ResourceCost, view_helpers::format_duration};

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
