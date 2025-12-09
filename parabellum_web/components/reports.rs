use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[component]
pub fn ArmyDisplay(army_before: [u32; 10], survivors: [u32; 10], losses: [u32; 10]) -> Element {
    // Filter out zero troops
    let troops: Vec<(usize, u32, u32, u32)> = (0..10)
        .filter_map(|idx| {
            if army_before[idx] > 0 {
                Some((idx, army_before[idx], survivors[idx], losses[idx]))
            } else {
                None
            }
        })
        .collect();

    if troops.is_empty() {
        return rsx! {
            p { class: "text-sm text-gray-500 italic", "No troops" }
        };
    }

    rsx! {
        div { class: "space-y-2",
            div { class: "grid grid-cols-4 gap-2 text-xs font-semibold text-gray-500 border-b pb-1",
                div { "Unit" }
                div { class: "text-right", "Before" }
                div { class: "text-right", "Survived" }
                div { class: "text-right", "Losses" }
            }
            for (idx , before , survived , lost) in troops {
                div {
                    key: "{idx}",
                    class: "grid grid-cols-4 gap-2 text-sm",
                    div { class: "text-gray-700", "Unit {idx + 1}" }
                    div { class: "text-right text-gray-900", "{before}" }
                    div { class: "text-right text-green-700 font-semibold", "{survived}" }
                    div { class: "text-right text-red-700 font-semibold", "{lost}" }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericReportData {
    pub report_reference: String,
    pub report_reference_label: String,
    pub created_at_formatted: String,
    pub heading: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportListEntry {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub created_at_formatted: String,
    pub is_read: bool,
    pub permalink: String,
}
