use dioxus::prelude::*;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use serde::{Deserialize, Serialize};

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

/// Resource cost display component
#[component]
pub fn ResourceCost(cost: ResourceGroup) -> Element {
    rsx! {
        div { class: "flex gap-3 text-sm",
            span { class: "text-gray-700", "🌲 {cost.lumber()}" }
            span { class: "text-gray-700", "🧱 {cost.clay()}" }
            span { class: "text-gray-700", "⛏️ {cost.iron()}" }
            span { class: "text-gray-700", "🌾 {cost.crop()}" }
        }
    }
}

/// Missing requirements display component
#[component]
pub fn MissingRequirements(requirements: Vec<(BuildingName, u8)>) -> Element {
    if requirements.is_empty() {
        return rsx! { Fragment {} };
    }

    rsx! {
        div { class: "mt-2 text-sm text-red-600",
            p { class: "font-semibold", "❌ {t!(\"game.building.requirements\")}:" }
            ul { class: "list-disc list-inside",
                for (building_name , required_level) in requirements {
                    li { "{building_name:?} (Level {required_level})" }
                }
            }
        }
    }
}
