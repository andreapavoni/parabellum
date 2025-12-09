use dioxus::prelude::*;
use parabellum_types::{map::Position, tribe::Tribe};
use serde::{Deserialize, Serialize};

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

/// Army card category for rally point display
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ArmyCategory {
    Stationed,
    Reinforcement,
    Deployed,
    Incoming,
    Outgoing,
}

impl ArmyCategory {
    pub fn badge_color(&self) -> &'static str {
        match self {
            ArmyCategory::Stationed => "bg-gray-100 text-gray-800",
            ArmyCategory::Reinforcement => "bg-blue-100 text-blue-800",
            ArmyCategory::Deployed => "bg-purple-100 text-purple-800",
            ArmyCategory::Incoming => "bg-red-100 text-red-800",
            ArmyCategory::Outgoing => "bg-green-100 text-green-800",
        }
    }
}

/// Army action button for rally point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArmyAction {
    Recall { movement_id: String }, // Recall troops from outgoing reinforcement
    Release { source_village_id: u32 }, // Release reinforcements back to their village
}

/// Detailed army card data with full unit roster
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmyCardData {
    pub village_id: u32,
    pub village_name: Option<String>,
    pub position: Option<Position>,
    pub units: [u32; 10],
    pub tribe: Tribe,
    pub category: ArmyCategory,
    pub movement_kind: Option<MovementKind>,
    pub arrival_time: Option<u32>, // seconds remaining
    pub action_button: Option<ArmyAction>,
}

/// Army card component for rally point display
#[component]
pub fn ArmyCard(card: ArmyCardData, csrf_token: String) -> Element {
    use crate::view_helpers::{format_duration, unit_display_name};

    let movement_kind_class = card.movement_kind.map(|kind| match kind {
        MovementKind::Attack => "text-xs px-2 py-0.5 rounded bg-red-100 text-red-800",
        MovementKind::Raid => "text-xs px-2 py-0.5 rounded bg-orange-100 text-orange-800",
        MovementKind::Reinforcement => "text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-800",
        MovementKind::Return => "text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-800",
    });

    let tribe_units = card.tribe.units();
    let title = card.village_name.as_deref().unwrap_or("Unknown Village");
    let subtitle = card
        .position
        .as_ref()
        .map(|p| format!("({}, {})", p.x, p.y));

    rsx! {
        div { class: "border rounded-lg p-4 bg-white shadow-sm space-y-3",
            // Card header with village name, position, movement kind, and category badge
            div { class: "flex justify-between items-start",
                div { class: "flex-1",
                    div { class: "flex items-center gap-2",
                        h3 { class: "font-semibold text-gray-900", "{title}" }
                        if let Some(kind) = card.movement_kind {
                            span {
                                class: "{movement_kind_class.unwrap()}",
                                "{kind:?}"
                            }
                        }
                    }
                    if let Some(ref pos) = subtitle {
                        p { class: "text-sm text-gray-600 mt-1", "{pos}" }
                    }
                    if let Some(arrival_time) = card.arrival_time {
                        p {
                            class: "text-sm text-gray-500 mt-1 font-mono countdown-timer",
                            "data-seconds": "{arrival_time}",
                            "⏱️ {format_duration(arrival_time)}"
                        }
                    }
                }
                span {
                    class: "text-xs px-2 py-1 rounded font-medium whitespace-nowrap {card.category.badge_color()}",
                    "{card.category:?}"
                }
            }

            // Units display - grid showing all units
            div { class: "grid grid-cols-5 gap-2",
                for (idx, &count) in card.units.iter().enumerate() {
                    {
                        let unit_name = if idx < tribe_units.len() {
                            unit_display_name(&tribe_units[idx].name)
                        } else {
                            "?".to_string()
                        };
                        let is_zero = count == 0;
                        rsx! {
                            div {
                                class: "text-center p-2 rounded",
                                class: if is_zero { "bg-gray-50 opacity-40" } else { "bg-gray-100" },
                                div {
                                    class: "text-xs text-gray-600 truncate",
                                    title: "{unit_name}",
                                    "{unit_name}"
                                }
                                div {
                                    class: if is_zero { "text-gray-400 text-sm" } else { "text-gray-900 font-semibold" },
                                    "{count}"
                                }
                            }
                        }
                    }
                }
            }

            // Action button if applicable
            if let Some(action) = &card.action_button {
                div { class: "pt-2 border-t",
                    match action {
                        ArmyAction::Recall { movement_id } => rsx! {
                            form {
                                method: "post",
                                action: "/army/recall",
                                class: "flex gap-2",
                                input { r#type: "hidden", name: "movement_id", value: "{movement_id}" }
                                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                button {
                                    r#type: "submit",
                                    class: "px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white text-sm rounded",
                                    "↩️ Recall Troops"
                                }
                            }
                        },
                        ArmyAction::Release { source_village_id } => rsx! {
                            form {
                                method: "post",
                                action: "/army/release",
                                class: "flex gap-2",
                                input { r#type: "hidden", name: "source_village_id", value: "{source_village_id}" }
                                input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                                button {
                                    r#type: "submit",
                                    class: "px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded",
                                    "🏠 Release Reinforcements"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
