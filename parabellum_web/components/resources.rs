use dioxus::prelude::*;

use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};

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

#[component]
pub fn ResourceFieldsMap(slots: Vec<ResourceSlot>) -> Element {
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
                            href: "/build/{slot_number}",
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

#[component]
pub fn ProductionPanel(lumber: u32, clay: u32, iron: u32, crop: u32) -> Element {
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
