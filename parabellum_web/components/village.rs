use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use serde::{Deserialize, Serialize};

use crate::components::BuildingSlot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageListItem {
    pub id: i64,
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

    // Regular building slots (20-38)
    let building_positions = [
        (20, "28%", "47%"),
        (21, "35%", "63%"),
        (22, "37%", "32%"),
        (23, "72%", "53%"),
        (24, "63%", "38%"),
        (25, "52%", "27%"),
        (26, "22%", "24%"),
        (27, "36%", "15%"),
        (28, "52%", "12%"),
        (29, "67%", "18%"),
        (30, "77%", "28%"),
        (31, "13%", "40%"),
        (32, "86%", "43%"),
        (33, "85%", "60%"),
        (34, "76%", "75%"),
        (35, "63%", "85%"),
        (36, "45%", "88%"),
        (37, "30%", "80%"),
        (38, "15%", "62%"),
    ];

    rsx! {
        div { class: "village-container-responsive relative",
            // Wall (slot 40)
            if let Some(wall) = wall_slot {
                a {
                    class: "{wall.render_classes(\"wall-ring-link\", false)}",
                    href: "/build/40",
                    title: "{wall.title()}",
                    "aria-label": "{wall.title()}",
                    svg {
                        class: "{wall.render_classes(\"wall-ring\", true)}",
                        view_box: "0 0 100 100",
                        role: "img",
                        "aria-hidden": "true",
                        title { "{wall.title()}" }
                        circle { class: "wall-ring-track", cx: "50", cy: "50", r: "51" }
                    }
                }
            }

            // Main Building (slot 19)
            if let Some(main) = main_building {
                a {
                    class: "{main.render_classes(\"building-slot main-building\", true)}",
                    href: "/build/19",
                    style: "top: 50%; left: 50%;",
                    title: "{main.title()}",
                    span { class: "slot-label", "Main" }
                }
            }

            // Rally Point (slot 39)
            if let Some(rally) = rally_point {
                a {
                    class: "{rally.render_classes(\"building-slot rally-point\", true)}",
                    href: "/build/39",
                    style: "top: 55%; left: 67%;",
                    title: "{rally.title()}"
                }
            }

            // Regular building slots (20-38)
            for (slot_id, top, left) in building_positions {
                {
                    let slot = slots.iter().find(|s| s.slot_id == slot_id);
                    if let Some(slot) = slot {
                        rsx! {
                            a {
                                class: "{slot.render_classes(\"building-slot\", true)}",
                                href: "/build/{slot_id}",
                                style: "top: {top}; left: {left};",
                                title: "{slot.title()}",
                                span { class: "slot-label", "{slot.level}" }
                            }
                        }
                    } else {
                        rsx! { span {} }
                    }
                }
            }
        }
    }
}

#[component]
pub fn VillagesList(villages: Vec<VillageListItem>) -> Element {
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
                            "flex justify-between items-center p-1 rounded cursor-pointer hover:bg-gray-100"
                        },
                        span { class: "flex items-center",
                            span {
                                class: if village.is_current {
                                    "w-2 h-2 rounded-full mr-2 bg-orange-500"
                                } else {
                                    "w-2 h-2 rounded-full mr-2 bg-green-500"
                                },
                            }
                            "{village.name}"
                        }
                        span {
                            class: if village.is_current { "text-gray-600" } else { "text-gray-500" },
                            "({village.x}|{village.y})"
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
            army.units().units()
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
