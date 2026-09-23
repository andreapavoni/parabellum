//! Rally point card composition helpers.
//!
//! This module maps app-facing army and movement read models into a web-local
//! intermediate shape. DTO serialization stays in `buildings.rs`.

use std::collections::{HashMap, HashSet};

use parabellum_app::{
    read_models::VillageReference,
    villages::read_models::{
        TroopMovement, TroopMovementType, VillageArmyStateView, VillageTroopMovements,
    },
};
use parabellum_game::models::army::Army;
use parabellum_types::{army::TroopSet, common::ResourceGroup, map::Position, tribe::Tribe};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MovementKind {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ArmyCategory {
    Stationed,
    Reinforcement,
    Deployed,
    Trapped,
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ArmyAction {
    Recall { army_id: String },
    Release { army_id: String },
    Cancel { movement_id: String },
    ReleaseTrapped { army_id: String },
    DisbandTrapped { army_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ArmyCardData {
    pub(super) village_id: u32,
    pub(super) village_name: Option<String>,
    pub(super) position: Option<Position>,
    pub(super) units: TroopSet,
    pub(super) has_hero: bool,
    pub(super) tribe: Tribe,
    pub(super) expose_composition: bool,
    pub(super) category: ArmyCategory,
    pub(super) movement_kind: Option<MovementKind>,
    pub(super) arrives_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(super) bounty: Option<ResourceGroup>,
    pub(super) action_button: Option<ArmyAction>,
}

pub(super) fn prepare_rally_point_cards(
    village_id: u32,
    village_name: &str,
    village_position: &Position,
    village_tribe: &Tribe,
    armies: &VillageArmyStateView,
    movements: &VillageTroopMovements,
    village_references: &HashMap<u32, VillageReference>,
    cancelable_movement_ids: &HashSet<uuid::Uuid>,
) -> Vec<ArmyCardData> {
    let mut cards = Vec::new();

    if let Some(army) = &armies.home_army {
        cards.push(stationed_army_card(
            village_id,
            village_name,
            village_position,
            village_tribe,
            army,
        ));
    }

    for army in &armies.deployed_armies {
        cards.push(deployed_army_card(village_id, army, village_references));
    }

    for reinforcement in &armies.reinforcements {
        cards.push(stationed_reinforcement_card(
            reinforcement,
            village_references,
        ));
    }

    for trapped in &armies.trapped_here {
        cards.push(trapped_here_card(trapped, village_references));
    }

    for trapped in &armies.trapped_away {
        cards.push(trapped_away_card(village_id, trapped, village_references));
    }

    for movement in &movements.outgoing {
        if let Some(card) = outgoing_movement_card(movement, cancelable_movement_ids) {
            cards.push(card);
        }
    }

    for movement in &movements.incoming {
        cards.push(incoming_movement_card(movement));
    }

    cards
}

fn village_reference(
    village_references: &HashMap<u32, VillageReference>,
    village_id: u32,
) -> (Option<String>, Option<Position>) {
    village_references
        .get(&village_id)
        .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
        .unwrap_or_else(|| (Some(format!("Village #{}", village_id)), None))
}

fn stationed_army_card(
    village_id: u32,
    village_name: &str,
    village_position: &Position,
    village_tribe: &Tribe,
    army: &Army,
) -> ArmyCardData {
    ArmyCardData {
        village_id,
        village_name: Some(village_name.to_string()),
        position: Some(village_position.clone()),
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
        tribe: village_tribe.clone(),
        expose_composition: true,
        category: ArmyCategory::Stationed,
        movement_kind: None,
        arrives_at: None,
        bounty: None,
        action_button: None,
    }
}

fn deployed_army_card(
    current_village_id: u32,
    army: &Army,
    village_references: &HashMap<u32, VillageReference>,
) -> ArmyCardData {
    let destination_id = army.current_map_field_id.unwrap_or(current_village_id);
    let (destination_name, destination_position) =
        village_reference(village_references, destination_id);

    ArmyCardData {
        village_id: destination_id,
        village_name: destination_name,
        position: destination_position,
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
        tribe: army.tribe.clone(),
        expose_composition: true,
        category: ArmyCategory::Deployed,
        movement_kind: None,
        arrives_at: None,
        bounty: None,
        action_button: Some(ArmyAction::Recall {
            army_id: army.id.to_string(),
        }),
    }
}

fn stationed_reinforcement_card(
    army: &Army,
    village_references: &HashMap<u32, VillageReference>,
) -> ArmyCardData {
    let origin_id = army.village_id;
    let (origin_name, origin_position) = village_reference(village_references, origin_id);

    ArmyCardData {
        village_id: origin_id,
        village_name: origin_name,
        position: origin_position,
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
        tribe: army.tribe.clone(),
        expose_composition: true,
        category: ArmyCategory::Reinforcement,
        movement_kind: None,
        arrives_at: None,
        bounty: None,
        action_button: Some(ArmyAction::Release {
            army_id: army.id.to_string(),
        }),
    }
}

fn trapped_here_card(
    army: &Army,
    village_references: &HashMap<u32, VillageReference>,
) -> ArmyCardData {
    let origin_id = army.village_id;
    let (origin_name, origin_position) = village_reference(village_references, origin_id);

    ArmyCardData {
        village_id: origin_id,
        village_name: origin_name,
        position: origin_position,
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
        tribe: army.tribe.clone(),
        expose_composition: true,
        category: ArmyCategory::Trapped,
        movement_kind: None,
        arrives_at: None,
        bounty: None,
        action_button: Some(ArmyAction::ReleaseTrapped {
            army_id: army.id.to_string(),
        }),
    }
}

fn trapped_away_card(
    current_village_id: u32,
    army: &Army,
    village_references: &HashMap<u32, VillageReference>,
) -> ArmyCardData {
    let destination_id = army.current_map_field_id.unwrap_or(current_village_id);
    let (destination_name, destination_position) =
        village_reference(village_references, destination_id);

    ArmyCardData {
        village_id: destination_id,
        village_name: destination_name,
        position: destination_position,
        units: army.units().clone(),
        has_hero: army.hero().is_some(),
        tribe: army.tribe.clone(),
        expose_composition: true,
        category: ArmyCategory::Trapped,
        movement_kind: None,
        arrives_at: None,
        bounty: None,
        action_button: Some(ArmyAction::DisbandTrapped {
            army_id: army.id.to_string(),
        }),
    }
}

fn outgoing_movement_card(
    movement: &TroopMovement,
    cancelable_movement_ids: &HashSet<uuid::Uuid>,
) -> Option<ArmyCardData> {
    if movement.movement_type == TroopMovementType::Return {
        return None;
    }

    let action_button = if cancelable_movement_ids.contains(&movement.job_id) {
        Some(ArmyAction::Cancel {
            movement_id: movement.job_id.to_string(),
        })
    } else {
        None
    };

    Some(ArmyCardData {
        village_id: movement.target_village_id,
        village_name: movement.target_village_name.clone(),
        position: Some(movement.target_position.clone()),
        units: movement.units.clone(),
        has_hero: movement.has_hero,
        tribe: movement.tribe.clone(),
        expose_composition: movement.exposes_army_composition_to_viewer(),
        category: ArmyCategory::Outgoing,
        movement_kind: Some(movement_kind_to_card_kind(movement.movement_type)),
        arrives_at: Some(movement.arrives_at),
        bounty: movement.bounty.clone(),
        action_button,
    })
}

fn incoming_movement_card(movement: &TroopMovement) -> ArmyCardData {
    ArmyCardData {
        village_id: movement.origin_village_id,
        village_name: movement.origin_village_name.clone(),
        position: Some(movement.origin_position.clone()),
        units: movement.units.clone(),
        has_hero: movement.has_hero,
        tribe: movement.tribe.clone(),
        expose_composition: movement.exposes_army_composition_to_viewer(),
        category: ArmyCategory::Incoming,
        movement_kind: Some(movement_kind_to_card_kind(movement.movement_type)),
        arrives_at: Some(movement.arrives_at),
        bounty: movement.bounty.clone(),
        action_button: None,
    }
}

fn movement_kind_to_card_kind(kind: TroopMovementType) -> MovementKind {
    match kind {
        TroopMovementType::Attack => MovementKind::Attack,
        TroopMovementType::Raid => MovementKind::Raid,
        TroopMovementType::Scout => MovementKind::Scout,
        TroopMovementType::Reinforcement => MovementKind::Reinforcement,
        TroopMovementType::Return => MovementKind::Return,
        TroopMovementType::FoundVillage => MovementKind::FoundVillage,
    }
}
