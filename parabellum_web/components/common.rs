use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// Shared building queue item data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingQueueItem {
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub is_processing: bool,
}

/// Village information (used in lists and headers)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageInfo {
    pub id: i64,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

/// Troop count display (used in RallyPoint)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TroopCount {
    pub name: String,
    pub count: u32,
}

/// Unit available for sending (used in RallyPoint)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RallyPointUnit {
    pub name: String,
    pub unit_idx: usize,
    pub available: u32,
}

/// Troop movement display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TroopMovement {
    pub direction: MovementDirection,
    pub kind: MovementKind,
    pub origin_name: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub destination_name: String,
    pub destination_x: i32,
    pub destination_y: i32,
    pub time_remaining: String,
    pub time_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MovementDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MovementKind {
    Attack,
    Raid,
    Reinforcement,
    Return,
}

/// Reusable building queue component
#[component]
pub fn BuildingQueue(queue: Vec<BuildingQueueItem>) -> Element {
    rsx! {
        div { class: "w-full mt-4 flex flex-col text-[11px] text-gray-600 px-4 max-w-[400px] gap-1",
            div { class: "font-bold text-gray-800 border-b border-gray-300 pb-1 mb-1",
                "Building Queue:"
            }
            if queue.is_empty() {
                div { class: "text-xs text-gray-500",
                    "No buildings in queue"
                }
            } else {
                for item in queue {
                    div { class: "flex justify-between w-full items-center",
                        a {
                            class: "flex items-center gap-2 text-gray-800 hover:text-gray-900 hover:underline",
                            href: "/build/{item.slot_id}",
                            span {
                                class: if item.is_processing { "text-green-600" } else { "text-yellow-600" },
                                "⏳"
                            }
                            "{item.building_name} (Lv {item.target_level})"
                        }
                        span {
                            class: "font-semibold text-gray-800 queue-timer",
                            "data-seconds": "{item.time_seconds}",
                            "{item.time_remaining}"
                        }
                    }
                }
            }
        }
    }
}
