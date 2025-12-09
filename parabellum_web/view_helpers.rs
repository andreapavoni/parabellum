use chrono::Utc;
use parabellum_app::cqrs::queries::{
    BuildingQueueItem, TrainingQueueItem, TroopMovementType, VillageTroopMovements,
};
use parabellum_app::jobs::JobStatus;
use parabellum_game::models::village::Village;
use parabellum_types::{army::UnitName, buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;
use uuid::Uuid;

use crate::components::{ArmyAction, ArmyCardData, ArmyCategory, MovementKind};

#[derive(Debug, Clone)]
pub struct BuildingQueueItemView {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub is_processing: bool,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub queue_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnitTrainingQueueItemView {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit_name: String,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub time_remaining: String,
    pub time_seconds: u32,
}

/// Formats a duration in seconds to HH:MM:SS.
pub fn format_duration(total_seconds: u32) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Converts queue items into view representations with formatted timers.
pub fn building_queue_to_views(items: &[BuildingQueueItem]) -> Vec<BuildingQueueItemView> {
    let now = Utc::now();
    items
        .iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            BuildingQueueItemView {
                job_id: item.job_id,
                slot_id: item.slot_id,
                building_name: item.building_name.clone(),
                target_level: item.target_level,
                is_processing: matches!(item.status, JobStatus::Processing),
                time_remaining: format_duration(remaining),
                time_seconds: remaining,
                queue_class: None,
            }
        })
        .collect()
}

/// Converts unit training jobs into view models with countdown timers.
pub fn training_queue_to_views(items: &[TrainingQueueItem]) -> Vec<UnitTrainingQueueItemView> {
    let now = Utc::now();
    items
        .iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            UnitTrainingQueueItemView {
                job_id: item.job_id,
                slot_id: item.slot_id,
                unit_name: unit_display_name(&item.unit),
                quantity: item.quantity,
                time_per_unit: item.time_per_unit,
                time_remaining: format_duration(remaining),
                time_seconds: remaining,
            }
        })
        .collect()
}

/// Returns the localized display name for a unit.
pub fn unit_display_name(unit: &UnitName) -> String {
    let name = match unit {
        UnitName::Legionnaire => t!("game.units.romans.legionnaire"),
        UnitName::Praetorian => t!("game.units.romans.praetorian"),
        UnitName::Imperian => t!("game.units.romans.imperian"),
        UnitName::EquitesLegati => t!("game.units.romans.equites_legati"),
        UnitName::EquitesImperatoris => t!("game.units.romans.equites_imperatoris"),
        UnitName::EquitesCaesaris => t!("game.units.romans.equites_caesaris"),
        UnitName::BatteringRam => t!("game.units.romans.battering_ram"),
        UnitName::FireCatapult => t!("game.units.romans.fire_catapult"),
        UnitName::Senator => t!("game.units.romans.senator"),
        UnitName::Settler => t!("game.units.romans.settler"),
        UnitName::Maceman => t!("game.units.teutons.maceman"),
        UnitName::Spearman => t!("game.units.teutons.spearman"),
        UnitName::Axeman => t!("game.units.teutons.axeman"),
        UnitName::Scout => t!("game.units.teutons.scout"),
        UnitName::Paladin => t!("game.units.teutons.paladin"),
        UnitName::TeutonicKnight => t!("game.units.teutons.teutonic_knight"),
        UnitName::Ram => t!("game.units.teutons.ram"),
        UnitName::Catapult => t!("game.units.teutons.catapult"),
        UnitName::Chief => t!("game.units.teutons.chief"),
        UnitName::Phalanx => t!("game.units.gauls.phalanx"),
        UnitName::Swordsman => t!("game.units.gauls.swordsman"),
        UnitName::Pathfinder => t!("game.units.gauls.pathfinder"),
        UnitName::TheutatesThunder => t!("game.units.gauls.theutates_thunder"),
        UnitName::Druidrider => t!("game.units.gauls.druidrider"),
        UnitName::Haeduan => t!("game.units.gauls.haeduan"),
        UnitName::Trebuchet => t!("game.units.gauls.trebuchet"),
        UnitName::Chieftain => t!("game.units.gauls.chieftain"),
        UnitName::Rat => t!("game.units.nature.rat"),
        UnitName::Spider => t!("game.units.nature.spider"),
        UnitName::Serpent => t!("game.units.nature.snake"),
        UnitName::Bat => t!("game.units.nature.bat"),
        UnitName::WildBoar => t!("game.units.nature.wild_boar"),
        UnitName::Wolf => t!("game.units.nature.wolf"),
        UnitName::Bear => t!("game.units.nature.bear"),
        UnitName::Crocodile => t!("game.units.nature.crocodile"),
        UnitName::Tiger => t!("game.units.nature.tiger"),
        UnitName::Elephant => t!("game.units.nature.elephant"),
        UnitName::Pikeman => t!("game.units.natars.pikeman"),
        UnitName::ThornedWarrior => t!("game.units.natars.thorned_warrior"),
        UnitName::Guardsman => t!("game.units.natars.guardsman"),
        UnitName::BirdsOfPrey => t!("game.units.natars.birds_of_prey"),
        UnitName::Axerider => t!("game.units.natars.axerider"),
        UnitName::NatarianKnight => t!("game.units.natars.natarian_knight"),
        UnitName::Warelephant => t!("game.units.natars.warelephant"),
        UnitName::Ballista => t!("game.units.natars.ballista"),
        UnitName::NatarianEmperor => t!("game.units.natars.natarian_emperor"),
        // _ => return format!("{:?}", unit),
    };

    name.to_string()
}

/// Formats a resource group into a short inline summary.
pub fn format_resource_summary(resources: &ResourceGroup) -> String {
    format!(
        "🌲 {} 🧱 {} ⛏️ {} 🌾 {}",
        resources.lumber(),
        resources.clay(),
        resources.iron(),
        resources.crop()
    )
}

/// Prepares all army cards for the Rally Point page from domain data.
/// This function transforms:
/// - Village's stationed army
/// - Village's deployed armies (own troops elsewhere)
/// - Village's reinforcements (foreign troops helping)
/// - Outgoing troop movements
/// - Incoming troop movements
pub fn prepare_rally_point_cards(
    village: &Village,
    movements: &VillageTroopMovements,
) -> Vec<ArmyCardData> {
    let mut cards = Vec::new();

    // 1. Stationed troops (home army)
    if let Some(army) = village.army() {
        cards.push(ArmyCardData {
            village_id: village.id,
            village_name: Some(village.name.clone()),
            position: Some(village.position.clone()),
            units: *army.units(),
            tribe: village.tribe.clone(),
            category: ArmyCategory::Stationed,
            movement_kind: None,
            arrival_time: None,
            action_button: None,
        });
    }

    // 2. Deployed armies (own troops in other villages/oases)
    for army in village.deployed_armies() {
        // TODO: Need village names for destination - will be enriched in Step 8
        let destination_name = army
            .current_map_field_id
            .map(|id| format!("Village #{}", id));

        cards.push(ArmyCardData {
            village_id: army.current_map_field_id.unwrap_or(village.id),
            village_name: destination_name,
            position: None, // TODO: Position lookup in Step 8
            units: *army.units(),
            tribe: army.tribe.clone(),
            category: ArmyCategory::Deployed,
            movement_kind: None,
            arrival_time: None,
            action_button: Some(ArmyAction::Recall {
                movement_id: army.id.to_string(),
            }),
        });
    }

    // 3. Reinforcements (troops from other players helping us)
    for reinforcement in village.reinforcements() {
        // TODO: Need village names for origin - will be enriched in Step 8
        let origin_name = Some(format!("Village #{}", reinforcement.village_id));

        cards.push(ArmyCardData {
            village_id: reinforcement.village_id,
            village_name: origin_name,
            position: None, // TODO: Position lookup in Step 8
            units: *reinforcement.units(),
            tribe: reinforcement.tribe.clone(),
            category: ArmyCategory::Reinforcement,
            movement_kind: None,
            arrival_time: None,
            action_button: Some(ArmyAction::Release {
                source_village_id: reinforcement.village_id,
            }),
        });
    }

    // 4. Outgoing movements
    for movement in &movements.outgoing {
        let now = chrono::Utc::now();
        let time_remaining_secs = (movement.arrives_at - now).num_seconds().max(0) as u32;

        let movement_kind = match movement.movement_type {
            TroopMovementType::Attack => MovementKind::Attack,
            TroopMovementType::Raid => MovementKind::Raid,
            TroopMovementType::Reinforcement => MovementKind::Reinforcement,
            TroopMovementType::Return => MovementKind::Return,
        };

        let action_button = if matches!(movement_kind, MovementKind::Reinforcement) {
            Some(ArmyAction::Recall {
                movement_id: movement.job_id.to_string(),
            })
        } else {
            None
        };

        cards.push(ArmyCardData {
            village_id: movement.target_village_id,
            village_name: movement.target_village_name.clone(),
            position: Some(movement.target_position.clone()),
            units: movement.units,
            tribe: movement.tribe.clone(),
            category: ArmyCategory::Outgoing,
            movement_kind: Some(movement_kind),
            arrival_time: Some(time_remaining_secs),
            action_button,
        });
    }

    // 5. Incoming movements
    for movement in &movements.incoming {
        let now = chrono::Utc::now();
        let time_remaining_secs = (movement.arrives_at - now).num_seconds().max(0) as u32;

        let movement_kind = match movement.movement_type {
            TroopMovementType::Attack => MovementKind::Attack,
            TroopMovementType::Raid => MovementKind::Raid,
            TroopMovementType::Reinforcement => MovementKind::Reinforcement,
            TroopMovementType::Return => MovementKind::Return,
        };

        cards.push(ArmyCardData {
            village_id: movement.origin_village_id,
            village_name: movement.origin_village_name.clone(),
            position: Some(movement.origin_position.clone()),
            units: movement.units,
            tribe: movement.tribe.clone(),
            category: ArmyCategory::Incoming,
            movement_kind: Some(movement_kind),
            arrival_time: Some(time_remaining_secs),
            action_button: None,
        });
    }

    cards
}
