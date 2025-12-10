use crate::{
    components::{ArmyCard, ArmyCategory, UpgradeBlock},
    view_helpers::{prepare_rally_point_cards, unit_display_name},
};
use dioxus::prelude::*;
use parabellum_app::repository::VillageInfo;
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;
use std::collections::HashMap;

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
    movements: parabellum_app::cqrs::queries::VillageTroopMovements,
    village_info: HashMap<u32, VillageInfo>,
    csrf_token: String,
    flash_error: Option<String>,
) -> Element {
    // Prepare all army cards using the view helper
    let army_cards = prepare_rally_point_cards(&village, &movements, &village_info);

    // Prepare sendable units from village army
    let available_units = village.army().map(|army| *army.units()).unwrap_or([0; 10]);
    let tribe_units = village.tribe.units();

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

                // Army overview - grouped by category
                div { class: "space-y-4",
                    // Stationed troops (own troops in this village)
                    {
                        let stationed = army_cards.iter().filter(|c| c.category == ArmyCategory::Stationed).collect::<Vec<_>>();
                        if !stationed.is_empty() {
                            rsx! {
                                div { class: "space-y-2",
                                    h3 { class: "text-sm font-semibold text-gray-700", "Own Troops in Village" }
                                    div { class: "space-y-2",
                                        for card in stationed {
                                            ArmyCard { card: card.clone(), csrf_token: csrf_token.clone() }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { }
                        }
                    }

                    // Deployed armies (own troops in other villages/oases)
                    {
                        let deployed = army_cards.iter().filter(|c| c.category == ArmyCategory::Deployed).collect::<Vec<_>>();
                        if !deployed.is_empty() {
                            rsx! {
                                div { class: "space-y-2",
                                    h3 { class: "text-sm font-semibold text-gray-700", "Own Troops Deployed" }
                                    div { class: "space-y-2",
                                        for card in deployed {
                                            ArmyCard { card: card.clone(), csrf_token: csrf_token.clone() }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { }
                        }
                    }

                    // Reinforcements (foreign troops helping us)
                    {
                        let reinforcements = army_cards.iter().filter(|c| c.category == ArmyCategory::Reinforcement).collect::<Vec<_>>();
                        if !reinforcements.is_empty() {
                            rsx! {
                                div { class: "space-y-2",
                                    h3 { class: "text-sm font-semibold text-gray-700", "Reinforcements Here" }
                                    div { class: "space-y-2",
                                        for card in reinforcements {
                                            ArmyCard { card: card.clone(), csrf_token: csrf_token.clone() }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { }
                        }
                    }

                    // Outgoing movements (troops going out)
                    {
                        let outgoing = army_cards.iter().filter(|c| c.category == ArmyCategory::Outgoing).collect::<Vec<_>>();
                        if !outgoing.is_empty() {
                            rsx! {
                                div { class: "space-y-2",
                                    h3 { class: "text-sm font-semibold text-gray-700", "Troops Going" }
                                    div { class: "space-y-2",
                                        for card in outgoing {
                                            ArmyCard { card: card.clone(), csrf_token: csrf_token.clone() }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { }
                        }
                    }

                    // Incoming movements (troops coming in)
                    {
                        let incoming = army_cards.iter().filter(|c| c.category == ArmyCategory::Incoming).collect::<Vec<_>>();
                        if !incoming.is_empty() {
                            rsx! {
                                div { class: "space-y-2",
                                    h3 { class: "text-sm font-semibold text-gray-700", "Troops Coming" }
                                    div { class: "space-y-2",
                                        for card in incoming {
                                            ArmyCard { card: card.clone(), csrf_token: csrf_token.clone() }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { }
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
                        action: "/army/send",
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
                            for (idx, unit) in tribe_units.iter().enumerate() {
                                {
                                    let available = available_units[idx];
                                    let name = unit_display_name(&unit.name);
                                    rsx! {
                                        label {
                                            class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-sm text-gray-700 border rounded-md px-3 py-2",
                                            span { class: "font-semibold", "{name}" }
                                            span { class: "text-xs text-gray-500", "{t!(\"game.rally_point.available\")}: {available}" }
                                            input {
                                                r#type: "number",
                                                min: "0",
                                                max: "{available}",
                                                name: "units[]",
                                                value: "0",
                                                class: "w-full sm:w-32 border rounded px-2 py-1 text-gray-700"
                                            }
                                        }
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
