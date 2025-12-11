use dioxus::prelude::*;
use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};
use rust_i18n::t;

use crate::components::BattleArmyTable;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationType {
    /// Simple confirmation - just confirm troops to send
    Simple,
    /// Choose scouting target: resources or defenses
    ScoutingChoice,
    /// Choose catapult targets
    CatapultTargets {
        available_buildings: Vec<BuildingName>,
    },
}

/// Send Troops Confirmation Page
#[component]
pub fn SendTroopsConfirmationPage(
    village_id: u32,
    village_name: String,
    village_position: Position,
    target_position: Position,
    movement_type: String,
    movement_type_value: String,
    tribe: Tribe,
    troops: [u32; 10],
    confirmation_type: ConfirmationType,
    csrf_token: String,
    slot_id: u8,
) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{t!(\"game.rally_point.confirm_troops\")}"
            }
            p { class: "text-gray-600 mb-6",
                "{village_name} ({village_position.x}|{village_position.y})"
            }

            // Summary section
            div { class: "border rounded-md p-4 bg-white space-y-4 mb-6",
                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "{t!(\"game.rally_point.attack_summary\")}"
                }

                // Target coordinates
                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.rally_point.target\")}:" }
                    span { "({target_position.x}|{target_position.y})" }
                }

                // Movement type
                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.rally_point.movement_type\")}:" }
                    span { "{movement_type}" }
                }

                // Troops table
                div { class: "mt-4",
                    h3 { class: "text-sm font-semibold text-gray-700 mb-2",
                        "{t!(\"game.rally_point.troops_to_send\")}"
                    }
                    BattleArmyTable {
                        tribe: tribe.clone(),
                        army_before: troops,
                        losses: [0; 10],
                    }
                }
            }

            // Confirmation form based on type
            form {
                action: "/army/send/confirm",
                method: "post",
                class: "border rounded-md p-4 bg-white space-y-4",

                // Hidden fields to preserve the original form data
                input { r#type: "hidden", name: "village_id", value: "{village_id}" }
                input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                input { r#type: "hidden", name: "target_x", value: "{target_position.x}" }
                input { r#type: "hidden", name: "target_y", value: "{target_position.y}" }
                input { r#type: "hidden", name: "movement", value: "{movement_type_value}" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                // Send troops as hidden fields (all 10 unit slots)
                for quantity in troops.iter() {
                    input { r#type: "hidden", name: "units[]", value: "{quantity}" }
                }

                // Render specific confirmation UI based on type
                match confirmation_type {
                    ConfirmationType::Simple => rsx! {
                        p { class: "text-sm text-gray-600",
                            "{t!(\"game.rally_point.confirm_simple\")}"
                        }
                    },
                    ConfirmationType::ScoutingChoice => rsx! {
                        div { class: "space-y-3",
                            h3 { class: "text-sm font-semibold text-gray-700",
                                "{t!(\"game.rally_point.scouting_target\")}"
                            }
                            label { class: "flex items-center gap-2 text-sm text-gray-700",
                                input {
                                    r#type: "radio",
                                    name: "scouting_target",
                                    value: "resources",
                                    checked: true,
                                    class: "form-radio"
                                }
                                span { "{t!(\"game.rally_point.scout_resources\")}" }
                            }
                            label { class: "flex items-center gap-2 text-sm text-gray-700",
                                input {
                                    r#type: "radio",
                                    name: "scouting_target",
                                    value: "defenses",
                                    class: "form-radio"
                                }
                                span { "{t!(\"game.rally_point.scout_defenses\")}" }
                            }
                        }
                    },
                    ConfirmationType::CatapultTargets { available_buildings } => rsx! {
                        div { class: "space-y-3",
                            h3 { class: "text-sm font-semibold text-gray-700",
                                "{t!(\"game.rally_point.catapult_targets\")}"
                            }
                            p { class: "text-xs text-gray-500",
                                "{t!(\"game.rally_point.catapult_hint\")}"
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-2",
                                // Target 1
                                label { class: "text-sm text-gray-700",
                                    span { class: "font-semibold", "{t!(\"game.rally_point.target\")} 1:" }
                                    select {
                                        name: "catapult_target_1",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700",
                                        for building in available_buildings.iter() {
                                            option { value: "{building:?}", "{building:?}" }
                                        }
                                    }
                                }
                                // Target 2
                                label { class: "text-sm text-gray-700",
                                    span { class: "font-semibold", "{t!(\"game.rally_point.target\")} 2:" }
                                    select {
                                        name: "catapult_target_2",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700",
                                        for building in available_buildings.iter() {
                                            option { value: "{building:?}", "{building:?}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }

                // Action buttons
                div { class: "flex gap-3 mt-6",
                    button {
                        r#type: "submit",
                        class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                        "{t!(\"game.rally_point.confirm_send\")}"
                    }
                    a {
                        href: "/build/{slot_id}",
                        class: "bg-gray-500 hover:bg-gray-600 text-white font-semibold px-4 py-2 rounded",
                        "{t!(\"game.rally_point.cancel\")}"
                    }
                }
            }
        }
    }
}
