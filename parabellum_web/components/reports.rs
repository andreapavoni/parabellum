use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use parabellum_types::tribe::Tribe;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[component]
pub fn ReinforcementArmyTable(tribe: Tribe, units: [u32; 10]) -> Element {
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
                    // Single row - just the troop counts (no losses)
                    tr {
                        for (idx, &count) in units.iter().enumerate() {
                            {
                                let is_zero = count == 0;
                                rsx! {
                                    td {
                                        key: "{idx}",
                                        class: "text-center p-2 border-r last:border-r-0",
                                        class: if is_zero { "bg-gray-50 opacity-40" } else { "bg-blue-100" },
                                        div {
                                            class: if is_zero { "text-gray-400 text-sm" } else { "text-gray-900 font-semibold" },
                                            "{count}"
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
