use dioxus::prelude::*;
use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSlot {
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub level: u8,
    pub css_class: String,
    pub queue_state: Option<QueueState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueueState {
    Active,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionInfo {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TroopInfo {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingQueueItem {
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesPageData {
    pub village: VillageInfo,
    pub resource_slots: Vec<ResourceSlot>,
    pub production: ProductionInfo,
    pub troops: Vec<TroopInfo>,
    pub building_queue: Vec<BuildingQueueItem>,
}

#[component]
pub fn ResourcesPage(data: ResourcesPageData) -> Element {
    rsx! {
        div { class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            div { class: "flex flex-col items-center w-full md:w-auto",
                h1 { class: "text-xl font-bold mb-4 w-full text-left md:text-left",
                    "{data.village.name} ({data.village.x}|{data.village.y})"
                }

                // Resource fields map
                ResourceFieldsMap { slots: data.resource_slots.clone() }

                // Building queue
                BuildingQueue { queue: data.building_queue }
            }

            div { class: "w-full max-w-[360px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none",
                div { class: "flex flex-row md:flex-col justify-between md:justify-start gap-8 md:gap-0",

                    // Production info
                    ProductionPanel { production: data.production }

                    // Troops
                    TroopsPanel { troops: data.troops }
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
                    let mut classes = format!("hex {} occupied", slot.css_class);

                    if let Some(ref queue_state) = slot.queue_state {
                        match queue_state {
                            QueueState::Active => classes.push_str(" construction-active"),
                            QueueState::Pending => classes.push_str(" construction-pending"),
                        }
                    }

                    rsx! {
                        a {
                            class: "{classes}",
                            href: "/build?s={slot_number}",
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
fn BuildingQueue(queue: Vec<BuildingQueueItem>) -> Element {
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
                            href: "/build?s={item.slot_id}",
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

#[component]
fn ProductionPanel(production: ProductionInfo) -> Element {
    rsx! {
        div { class: "flex-1",
            h3 { class: "font-bold mb-3 text-sm", "Production:" }
            div { class: "text-xs space-y-3",
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🌲 Lumber" }
                    span { class: "font-bold text-gray-900", "{production.lumber}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🧱 Clay" }
                    span { class: "font-bold text-gray-900", "{production.clay}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "⛏️ Iron" }
                    span { class: "font-bold text-gray-900", "{production.iron}/hour" }
                }
                div { class: "flex justify-between border-b border-gray-100 pb-2",
                    span { "🌾 Crop" }
                    span { class: "font-bold text-gray-900", "{production.crop}/hour" }
                }
            }
        }
    }
}

#[component]
fn TroopsPanel(troops: Vec<TroopInfo>) -> Element {
    rsx! {
        div { class: "flex-1 md:mt-8",
            h3 { class: "font-bold mb-2 text-sm md:mt-6", "Troops:" }
            if troops.is_empty() {
                div { class: "text-xs text-gray-500 italic",
                    "No units"
                }
            } else {
                div { class: "text-xs space-y-2",
                    for troop in troops {
                        div { class: "flex justify-between border-b border-gray-100 pb-1",
                            span { "{troop.name}" }
                            span { class: "font-semibold text-gray-900", "{troop.count}" }
                        }
                    }
                }
            }
        }
    }
}
