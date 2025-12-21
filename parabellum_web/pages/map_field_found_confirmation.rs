use dioxus::prelude::*;
use parabellum_types::{army::TroopSet, map::Position, tribe::Tribe};
use rust_i18n::t;

use crate::components::BattleArmyTable;

/// Confirmation page for founding a new village
#[component]
pub fn FoundVillageConfirmationPage(
    village_id: u32,
    village_name: String,
    village_position: Position,
    target_field_id: u32,
    target_position: Position,
    tribe: Tribe,
    settlers: TroopSet,
    csrf_token: String,
) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{t!(\"game.expansion.confirm_founding\")}"
            }
            p { class: "text-gray-600 mb-6",
                "{village_name} ({village_position.x}|{village_position.y})"
            }

            // Summary section
            div { class: "border rounded-md p-4 bg-white space-y-4 mb-6",
                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "{t!(\"game.expansion.founding_summary\")}"
                }

                // Target coordinates
                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.expansion.target_valley\")}:" }
                    span { "({target_position.x}|{target_position.y})" }
                }

                // Settlers
                div { class: "mt-4",
                    h3 { class: "text-sm font-semibold text-gray-700 mb-2",
                        "{t!(\"game.expansion.settlers_to_send\")}"
                    }
                    BattleArmyTable {
                        tribe: tribe.clone(),
                        army_before: settlers.clone(),
                        losses: TroopSet::default(),
                    }
                }

                div { class: "p-4 bg-amber-50 border border-amber-200 rounded-md mt-4",
                    p { class: "text-sm text-amber-800",
                        "{t!(\"game.expansion.founding_warning\")}"
                    }
                }
            }

            // Confirmation form
            form {
                action: "/map/field/{target_field_id}/found/execute",
                method: "post",
                class: "border rounded-md p-4 bg-white space-y-4",

                input { r#type: "hidden", name: "village_id", value: "{village_id}" }
                input { r#type: "hidden", name: "target_field_id", value: "{target_field_id}" }
                input { r#type: "hidden", name: "target_x", value: "{target_position.x}" }
                input { r#type: "hidden", name: "target_y", value: "{target_position.y}" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                // Send settlers as hidden fields (all 10 unit slots)
                for quantity in settlers.units().iter() {
                    input { r#type: "hidden", name: "units[]", value: "{quantity}" }
                }

                p { class: "text-sm text-gray-600",
                    "{t!(\"game.expansion.confirm_founding_question\")}"
                }

                // Action buttons
                div { class: "flex gap-3 mt-6",
                    button {
                        r#type: "submit",
                        class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                        "{t!(\"game.expansion.confirm_found_village\")}"
                    }
                    a {
                        href: "/map/field/{target_field_id}",
                        class: "bg-gray-500 hover:bg-gray-600 text-white font-semibold px-4 py-2 rounded",
                        "{t!(\"game.common.cancel\")}"
                    }
                }
            }
        }
    }
}
