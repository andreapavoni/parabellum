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
    /// Get hex fill color based on resource type
    fn hex_color(&self) -> &'static str {
        match self.building_name {
            BuildingName::Woodcutter => "#6c9a35",
            BuildingName::ClayPit => "#d98536",
            BuildingName::IronMine => "#999999",
            BuildingName::Cropland => "#f2d649",
            _ => "#6c9a35",
        }
    }
}

#[component]
pub fn ResourceFieldsMap(slots: Vec<ResourceSlot>) -> Element {
    // SVG hex positions: (slot_id, translate_x, translate_y)
    let hex_positions = [
        (1, 279, 190),  // Top Left
        (2, 400, 190),  // Top Center
        (3, 521, 190),  // Top Right
        (4, 218, 295),  // Middle-Top Left
        (5, 339, 295),  // Middle-Top Center-Left
        (6, 460, 295),  // Middle-Top Center-Right
        (7, 581, 295),  // Middle-Top Right
        (8, 157, 400),  // Center Far Left
        (9, 278, 400),  // Center Left
        (10, 521, 400), // Center Right
        (11, 642, 400), // Center Far Right
        (12, 218, 505), // Middle-Bottom Left
        (13, 339, 505), // Middle-Bottom Center-Left
        (14, 460, 505), // Middle-Bottom Center-Right
        (15, 581, 505), // Middle-Bottom Right
        (16, 279, 610), // Bottom Left
        (17, 400, 610), // Bottom Center
        (18, 521, 610), // Bottom Right
    ];

    rsx! {
        div { class: "resource-fields-svg-container",
            svg {
                view_box: "0 200 800 600",
                xmlns: "http://www.w3.org/2000/svg",

                // Define the hexagon shape
                defs {
                    polygon {
                        id: "hex-shape",
                        points: "0,-70 60.62,-35 60.62,35 0,70 -60.62,35 -60.62,-35",
                        stroke: "white",
                        stroke_width: "3",
                    }
                }

                // Render each resource field hex
                for (slot_id , tx , ty) in hex_positions {
                    {
                        let slot = slots.iter().find(|s| s.slot_id == slot_id);
                        if let Some(slot) = slot {
                            let fill_color = slot.hex_color();
                            let has_construction = slot.in_queue.is_some();
                            let is_processing = slot.in_queue.unwrap_or(false);

                            rsx! {
                                a {
                                    href: "/build/{slot_id}",
                                    g {
                                        class: if has_construction {
                                            if is_processing {
                                                "resource-hex-group construction-active"
                                            } else {
                                                "resource-hex-group construction-pending"
                                            }
                                        } else {
                                            "resource-hex-group"
                                        },
                                        transform: "translate({tx}, {ty})",
                                        use {
                                            href: "#hex-shape",
                                            fill: "{fill_color}",
                                            stroke: "none",
                                        }
                                        text {
                                            x: "0",
                                            y: "5",
                                            text_anchor: "middle",
                                            "{slot.level}"
                                        }
                                        title { "{slot.building_name} (Level {slot.level})" }
                                    }
                                }
                            }
                        } else {
                            rsx! { g {} }
                        }
                    }
                }

                // Village center (city center circle)
                a {
                    href: "/village",
                    g {
                        class: "resource-city-center",
                        transform: "translate(400, 400)",
                        circle {
                            cx: "0",
                            cy: "0",
                            r: "68",
                            fill: "white",
                        }
                        circle {
                            class: "resource-main-circle",
                            cx: "0",
                            cy: "0",
                            r: "62",
                            fill: "#5c192d",
                        }
                        title { "Village Center" }
                    }
                }
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
