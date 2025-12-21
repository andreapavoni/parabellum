use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use serde::{Deserialize, Serialize};

use crate::components::BuildingSlot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageListItem {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_current: bool,
}

#[component]
pub fn VillageMap(slots: Vec<BuildingSlot>) -> Element {
    // Find specific slots
    let wall_slot = slots.iter().find(|s| s.slot_id == 40);
    let main_building = slots.iter().find(|s| s.slot_id == 19);
    let rally_point = slots.iter().find(|s| s.slot_id == 39);

    // Building positions mapped to SVG coordinates (slot_id, cx, cy)
    // These correspond to the node positions in the SVG
    let building_positions = [
        (26, 220, 260), // ID 1 in SVG
        (27, 340, 160), // ID 20 (Alto)
        (28, 490, 120), // ID 7 (Alto Dx)
        (29, 680, 180), // ID 2 (Dx)
        (30, 800, 300), // ID 11 (Dx estrema)
        (32, 860, 440), // ID 19 (Basso Dx)
        (33, 860, 600), // ID 7 (Basso Dx interno)
        (34, 780, 760), // ID 7 (Basso centrale)
        (35, 600, 860), // ID 0 (empty/construction site)
        (36, 350, 840), // ID 20 (Basso Sx)
        (37, 210, 740), // ID 5 (Sx)
        (38, 130, 590), // ID 5 (Sx Alto)
        (31, 140, 420), // ID 5 (Sx Alto estremo)
        (22, 380, 330), // 5 Interno
        (20, 300, 480), // 5 Interno basso
        (21, 350, 660), // 1 Interno
        (25, 530, 260), // 20 Interno alto
        (24, 640, 380), // 8 Interno dx
        (23, 520, 710), // 20 Interno basso dx
    ];

    rsx! {
        div { class: "village-svg-container",
            svg {
                view_box: "0 0 1000 1000",
                xmlns: "http://www.w3.org/2000/svg",

                // 1. External ring (Wall - slot 40)
                if let Some(wall) = wall_slot {
                    a {
                        href: "/build/40",
                        circle {
                            class: "village-wall-ring",
                            cx: "500",
                            cy: "500",
                            r: "460",
                            fill: "none",
                            stroke: "#E88C30",
                            stroke_width: "18",
                            opacity: "0.9",
                        }
                        title { "{wall.title()}" }
                    }
                }

                // 2. Rally point (Radar - slot 39)
                if let Some(rally) = rally_point {
                    a {
                        href: "/build/39",
                        path {
                            class: "village-radar-zone",
                            d: "M 535 778 A 280 280 0 0 0 765 605 L 588 541 A 120 120 0 0 1 512 618 Z",
                            fill: "rgba(74, 122, 41, 0.25)",
                            stroke: "#4a7a29",
                            stroke_width: "3",
                            stroke_dasharray: "10, 8",
                            transform: "rotate(-30, 500, 500)",
                        }
                        title { "{rally.title()}" }
                    }
                }

                // 3. Regular building slots (20-38)
                for (slot_id, cx, cy) in building_positions {
                    {
                        let slot = slots.iter().find(|s| s.slot_id == slot_id);
                        if let Some(slot) = slot {
                            let is_empty = slot.building_name.is_none();
                            let level_text = if is_empty {
                                "-".to_string()
                            } else {
                                slot.level.to_string()
                            };

                            rsx! {
                                a {
                                    href: "/build/{slot_id}",
                                    g {
                                        class: "village-node-group",
                                        circle {
                                            class: if is_empty { slot.render_classes("village-node-bg village-node-empty", false) } else { slot.render_classes("village-node-bg village-node-occupied", false) },
                                            cx: "{cx}",
                                            cy: "{cy}",
                                            r: "55",
                                            stroke_width: "2",
                                            stroke_dasharray: "6,4",
                                            opacity: if is_empty { "0.6" } else { "1.0" },
                                        }
                                        text {
                                            x: "{cx}",
                                            y: "{cy}",
                                            dy: "0.35em",
                                            text_anchor: "middle",
                                            font_weight: "bold",
                                            font_size: "28",
                                            fill: if is_empty { "#3e2b18" } else { "#1a3a10" },
                                            "{level_text}"
                                        }
                                        title { "{slot.title()}" }
                                    }
                                }
                            }
                        } else {
                            rsx! { g {} }
                        }
                    }
                }

                // 4. Main Building (slot 19) - center
                if let Some(main) = main_building {
                    a {
                        href: "/build/19",
                        g {
                            id: "village-main-node",
                            circle {
                                cx: "500",
                                cy: "520",
                                r: "90",
                                fill: "none",
                                stroke: "white",
                                stroke_width: "5",
                                opacity: "0.8",
                            }
                            circle {
                                cx: "500",
                                cy: "520",
                                r: "85",
                                fill: "#EDF4E1",
                            }
                            text {
                                x: "500",
                                y: "520",
                                dy: "0.35em",
                                text_anchor: "middle",
                                font_family: "Arial, sans-serif",
                                font_weight: "900",
                                font_size: "32",
                                fill: "#1a3a10",
                                "Main"
                            }
                            title { "{main.title()}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn VillagesList(villages: Vec<VillageListItem>, csrf_token: String) -> Element {
    rsx! {
        div { class: "w-full max-w-[400px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none",
            h3 { class: "font-bold mb-3 text-sm border-b border-gray-300 pb-2",
                "Villages:"
            }
            ul { class: "text-xs space-y-2 list-none pl-0",
                for village in villages {
                    li {
                        class: if village.is_current {
                            "flex justify-between items-center p-1 rounded font-bold bg-gray-100 cursor-default"
                        } else {
                            "p-1 rounded hover:bg-gray-100"
                        },
                        if village.is_current {
                            span { class: "flex items-center",
                                span { class: "w-2 h-2 rounded-full mr-2 bg-orange-500" }
                                "{village.name}"
                            }
                            span { class: "text-gray-600",
                                "({village.x}|{village.y})"
                            }
                        } else {
                            form {
                                method: "post",
                                action: "/village/switch/{village.id}",
                                class: "flex justify-between items-center w-full",
                                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                button {
                                    r#type: "submit",
                                    class: "flex justify-between items-center w-full text-left bg-transparent border-0 p-0",
                                    span { class: "flex items-center",
                                        span { class: "w-2 h-2 rounded-full mr-2 bg-green-500" }
                                        "{village.name}"
                                    }
                                    span { class: "text-gray-500",
                                        "({village.x}|{village.y})"
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

#[component]
pub fn TroopsPanel(village: Village) -> Element {
    use crate::view_helpers::unit_display_name;

    let troops: Vec<(String, u32)> = village
        .army()
        .map(|army| {
            let tribe_units = village.tribe.units();
            army.units()
                .units()
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
