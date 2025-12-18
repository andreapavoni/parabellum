use dioxus::prelude::*;
use parabellum_types::{army::TroopSet, map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::{components::BattleArmyTable, view_helpers::unit_display_name};

/// Recall Confirmation Page - allows user to edit quantities before recalling
#[component]
pub fn RecallConfirmationPage(
    village_id: u32,
    village_name: String,
    army_id: Uuid,
    destination_village_name: Option<String>,
    destination_position: Position,
    units: TroopSet,
    tribe: Tribe,
    csrf_token: String,
) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "Recall Troops"
            }
            p { class: "text-gray-600 mb-6",
                "From: {destination_village_name.clone().unwrap_or_else(|| \"Unknown\".to_string())} ({destination_position.x}|{destination_position.y})"
            }

            // Summary section
            div { class: "border rounded-md p-4 bg-white space-y-4 mb-6",
                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "Current Deployment"
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "Deployed at:" }
                    span { "{destination_village_name.clone().unwrap_or_else(|| \"Unknown\".to_string())} ({destination_position.x}|{destination_position.y})" }
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "Returning to:" }
                    span { "{village_name} " }
                }

                // Current troops
                div { class: "mt-4",
                    h3 { class: "text-sm font-semibold text-gray-700 mb-2",
                        "Available Troops"
                    }
                    BattleArmyTable {
                        tribe: tribe.clone(),
                        army_before: units.clone(),
                        losses: TroopSet::default(),
                    }
                }
            }

            // Editable recall form
            form {
                action: "/army/recall",
                method: "post",
                class: "border rounded-md p-4 bg-white space-y-4",

                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "Select Troops to Recall"
                }
                p { class: "text-sm text-gray-600 mb-4",
                    "Adjust the quantities below to recall specific troops. Set to 0 to leave them deployed."
                }

                // Hidden fields
                input { r#type: "hidden", name: "movement_id", value: "{army_id}" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                // Editable unit quantities
                div { class: "space-y-3",
                    for (idx, &quantity) in units.units().clone().iter().enumerate() {
                        if quantity > 0 {
                            {
                                let unit = &tribe.units()[idx];
                                rsx! {
                                    div { class: "flex items-center gap-3 py-2 border-b",
                                        // Unit icon/name
                                        div { class: "flex-1 flex items-center gap-2",
                                            span { class: "font-medium text-gray-700",
                                                "{unit_display_name(&unit.name)}"
                                            }
                                            span { class: "text-xs text-gray-500",
                                                "(Max: {quantity})"
                                            }
                                        }

                                        // Input field
                                        div { class: "flex items-center gap-2",
                                            input {
                                                r#type: "number",
                                                name: "units[]",
                                                value: "{quantity}",
                                                min: "0",
                                                max: "{quantity}",
                                                class: "w-24 border rounded px-3 py-1 text-center"
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Hidden input for zero quantities to maintain array structure
                            input { r#type: "hidden", name: "units[]", value: "0" }
                        }
                    }
                }

                // Action buttons
                div { class: "flex gap-3 mt-6",
                    button {
                        r#type: "submit",
                        class: "px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition",
                        "Confirm Recall"
                    }
                    a {
                        href: "/build/39",
                        class: "px-6 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400 transition",
                        "Cancel"
                    }
                }
            }
        }
    }
}

/// Release Confirmation Page - allows user to edit quantities before releasing
#[component]
pub fn ReleaseConfirmationPage(
    village_id: u32,
    village_name: String,
    army_id: Uuid,
    source_village_id: u32,
    source_village_name: String,
    source_position: Position,
    units: TroopSet,
    tribe: Tribe,
    csrf_token: String,
) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "Release Reinforcements"
            }
            p { class: "text-gray-600 mb-6",
                "To: {source_village_name} ({source_position.x}|{source_position.y})"
            }

            // Summary section
            div { class: "border rounded-md p-4 bg-white space-y-4 mb-6",
                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "Current Reinforcement"
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "Stationed at:" }
                    span { "{village_name}" }
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "Returning to:" }
                    span { "{source_village_name} ({source_position.x}|{source_position.y})" }
                }

                // Current reinforcement troops
                div { class: "mt-4",
                    h3 { class: "text-sm font-semibold text-gray-700 mb-2",
                        "Available Troops"
                    }
                    BattleArmyTable {
                        tribe: tribe.clone(),
                        army_before: units.clone(),
                        losses: TroopSet::default(),
                    }
                }
            }

            // Editable release form
            form {
                action: "/army/release",
                method: "post",
                class: "border rounded-md p-4 bg-white space-y-4",

                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "Select Troops to Release"
                }
                p { class: "text-sm text-gray-600 mb-4",
                    "Adjust the quantities below to release specific troops. Set to 0 to keep them as reinforcements."
                }

                // Hidden fields
                input { r#type: "hidden", name: "source_village_id", value: "{source_village_id}" }
                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                // Editable unit quantities
                div { class: "space-y-3",
                    for (idx, &quantity) in units.units().clone().iter().enumerate() {
                        if quantity > 0 {
                            {
                                let unit = &tribe.units()[idx];
                                rsx! {
                                    div { class: "flex items-center gap-3 py-2 border-b",
                                        // Unit icon/name
                                        div { class: "flex-1 flex items-center gap-2",
                                            span { class: "font-medium text-gray-700",
                                                "{unit_display_name(&unit.name)}"
                                            }
                                            span { class: "text-xs text-gray-500",
                                                "(Max: {quantity})"
                                            }
                                        }

                                        // Input field
                                        div { class: "flex items-center gap-2",
                                            input {
                                                r#type: "number",
                                                name: "units[]",
                                                value: "{quantity}",
                                                min: "0",
                                                max: "{quantity}",
                                                class: "w-24 border rounded px-3 py-1 text-center"
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Hidden input for zero quantities to maintain array structure
                            input { r#type: "hidden", name: "units[]", value: "0" }
                        }
                    }
                }

                // Action buttons
                div { class: "flex gap-3 mt-6",
                    button {
                        r#type: "submit",
                        class: "px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition",
                        "Confirm Release"
                    }
                    a {
                        href: "/build/39",
                        class: "px-6 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400 transition",
                        "Cancel"
                    }
                }
            }
        }
    }
}
