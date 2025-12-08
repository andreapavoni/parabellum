use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportsPageData {
    pub reports: Vec<ReportListEntry>,
}

#[component]
pub fn ReportsPage(data: ReportsPageData) -> Element {
    rsx! {
        div {
            class: "max-w-4xl mx-auto space-y-6",
            div {
                class: "flex items-center justify-between",
                h1 {
                    class: "text-2xl font-semibold text-gray-800",
                    "{t!(\"game.reports.title\")}"
                }
            }
            if data.reports.is_empty() {
                div {
                    class: "bg-white border rounded-md p-6 text-center text-gray-500",
                    "{t!(\"game.reports.empty\")}"
                }
            } else {
                div {
                    class: "space-y-3",
                    for report in data.reports {
                        a {
                            href: "{report.permalink}",
                            class: if !report.is_read {
                                "block border rounded-md p-4 bg-white space-y-2 hover:border-green-400 transition border-amber-400"
                            } else {
                                "block border rounded-md p-4 bg-white space-y-2 hover:border-green-400 transition"
                            },
                            div {
                                class: "flex items-center justify-between text-sm text-gray-500",
                                span { "{report.created_at_formatted}" }
                                if !report.is_read {
                                    span {
                                        class: "text-amber-600 font-semibold",
                                        "{t!(\"game.reports.unread\")}"
                                    }
                                }
                            }
                            div {
                                class: "text-lg font-semibold text-gray-900",
                                "{report.title}"
                            }
                            p {
                                class: "text-gray-700 text-sm",
                                "{report.summary}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleReportData {
    pub report_reference: String,
    pub report_reference_label: String,
    pub created_at_formatted: String,
    pub attacker_player: String,
    pub attacker_village: String,
    pub defender_player: String,
    pub defender_village: String,
    pub result_label: String,
    pub success: bool,
    pub bounty_summary: String,
}

#[component]
pub fn BattleReportPage(data: BattleReportData) -> Element {
    let result_border_class = if data.success {
        "p-4 rounded-md border border-green-200 bg-green-50"
    } else {
        "p-4 rounded-md border border-red-200 bg-red-50"
    };

    let result_text_class = if data.success {
        "text-xl font-bold text-green-700"
    } else {
        "text-xl font-bold text-red-700"
    };

    rsx! {
        div {
            class: "max-w-3xl mx-auto space-y-4",
            a {
                href: "/reports",
                class: "inline-flex items-center text-sm text-green-700 hover:underline",
                "← {t!(\"game.reports.back\")}"
            }
            div {
                class: "bg-white border rounded-md shadow-sm p-6 space-y-6",
                div {
                    class: "flex items-center justify-between",
                    div {
                        p {
                            class: "text-xs uppercase tracking-wide text-gray-500",
                            "{t!(\"game.reports.battle_detail.title\")}"
                        }
                        h1 {
                            class: "text-2xl font-semibold text-gray-900",
                            "{data.attacker_village} → {data.defender_village}"
                        }
                    }
                    div {
                        class: "text-sm text-gray-500 text-right",
                        div { "{data.created_at_formatted}" }
                        div { "{data.report_reference_label}" }
                    }
                }

                div {
                    class: "{result_border_class}",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold",
                        "{t!(\"game.reports.battle_detail.result\")}"
                    }
                    p {
                        class: "{result_text_class}",
                        "{data.result_label}"
                    }
                }

                div {
                    class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                    div {
                        class: "border rounded-md p-4 bg-gray-50",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold",
                            "{t!(\"game.reports.battle_detail.attacker\")}"
                        }
                        p {
                            class: "text-lg font-semibold text-gray-900",
                            "{data.attacker_player}"
                        }
                        p {
                            class: "text-sm text-gray-600",
                            "{data.attacker_village}"
                        }
                    }
                    div {
                        class: "border rounded-md p-4 bg-gray-50",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold",
                            "{t!(\"game.reports.battle_detail.defender\")}"
                        }
                        p {
                            class: "text-lg font-semibold text-gray-900",
                            "{data.defender_player}"
                        }
                        p {
                            class: "text-sm text-gray-600",
                            "{data.defender_village}"
                        }
                    }
                }

                div {
                    class: "border rounded-md p-4",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold mb-1",
                        "{t!(\"game.reports.battle_detail.bounty\")}"
                    }
                    p {
                        class: "font-mono text-gray-800",
                        "{data.bounty_summary}"
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

#[component]
pub fn GenericReportPage(data: GenericReportData) -> Element {
    rsx! {
        div {
            class: "max-w-2xl mx-auto space-y-4",
            a {
                href: "/reports",
                class: "inline-flex items-center text-sm text-green-700 hover:underline",
                "← {t!(\"game.reports.back\")}"
            }
            div {
                class: "bg-white border rounded-md shadow-sm p-6 space-y-3",
                div {
                    class: "flex items-center justify-between",
                    h1 {
                        class: "text-2xl font-semibold text-gray-900",
                        "{data.heading}"
                    }
                    div {
                        class: "text-sm text-gray-500 text-right",
                        div { "{data.created_at_formatted}" }
                        div { "{data.report_reference_label}" }
                    }
                }
                p {
                    class: "text-gray-700",
                    "{data.message}"
                }
            }
        }
    }
}
