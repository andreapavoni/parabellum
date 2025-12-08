use dioxus::prelude::*;
use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};

use super::common::BuildingQueue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSlot {
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub level: u8,
    pub in_queue: Option<bool>, // None = not in queue, Some(true) = processing, Some(false) = pending
}

impl ResourceSlot {
    /// Get the CSS class for this resource type
    pub fn css_class(&self) -> &'static str {
        match &self.building_name {
            BuildingName::Woodcutter => "wood",
            BuildingName::ClayPit => "clay",
            BuildingName::IronMine => "iron",
            BuildingName::Cropland => "crop",
            _ => "wood",
        }
    }

    /// Get the full CSS classes for rendering including queue state
    pub fn render_classes(&self) -> String {
        let mut classes = format!("hex {} occupied", self.css_class());
        if let Some(is_processing) = self.in_queue {
            if is_processing {
                classes.push_str(" construction-active");
            } else {
                classes.push_str(" construction-pending");
            }
        }
        classes
    }
}

use parabellum_game::models::village::Village;

#[component]
pub fn ResourcesPage(
    village: Village,
    resource_slots: Vec<ResourceSlot>,
    building_queue: Vec<super::common::BuildingQueueItem>,
) -> Element {
    let production = &village.production.effective;

    rsx! {
        div { class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            div { class: "flex flex-col items-center w-full md:w-auto",
                h1 { class: "text-xl font-bold mb-4 w-full text-left md:text-left",
                    "{village.name} ({village.position.x}|{village.position.y})"
                }

                // Resource fields map
                ResourceFieldsMap { slots: resource_slots.clone() }

                // Building queue
                BuildingQueue { queue: building_queue }
            }

            div { class: "w-full max-w-[360px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none",
                div { class: "flex flex-row md:flex-col justify-between md:justify-start gap-8 md:gap-0",

                    // Production info
                    ProductionPanel {
                        lumber: production.lumber,
                        clay: production.clay,
                        iron: production.iron,
                        crop: production.crop as u32
                    }

                    // Troops
                    TroopsPanel { village: village.clone() }
                }
            }
        }
    }
}

#[component]
fn ResourceFieldsMap(slots: Vec<ResourceSlot>) -> Element {
    // Positions for each slot (matching the original template)
    let positions = [
        (0, 60),
        (0, 120),
        (0, 180),
        (52, 30),
        (52, 90),
        (52, 150),
        (52, 210),
        (104, 0),
        (104, 60),
        (104, 180),
        (104, 240),
        (156, 30),
        (156, 90),
        (156, 150),
        (156, 210),
        (208, 60),
        (208, 120),
        (208, 180),
    ];

    rsx! {
        div { class: "map-container-responsive",
            for (idx , slot) in slots.iter().enumerate() {
                {
                    let (top, left) = positions[idx];
                    let slot_number = idx + 1;
                    let classes = slot.render_classes();

                    rsx! {
                        a {
                            class: "{classes}",
                            href: "/dioxus/build/{slot_number}",
                            style: "top: {top}px; left: {left}px;",
                            title: "{slot.building_name} (Level {slot.level})",
                            span { class: "level", "{slot.level}" }
                        }
                    }
                }
            }

            // Village center link
            a {
                class: "village",
                href: "/village",
                title: "Village Center"
            }
        }
    }
}

// BuildingQueue component moved to common.rs

#[component]
fn ProductionPanel(lumber: u32, clay: u32, iron: u32, crop: u32) -> Element {
    rsx! {
        div { class: "flex-1",
            h3 { class: "font-bold mb-3 text-sm", "Production:" }
            div { class: "text-xs space-y-3",
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🌲 Lumber" }
                    span { class: "font-bold text-gray-900", "{lumber}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🧱 Clay" }
                    span { class: "font-bold text-gray-900", "{clay}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "⛏️ Iron" }
                    span { class: "font-bold text-gray-900", "{iron}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🌾 Crop" }
                    span { class: "font-bold text-gray-900", "{crop}/hour" }
                }
            }
        }
    }
}

#[component]
fn TroopsPanel(village: Village) -> Element {
    use crate::view_helpers::unit_display_name;

    let troops: Vec<(String, u32)> = village
        .army()
        .map(|army| {
            let tribe_units = village.tribe.units();
            army.units()
                .iter()
                .enumerate()
                .filter_map(|(idx, quantity)| {
                    if *quantity == 0 {
                        return None;
                    }
                    let name = unit_display_name(&tribe_units[idx].name);
                    Some((name, *quantity))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "flex-1 md:mt-8",
            h3 { class: "font-bold mb-2 text-sm md:mt-6", "Troops:" }
            if troops.is_empty() {
                div { class: "text-xs text-gray-500 italic",
                    "No units"
                }
            } else {
                div { class: "text-xs space-y-2",
                    for (name , count) in troops {
                        div { class: "flex justify-between border-b border-gray-100 pb-1",
                            span { "{name}" }
                            span { class: "font-semibold text-gray-900", "{count}" }
                        }
                    }
                }
            }
        }
    }
}
