use dioxus::prelude::*;
use parabellum_app::cqrs::queries::{MarketplaceData, MerchantMovementKind};
use parabellum_game::models::village::Village;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;

use crate::{
    components::UpgradeBlock,
    view_helpers::{
        MerchantMovementDirection, building_description_paragraphs, format_duration,
        format_resource_summary, prepare_global_offers, prepare_merchant_movements,
        prepare_own_offers,
    },
};

/// Marketplace page - send resources and create/accept offers
#[component]
pub fn MarketplacePage(
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
    marketplace_data: MarketplaceData,
    csrf_token: String,
    flash_error: Option<String>,
    #[props(default = None)] next_value: Option<String>,
) -> Element {
    let description_paragraphs = building_description_paragraphs(&building_name);
    let own_offers = prepare_own_offers(&marketplace_data);
    let global_offers = prepare_global_offers(&marketplace_data);
    let outgoing_movements = prepare_merchant_movements(
        &marketplace_data.outgoing_merchants,
        &marketplace_data.village_info,
        MerchantMovementDirection::Outgoing,
    );
    let incoming_movements = prepare_merchant_movements(
        &marketplace_data.incoming_merchants,
        &marketplace_data.village_info,
        MerchantMovementDirection::Incoming,
    );
    let mut merchant_movements: Vec<_> = outgoing_movements
        .into_iter()
        .chain(incoming_movements.into_iter())
        .collect();
    merchant_movements.sort_by_key(|movement| movement.time_remaining_secs);
    let available_merchants = village.available_merchants();

    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-6xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{building_name} (Level {current_level})"
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
                // Building description and stats
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
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm mb-4",
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
                    next_value: next_value.clone(),
                }

                // Send resources form
                div { class: "border rounded-md p-4 bg-white space-y-4",
                    div {
                        div { class: "text-sm text-gray-500 uppercase", "Send resources" }
                        p { class: "text-sm text-gray-500",
                            "Available merchants: {available_merchants}/{village.total_merchants}"
                        }
                    }
                    form {
                        action: "/marketplace/send",
                        method: "post",
                        class: "space-y-4",
                        input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                        input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                        div { class: "grid gap-3 sm:grid-cols-2",
                            label { class: "text-sm text-gray-600",
                                "Target X"
                                input {
                                    r#type: "number",
                                    name: "target_x",
                                    required: true,
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "Target Y"
                                input {
                                    r#type: "number",
                                    name: "target_y",
                                    required: true,
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                        }

                        div { class: "grid gap-3 sm:grid-cols-4",
                            label { class: "text-sm text-gray-600",
                                "Lumber"
                                input {
                                    r#type: "number",
                                    min: "0",
                                    name: "lumber",
                                    value: "0",
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "Clay"
                                input {
                                    r#type: "number",
                                    min: "0",
                                    name: "clay",
                                    value: "0",
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "Iron"
                                input {
                                    r#type: "number",
                                    min: "0",
                                    name: "iron",
                                    value: "0",
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                            label { class: "text-sm text-gray-600",
                                "Crop"
                                input {
                                    r#type: "number",
                                    min: "0",
                                    name: "crop",
                                    value: "0",
                                    class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                }
                            }
                        }

                        button {
                            r#type: "submit",
                            class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                            "Send resources"
                        }
                    }
                }

                // Create offer form
                div { class: "border rounded-md p-4 bg-white space-y-4",
                    div {
                        div { class: "text-sm text-gray-500 uppercase", "Create offer" }
                        p { class: "text-sm text-gray-500", "Define what you offer and what you seek." }
                    }
                    form {
                        action: "/marketplace/offer/create",
                        method: "post",
                        class: "space-y-4",
                        input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                        input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                        div { class: "space-y-2",
                            div { class: "text-sm font-semibold text-gray-700", "Offering" }
                            div { class: "grid gap-3 sm:grid-cols-4",
                                label { class: "text-sm text-gray-600",
                                    "Lumber"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "offer_lumber",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Clay"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "offer_clay",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Iron"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "offer_iron",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Crop"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "offer_crop",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                            }
                        }

                        div { class: "space-y-2",
                            div { class: "text-sm font-semibold text-gray-700", "Seeking" }
                            div { class: "grid gap-3 sm:grid-cols-4",
                                label { class: "text-sm text-gray-600",
                                    "Lumber"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "seek_lumber",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Clay"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "seek_clay",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Iron"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "seek_iron",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                                label { class: "text-sm text-gray-600",
                                    "Crop"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        name: "seek_crop",
                                        value: "0",
                                        class: "mt-1 w-full border rounded px-3 py-2 text-gray-700"
                                    }
                                }
                            }
                        }

                        button {
                            r#type: "submit",
                            class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                            "Create offer"
                        }
                    }
                }

                // Own offers table
                div { class: "border rounded-md p-4 bg-white space-y-3",
                    div { class: "text-sm text-gray-500 uppercase", "Your offers" }
                    if own_offers.is_empty() {
                        p { class: "text-sm text-gray-500", "No active offers." }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "min-w-full text-sm",
                                thead { class: "text-left text-xs uppercase text-gray-500 border-b",
                                    tr {
                                        th { class: "py-2 pr-4", "Offering" }
                                        th { class: "py-2 pr-4", "Seeking" }
                                        th { class: "py-2 pr-4", "Created" }
                                        th { class: "py-2", "Actions" }
                                    }
                                }
                                tbody {
                                    for offer in own_offers.iter() {
                                        tr { class: "border-b last:border-b-0",
                                            td { class: "py-2 pr-4",
                                                "{format_resource_summary(&offer.offer_resources)}"
                                            }
                                            td { class: "py-2 pr-4",
                                                "{format_resource_summary(&offer.seek_resources)}"
                                            }
                                            td { class: "py-2 pr-4 text-gray-600",
                                                "{offer.created_at_text}"
                                            }
                                            td { class: "py-2",
                                                form {
                                                    action: "/marketplace/offer/cancel/{offer.offer_id}",
                                                    method: "post",
                                                    input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                                                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                                    button {
                                                        r#type: "submit",
                                                        class: "bg-red-600 hover:bg-red-700 text-white text-xs font-semibold px-3 py-1.5 rounded",
                                                        "Cancel"
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

                // Global offers table
                div { class: "border rounded-md p-4 bg-white space-y-3",
                    div { class: "text-sm text-gray-500 uppercase", "Global marketplace" }
                    if global_offers.is_empty() {
                        p { class: "text-sm text-gray-500", "No offers available." }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "min-w-full text-sm",
                                thead { class: "text-left text-xs uppercase text-gray-500 border-b",
                                    tr {
                                        th { class: "py-2 pr-4", "Village" }
                                        th { class: "py-2 pr-4", "Offering" }
                                        th { class: "py-2 pr-4", "Seeking" }
                                        th { class: "py-2 pr-4", "Merchants" }
                                        th { class: "py-2", "Actions" }
                                    }
                                }
                                tbody {
                                    for offer in global_offers.iter() {
                                        tr { class: "border-b last:border-b-0",
                                            td { class: "py-2 pr-4",
                                                "{offer.village_name} ({offer.position.x}|{offer.position.y})"
                                            }
                                            td { class: "py-2 pr-4",
                                                "{format_resource_summary(&offer.offer_resources)}"
                                            }
                                            td { class: "py-2 pr-4",
                                                "{format_resource_summary(&offer.seek_resources)}"
                                            }
                                            td { class: "py-2 pr-4",
                                                "{offer.merchants_required}"
                                            }
                                            td { class: "py-2",
                                                form {
                                                    action: "/marketplace/offer/accept/{offer.offer_id}",
                                                    method: "post",
                                                    input { r#type: "hidden", name: "slot_id", value: "{slot_id}" }
                                                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                                    button {
                                                        r#type: "submit",
                                                        class: "bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-semibold px-3 py-1.5 rounded",
                                                        "Accept"
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

                // Merchant movements
                div { class: "border rounded-md p-4 bg-white space-y-3",
                    div { class: "text-sm text-gray-500 uppercase", "Merchant movements" }
                    if merchant_movements.is_empty() {
                        p { class: "text-sm text-gray-500", "No merchant movements to display." }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "min-w-full text-sm",
                                thead { class: "text-left text-xs uppercase text-gray-500 border-b",
                                    tr {
                                        th { class: "py-2 pr-4", "Direction" }
                                        th { class: "py-2 pr-4", "Route" }
                                        th { class: "py-2 pr-4", "Resources" }
                                        th { class: "py-2 pr-4", "Merchants" }
                                        th { class: "py-2", "Arrives" }
                                    }
                                }
                                tbody {
                                    for movement in merchant_movements.iter() {
                                        {
                                            let origin = movement
                                                .origin_position
                                                .as_ref()
                                                .map(|pos| format!("{} ({}|{})", movement.origin_name, pos.x, pos.y))
                                                .unwrap_or_else(|| movement.origin_name.clone());
                                            let destination = movement
                                                .destination_position
                                                .as_ref()
                                                .map(|pos| format!("{} ({}|{})", movement.destination_name, pos.x, pos.y))
                                                .unwrap_or_else(|| movement.destination_name.clone());
                                            let route = format!("{origin} → {destination}");
                                            let arrival = format_duration(movement.time_remaining_secs);
                                            let kind_label = match movement.kind {
                                                MerchantMovementKind::Going => "Going",
                                                MerchantMovementKind::Return => "Returning",
                                            };
                                            let direction_label = match movement.direction {
                                                MerchantMovementDirection::Outgoing => "Outgoing",
                                                MerchantMovementDirection::Incoming => "Incoming",
                                            };
                                            rsx! {
                                                tr { class: "border-b last:border-b-0",
                                                    td { class: "py-2 pr-4 text-gray-700",
                                                        "{direction_label} ({kind_label})"
                                                    }
                                                    td { class: "py-2 pr-4", "{route}" }
                                                    td { class: "py-2 pr-4",
                                                        "{format_resource_summary(&movement.resources)}"
                                                    }
                                                    td { class: "py-2 pr-4", "{movement.merchants_used}" }
                                                    td {
                                                        class: "py-2 font-mono text-gray-600 countdown-timer",
                                                        "data-seconds": "{movement.time_remaining_secs}",
                                                        "{arrival}"
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
    }
}
