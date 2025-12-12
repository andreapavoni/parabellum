use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use parabellum_types::{
    battle::AttackType,
    reports::{BattleReportPayload, ReinforcementReportPayload},
};
use rust_i18n::t;
use uuid::Uuid;

use crate::components::{
    BattleArmyTable, GenericReportData, ReinforcementArmyTable, ReportListEntry,
};

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

#[component]
pub fn ReinforcementReportPage(
    report_id: Uuid,
    created_at: DateTime<Utc>,
    payload: ReinforcementReportPayload,
) -> Element {
    let created_at_formatted = created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let report_reference_label = t!("game.reports.detail_id", id = report_id.to_string());

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
                            "Reinforcement Report"
                        }
                        h1 {
                            class: "text-2xl font-semibold text-gray-900",
                            "{payload.sender_village} reinforced {payload.receiver_village}"
                        }
                    }
                    div {
                        class: "text-sm text-gray-500 text-right",
                        div { "{created_at_formatted}" }
                        div { "{report_reference_label}" }
                    }
                }

                // Sender info
                div {
                    class: "border rounded-md p-4 bg-blue-50",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                        "🤝 From: {payload.sender_player}"
                    }
                    p {
                        class: "text-sm text-gray-600 mb-3",
                        "{payload.sender_village} ({payload.sender_position.x}|{payload.sender_position.y})"
                    }
                }

                // Receiver info
                div {
                    class: "border rounded-md p-4 bg-green-50",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                        "🏰 To: {payload.receiver_player}"
                    }
                    p {
                        class: "text-sm text-gray-600 mb-3",
                        "{payload.receiver_village} ({payload.receiver_position.x}|{payload.receiver_position.y})"
                    }
                }

                // Troops sent (no losses row)
                div {
                    class: "border rounded-md p-4",
                    p {
                        class: "text-xs uppercase text-gray-500 font-semibold mb-3",
                        "Troops Sent"
                    }
                    ReinforcementArmyTable {
                        tribe: payload.tribe.clone(),
                        units: payload.units
                    }
                }
            }
        }
    }
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
                            {
                                let verb = match (payload.attack_type, payload.scouting.is_some()) {
                                    (_, true) => "scouted",
                                    (AttackType::Raid, _) => "raided",
                                    (AttackType::Normal, _) => "attacked",
                                };

                                format!("{} {} {}", payload.attacker_village, verb, payload.defender_village)
                            }
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
                            "{payload.attacker_village} ({payload.attacker_position.x}|{payload.attacker_position.y})"
                        }
                        BattleArmyTable {
                            tribe: attacker.tribe.clone(),
                            army_before: attacker.army_before,
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
                            "{payload.defender_village} ({payload.defender_position.x}|{payload.defender_position.y})"
                        }
                        BattleArmyTable {
                            tribe: defender.tribe.clone(),
                            army_before: defender.army_before,
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
                                BattleArmyTable {
                                    tribe: reinf.tribe.clone(),
                                    army_before: reinf.army_before,
                                    losses: reinf.losses
                                }
                            }
                        }
                    }
                }

                // Scouting Information
                if let Some(ref scouting) = payload.scouting {
                    div {
                        class: "border rounded-md p-4 bg-blue-50",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                            "🔍 Scouting Report"
                        }
                        if scouting.was_detected {
                            p {
                                class: "text-sm text-orange-600 mb-2",
                                "⚠️ Scouts were detected"
                            }
                        } else {
                            p {
                                class: "text-sm text-green-600 mb-2",
                                "✓ Scouts were not detected"
                            }
                        }
                        {
                            match &scouting.target_report {
                                parabellum_types::battle::ScoutingTargetReport::Resources(resources) => {
                                    rsx! {
                                        p {
                                            class: "text-xs text-gray-500 mb-1",
                                            "Scouted Resources:"
                                        }
                                        p {
                                            class: "font-mono text-gray-800",
                                            "🌲 {resources.lumber()} | 🧱 {resources.clay()} | ⛏️ {resources.iron()} | 🌾 {resources.crop()}"
                                        }
                                    }
                                }
                                parabellum_types::battle::ScoutingTargetReport::Defenses { wall, palace, residence } => {
                                    rsx! {
                                        p {
                                            class: "text-xs text-gray-500 mb-1",
                                            "Scouted Defenses:"
                                        }
                                        div {
                                            class: "space-y-1 text-sm text-gray-700",
                                            if let Some(wall_level) = wall {
                                                p { "🏰 Wall: Level {wall_level}" }
                                            }
                                            if let Some(palace_level) = palace {
                                                p { "👑 Palace: Level {palace_level}" }
                                            }
                                            if let Some(residence_level) = residence {
                                                p { "🏛️ Residence: Level {residence_level}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Wall Damage (Rams)
                if let Some(ref wall_dmg) = payload.wall_damage {
                    div {
                        class: "border rounded-md p-4 bg-orange-50",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                            "🐏 Ram Damage"
                        }
                        p {
                            class: "text-sm text-gray-700",
                            "{wall_dmg.name} destroyed: Level {wall_dmg.level_before} → Level {wall_dmg.level_after}"
                        }
                    }
                }

                // Catapult Damage
                if !payload.catapult_damage.is_empty() {
                    div {
                        class: "border rounded-md p-4 bg-red-50",
                        p {
                            class: "text-xs uppercase text-gray-500 font-semibold mb-2",
                            "🎯 Catapult Damage"
                        }
                        div {
                            class: "space-y-1",
                            for dmg in &payload.catapult_damage {
                                p {
                                    class: "text-sm text-gray-700",
                                    "{dmg.name} destroyed: Level {dmg.level_before} → Level {dmg.level_after}"
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
