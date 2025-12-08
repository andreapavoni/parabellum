use dioxus::prelude::*;
use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};

use super::common::BuildingQueue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingSlot {
    pub slot_id: u8,
    pub building_name: Option<BuildingName>,
    pub level: u8,
    pub in_queue: Option<bool>, // None = not in queue, Some(true) = processing, Some(false) = pending
}

impl BuildingSlot {
    /// Get CSS classes for rendering including queue state
    pub fn render_classes(&self, base_class: &str, include_occupied: bool) -> String {
        let mut classes = base_class.to_string();

        if include_occupied && self.building_name.is_some() {
            classes.push_str(" occupied");
        }

        if let Some(is_processing) = self.in_queue {
            if is_processing {
                classes.push_str(" construction-active");
            } else {
                classes.push_str(" construction-pending");
            }
        }

        classes
    }

    /// Get title/tooltip for the slot
    pub fn title(&self) -> String {
        if let Some(ref building) = self.building_name {
            if self.level > 0 {
                format!("{} (Level {})", building, self.level)
            } else {
                "Empty slot".to_string()
            }
        } else {
            "Empty slot".to_string()
        }
    }
}

use parabellum_game::models::village::Village;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageListItem {
    pub id: i64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_current: bool,
}

#[component]
pub fn VillagePage(
    village: Village,
    building_slots: Vec<BuildingSlot>,
    building_queue: Vec<super::common::BuildingQueueItem>,
    villages: Vec<VillageListItem>,
) -> Element {
    rsx! {
        div { class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            div { class: "flex flex-col items-center w-full md:w-auto",
                h1 { class: "text-xl font-bold mb-4 w-full text-left md:text-left",
                    "{village.name} ({village.position.x}|{village.position.y})"
                }

                VillageMap { slots: building_slots.clone() }

                BuildingQueue { queue: building_queue }
            }

            VillagesList { villages: villages }
        }
    }
}

#[component]
fn VillageMap(slots: Vec<BuildingSlot>) -> Element {
    // Find specific slots
    let wall_slot = slots.iter().find(|s| s.slot_id == 40);
    let main_building = slots.iter().find(|s| s.slot_id == 19);
    let rally_point = slots.iter().find(|s| s.slot_id == 39);

    // Regular building slots (20-38)
    let building_positions = [
        (20, "28%", "47%", "1"),
        (21, "35%", "63%", "2"),
        (22, "37%", "32%", "3"),
        (23, "72%", "53%", "4"),
        (24, "63%", "38%", "5"),
        (25, "52%", "27%", "6"),
        (26, "22%", "24%", "7"),
        (27, "36%", "15%", "8"),
        (28, "52%", "12%", "9"),
        (29, "67%", "18%", "10"),
        (30, "77%", "28%", "11"),
        (31, "13%", "40%", "12"),
        (32, "86%", "43%", "13"),
        (33, "85%", "60%", "14"),
        (34, "76%", "75%", "15"),
        (35, "63%", "85%", "16"),
        (36, "45%", "88%", "17"),
        (37, "30%", "80%", "18"),
        (38, "15%", "62%", "19"),
    ];

    rsx! {
        div { class: "village-container-responsive relative",
            // Wall (slot 40)
            if let Some(wall) = wall_slot {
                a {
                    class: "{wall.render_classes(\"wall-ring-link\", false)}",
                    href: "/dioxus/build/40",
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
                    href: "/dioxus/build/19",
                    style: "top: 50%; left: 50%;",
                    title: "{main.title()}",
                    span { class: "slot-label", "Main Building" }
                }
            }

            // Rally Point (slot 39)
            if let Some(rally) = rally_point {
                a {
                    class: "{rally.render_classes(\"building-slot rally-point\", true)}",
                    href: "/dioxus/build/39",
                    style: "top: 55%; left: 67%;",
                    title: "{rally.title()}"
                }
            }

            // Regular building slots (20-38)
            for (slot_id, top, left, label) in building_positions {
                {
                    let slot = slots.iter().find(|s| s.slot_id == slot_id);
                    if let Some(slot) = slot {
                        rsx! {
                            a {
                                class: "{slot.render_classes(\"building-slot\", true)}",
                                href: "/dioxus/build/{slot_id}",
                                style: "top: {top}; left: {left};",
                                title: "{slot.title()}",
                                span { class: "slot-label", "{label}" }
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
fn VillagesList(villages: Vec<VillageListItem>) -> Element {
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
