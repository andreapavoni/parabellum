use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use parabellum_types::tribe::Tribe;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[component]
pub fn BattleArmyTable(tribe: Tribe, army_before: [u32; 10], losses: [u32; 10]) -> Element {
    let tribe_units = tribe.units();

    rsx! {
        div { class: "overflow-x-auto",
            table { class: "w-full border-collapse",
                thead {
                    tr {
                        for (idx, unit) in tribe_units.iter().enumerate() {
                            th {
                                key: "{idx}",
                                class: "text-center p-1 text-xs text-gray-500 border-b",
                                title: "{unit.name:?}",
                                "{unit.name:?}"
                            }
                        }
                    }
                }
                tbody {
                    // Initial army row
                    tr {
                        for (idx, &count) in army_before.iter().enumerate() {
                            {
                                let is_zero = count == 0;
                                rsx! {
                                    td {
                                        key: "{idx}",
                                        class: "text-center p-2 border-r last:border-r-0",
                                        class: if is_zero { "bg-gray-50 opacity-40" } else { "bg-gray-100" },
                                        div {
                                            class: if is_zero { "text-gray-400 text-sm" } else { "text-gray-900 font-semibold" },
                                            "{count}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Losses row (always shown for battles)
                    tr {
                        for (idx, &loss) in losses.iter().enumerate() {
                            {
                                let has_loss = loss > 0;
                                rsx! {
                                    td {
                                        key: "{idx}",
                                        class: "text-center p-2 border-r last:border-r-0 bg-red-50",
                                        div {
                                            class: if has_loss { "text-red-600 font-semibold text-sm" } else { "text-gray-300 text-xs" },
                                            if has_loss {
                                                "↓{loss}"
                                            } else {
                                                "-"
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
    }
}

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
