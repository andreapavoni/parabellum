use chrono::Utc;
use parabellum_app::cqrs::queries::{
    BuildingQueueItem, MarketplaceData, MerchantMovement, MerchantMovementKind, TrainingQueueItem,
    TroopMovementType, VillageTroopMovements,
};
use parabellum_app::jobs::JobStatus;
use parabellum_app::repository::VillageInfo;
use parabellum_game::models::village::Village;
use parabellum_types::{
    army::UnitName, buildings::BuildingName, common::ResourceGroup, map::Position,
};
use rust_i18n::t;
use std::collections::HashMap;
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
///
/// The `village_info` map provides names and positions for villages referenced by armies.
pub fn prepare_rally_point_cards(
    village: &Village,
    movements: &VillageTroopMovements,
    village_info: &HashMap<u32, VillageInfo>,
) -> Vec<ArmyCardData> {
    let mut cards = Vec::new();

    // 1. Stationed troops (home army)
    if let Some(army) = village.army() {
        cards.push(ArmyCardData {
            village_id: village.id,
            village_name: Some(village.name.clone()),
            position: Some(village.position.clone()),
            units: army.units().clone(),
            tribe: village.tribe.clone(),
            category: ArmyCategory::Stationed,
            movement_kind: None,
            arrival_time: None,
            action_button: None,
        });
    }

    // 2. Deployed armies (own troops in other villages/oases)
    for army in village.deployed_armies() {
        let destination_id = army.current_map_field_id.unwrap_or(village.id);
        let (destination_name, destination_position) = village_info
            .get(&destination_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", destination_id)), None));

        cards.push(ArmyCardData {
            village_id: destination_id,
            village_name: destination_name,
            position: destination_position,
            units: army.units().clone(),
            tribe: army.tribe.clone(),
            category: ArmyCategory::Deployed,
            movement_kind: None,
            arrival_time: None,
            action_button: Some(ArmyAction::Recall {
                army_id: army.id.to_string(),
            }),
        });
    }

    // 3. Reinforcements (troops from other players helping us)
    for reinforcement in village.reinforcements() {
        let origin_id = reinforcement.village_id;
        let (origin_name, origin_position) = village_info
            .get(&origin_id)
            .map(|info| (Some(info.name.clone()), Some(info.position.clone())))
            .unwrap_or_else(|| (Some(format!("Village #{}", origin_id)), None));

        cards.push(ArmyCardData {
            village_id: origin_id,
            village_name: origin_name,
            position: origin_position,
            units: reinforcement.units().clone(),
            tribe: reinforcement.tribe.clone(),
            category: ArmyCategory::Reinforcement,
            movement_kind: None,
            arrival_time: None,
            action_button: Some(ArmyAction::Release {
                army_id: reinforcement.id.to_string(),
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
            TroopMovementType::FoundVillage => MovementKind::FoundVillage,
        };

        // No action buttons for traveling armies - can only recall once deployed
        let action_button = None;

        cards.push(ArmyCardData {
            village_id: movement.target_village_id,
            village_name: movement.target_village_name.clone(),
            position: Some(movement.target_position.clone()),
            units: movement.units.clone(),
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
            TroopMovementType::FoundVillage => MovementKind::FoundVillage,
        };

        cards.push(ArmyCardData {
            village_id: movement.origin_village_id,
            village_name: movement.origin_village_name.clone(),
            position: Some(movement.origin_position.clone()),
            units: movement.units.clone(),
            tribe: movement.tribe.clone(),
            category: ArmyCategory::Incoming,
            movement_kind: Some(movement_kind),
            arrival_time: Some(time_remaining_secs),
            action_button: None,
        });
    }

    cards
}

/// Returns the localized description for a building.
pub fn building_description(building: &BuildingName) -> String {
    let description = match building {
        BuildingName::Woodcutter => t!("game.buildings.woodcutter.description"),
        BuildingName::ClayPit => t!("game.buildings.clay_pit.description"),
        BuildingName::IronMine => t!("game.buildings.iron_mine.description"),
        BuildingName::Cropland => t!("game.buildings.cropland.description"),
        BuildingName::Sawmill => t!("game.buildings.sawmill.description"),
        BuildingName::Brickyard => t!("game.buildings.brickyard.description"),
        BuildingName::IronFoundry => t!("game.buildings.iron_foundry.description"),
        BuildingName::GrainMill => t!("game.buildings.grain_mill.description"),
        BuildingName::Bakery => t!("game.buildings.bakery.description"),
        BuildingName::Warehouse => t!("game.buildings.warehouse.description"),
        BuildingName::Granary => t!("game.buildings.granary.description"),
        BuildingName::Smithy => t!("game.buildings.smithy.description"),
        BuildingName::TournamentSquare => t!("game.buildings.tournament_square.description"),
        BuildingName::MainBuilding => t!("game.buildings.main_building.description"),
        BuildingName::RallyPoint => t!("game.buildings.rally_point.description"),
        BuildingName::Marketplace => t!("game.buildings.marketplace.description"),
        BuildingName::Embassy => t!("game.buildings.embassy.description"),
        BuildingName::Barracks => t!("game.buildings.barracks.description"),
        BuildingName::Stable => t!("game.buildings.stable.description"),
        BuildingName::Workshop => t!("game.buildings.workshop.description"),
        BuildingName::Academy => t!("game.buildings.academy.description"),
        BuildingName::Cranny => t!("game.buildings.cranny.description"),
        BuildingName::TownHall => t!("game.buildings.town_hall.description"),
        BuildingName::Residence => t!("game.buildings.residence.description"),
        BuildingName::Palace => t!("game.buildings.palace.description"),
        BuildingName::Treasury => t!("game.buildings.treasury.description"),
        BuildingName::TradeOffice => t!("game.buildings.trade_office.description"),
        BuildingName::GreatBarracks => t!("game.buildings.great_barracks.description"),
        BuildingName::GreatStable => t!("game.buildings.great_stable.description"),
        BuildingName::CityWall => t!("game.buildings.city_wall.description"),
        BuildingName::EarthWall => t!("game.buildings.earth_wall.description"),
        BuildingName::Palisade => t!("game.buildings.palisade.description"),
        BuildingName::StonemansionLodge => t!("game.buildings.stonemason.description"),
        BuildingName::Brewery => t!("game.buildings.brewery.description"),
        BuildingName::Trapper => t!("game.buildings.trapper.description"),
        BuildingName::HeroMansion => t!("game.buildings.heros_mansion.description"),
        BuildingName::GreatWarehouse => t!("game.buildings.great_warehouse.description"),
        BuildingName::GreatGranary => t!("game.buildings.great_granary.description"),
        BuildingName::WonderOfTheWorld => t!("game.buildings.wonder.description"),
        BuildingName::HorseDrinkingTrough => t!("game.buildings.horse_drinking.description"),
        BuildingName::GreatWorkshop => t!("game.buildings.great_workshop.description"),
        BuildingName::AncientConstructionPlan => t!(""), // No description in i18n
    };

    description.to_string()
}

/// Returns building description paragraphs split by <br> tags
pub fn building_description_paragraphs(building: &BuildingName) -> Vec<String> {
    let raw_description = building_description(building);

    // Split by <br> tag and clean up whitespace
    raw_description
        .split("<br>")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ============================================================================
// Marketplace View Helpers
// ============================================================================

/// View model for marketplace offers displayed in tables
#[derive(Debug, Clone)]
pub struct MarketplaceOfferView {
    pub offer_id: String,
    pub village_id: u32,
    pub village_name: String,
    pub position: Position,
    pub offer_resources: ResourceGroup,
    pub seek_resources: ResourceGroup,
    pub merchants_required: u8,
    pub created_at_text: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MerchantMovementDirection {
    Outgoing,
    Incoming,
}

/// View model for merchant movements on the marketplace page
#[derive(Debug, Clone)]
pub struct MerchantMovementView {
    pub job_id: String,
    pub direction: MerchantMovementDirection,
    pub kind: MerchantMovementKind,
    pub origin_name: String,
    pub origin_position: Option<Position>,
    pub destination_name: String,
    pub destination_position: Option<Position>,
    pub resources: ResourceGroup,
    pub merchants_used: u8,
    pub time_remaining_secs: u32,
}

/// Formats a trade offer as "Offering → Seeking" with resource emojis
pub fn format_trade_offer(offer: &ResourceGroup, seek: &ResourceGroup) -> String {
    let offer_parts: Vec<String> = [
        (offer.lumber(), "🌲"),
        (offer.clay(), "🧱"),
        (offer.iron(), "⛏️"),
        (offer.crop(), "🌾"),
    ]
    .iter()
    .filter(|(amount, _)| *amount > 0)
    .map(|(amount, emoji)| format!("{} {}", amount, emoji))
    .collect();

    let seek_parts: Vec<String> = [
        (seek.lumber(), "🌲"),
        (seek.clay(), "🧱"),
        (seek.iron(), "⛏️"),
        (seek.crop(), "🌾"),
    ]
    .iter()
    .filter(|(amount, _)| *amount > 0)
    .map(|(amount, emoji)| format!("{} {}", amount, emoji))
    .collect();

    format!("{} → {}", offer_parts.join(" "), seek_parts.join(" "))
}

/// Converts MarketplaceData into view models for own offers
pub fn prepare_own_offers(marketplace_data: &MarketplaceData) -> Vec<MarketplaceOfferView> {
    marketplace_data
        .own_offers
        .iter()
        .map(|offer| {
            let village_info = marketplace_data
                .village_info
                .get(&offer.village_id)
                .expect("Village info should exist for own offer");

            MarketplaceOfferView {
                offer_id: offer.id.to_string(),
                village_id: offer.village_id,
                village_name: village_info.name.clone(),
                position: village_info.position.clone(),
                offer_resources: offer.offer_resources.clone(),
                seek_resources: offer.seek_resources.clone(),
                merchants_required: offer.merchants_required,
                created_at_text: format_relative_time(offer.created_at),
            }
        })
        .collect()
}

/// Converts MarketplaceData into view models for global offers (sorted by distance)
pub fn prepare_global_offers(marketplace_data: &MarketplaceData) -> Vec<MarketplaceOfferView> {
    marketplace_data
        .global_offers
        .iter()
        .map(|offer| {
            let village_info = marketplace_data
                .village_info
                .get(&offer.village_id)
                .expect("Village info should exist for global offer");

            MarketplaceOfferView {
                offer_id: offer.id.to_string(),
                village_id: offer.village_id,
                village_name: village_info.name.clone(),
                position: village_info.position.clone(),
                offer_resources: offer.offer_resources.clone(),
                seek_resources: offer.seek_resources.clone(),
                merchants_required: offer.merchants_required,
                created_at_text: format_relative_time(offer.created_at),
            }
        })
        .collect()
}

/// Converts merchant movements into view models for the marketplace page
pub fn prepare_merchant_movements(
    movements: &[MerchantMovement],
    village_info: &HashMap<u32, VillageInfo>,
    direction: MerchantMovementDirection,
) -> Vec<MerchantMovementView> {
    let now = Utc::now();
    movements
        .iter()
        .map(|movement| {
            let origin_info = village_info.get(&movement.origin_village_id);
            let destination_info = village_info.get(&movement.destination_village_id);
            let time_remaining_secs = (movement.arrives_at - now).num_seconds().max(0) as u32;

            MerchantMovementView {
                job_id: movement.job_id.to_string(),
                direction: direction.clone(),
                kind: movement.kind.clone(),
                origin_name: origin_info
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| format!("Village #{}", movement.origin_village_id)),
                origin_position: origin_info.map(|info| info.position.clone()),
                destination_name: destination_info
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| format!("Village #{}", movement.destination_village_id)),
                destination_position: destination_info.map(|info| info.position.clone()),
                resources: movement.resources.clone(),
                merchants_used: movement.merchants_used,
                time_remaining_secs,
            }
        })
        .collect()
}

/// Formats a timestamp as a relative time string (e.g., "5 minutes ago")
fn format_relative_time(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    }
}
