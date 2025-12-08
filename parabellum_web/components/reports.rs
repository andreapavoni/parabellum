use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use parabellum_types::reports::BattleReportPayload;
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

#[component]
pub fn ReportsPage(reports: Vec<ReportListEntry>) -> Element {
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
            if reports.is_empty() {
                div {
                    class: "bg-white border rounded-md p-6 text-center text-gray-500",
                    "{t!(\"game.reports.empty\")}"
                }
            } else {
                div {
                    class: "space-y-3",
                    for report in reports {
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

#[component]
pub fn BattleReportPage(
    report_id: Uuid,
    created_at: DateTime<Utc>,
    payload: BattleReportPayload,
) -> Element {
    // Format helpers - presentation logic in component
    let created_at_formatted = created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let report_reference_label = t!("game.reports.detail_id", id = report_id.to_string());
    let result_label = if payload.success {
        t!("game.reports.battle_success")
    } else {
        t!("game.reports.battle_failure")
    };

    let bounty_summary = format!(
        "🌲 {} | 🧱 {} | ⛏️ {} | 🌾 {}",
        payload.bounty.lumber(),
        payload.bounty.clay(),
        payload.bounty.iron(),
        payload.bounty.crop()
    );

    let result_border_class = if payload.success {
        "p-4 rounded-md border border-green-200 bg-green-50"
    } else {
        "p-4 rounded-md border border-red-200 bg-red-50"
    };

    let result_text_class = if payload.success {
        "text-xl font-bold text-green-700"
    } else {
        "text-xl font-bold text-red-700"
    };

    rsx! {
        div {
            class: "max-w-4xl mx-auto space-y-4",
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
                            "{payload.attacker_village} → {payload.defender_village}"
                        }
                    }
                    div {
                        class: "text-sm text-gray-500 text-right",
                        div { "{created_at_formatted}" }
                        div { "{report_reference_label}" }
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
                        "{result_label}"
                    }
                }

                // Attacker Army
                if let Some(ref attacker) = payload.attacker {
                    div {
                        class: "border rounded-md p-4",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                            "⚔️ {t!(\"game.reports.battle_detail.attacker\")} - {payload.attacker_player}"
                        }
                        p {
                            class: "text-sm text-gray-600 mb-3",
                            "{payload.attacker_village}"
                        }
                        ArmyDisplay {
                            army_before: attacker.army_before,
                            survivors: attacker.survivors,
                            losses: attacker.losses
                        }
                    }
                }

                // Defender Army
                if let Some(ref defender) = payload.defender {
                    div {
                        class: "border rounded-md p-4",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                            "🛡️ {t!(\"game.reports.battle_detail.defender\")} - {payload.defender_player}"
                        }
                        p {
                            class: "text-sm text-gray-600 mb-3",
                            "{payload.defender_village}"
                        }
                        ArmyDisplay {
                            army_before: defender.army_before,
                            survivors: defender.survivors,
                            losses: defender.losses
                        }
                    }
                }

                // Reinforcements
                if !payload.reinforcements.is_empty() {
                    div {
                        class: "border rounded-md p-4",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-3",
                            "🤝 Reinforcements"
                        }
                        for (idx , reinf) in payload.reinforcements.iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "mb-4 last:mb-0 pb-4 last:pb-0 border-b last:border-b-0",
                                p {
                                    class: "text-sm text-gray-600 mb-2",
                                    "Reinforcement #{idx + 1}"
                                }
                                ArmyDisplay {
                                    army_before: reinf.army_before,
                                    survivors: reinf.survivors,
                                    losses: reinf.losses
                                }
                            }
                        }
                    }
                }

                // Bounty
                div {
                    class: "border rounded-md p-4",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold mb-1",
                        "{t!(\"game.reports.battle_detail.bounty\")}"
                    }
                    p {
                        class: "font-mono text-gray-800",
                        "{bounty_summary}"
                    }
                }
            }
        }
    }
}

#[component]
fn ArmyDisplay(army_before: [u32; 10], survivors: [u32; 10], losses: [u32; 10]) -> Element {
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
