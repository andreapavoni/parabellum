use chrono::Utc;
use parabellum_app::{
    cqrs::queries::{AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem},
    jobs::JobStatus,
};
use parabellum_game::models::village::{Village, VillageBuilding};
use parabellum_types::{army::UnitName, buildings::BuildingName};
use rust_i18n::t;
use std::collections::HashMap;

use crate::templates::{
    AcademyResearchQueueItemView, BuildingQueueItemView, ServerTime, SmithyQueueItemView,
    UnitTrainingQueueItemView,
};

/// Formats a duration in seconds to HH:MM:SS.
pub fn format_duration(total_seconds: u32) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Returns the CSS class for a resource slot based on the building placed in it.
pub fn resource_css_class(slot: Option<&VillageBuilding>) -> &'static str {
    match slot.map(|vb| &vb.building.name) {
        Some(BuildingName::Woodcutter) => "wood",
        Some(BuildingName::ClayPit) => "clay",
        Some(BuildingName::IronMine) => "iron",
        Some(BuildingName::Cropland) => "crop",
        _ => "wood",
    }
}

/// Helper to get the level of a resource field, falling back to 0 if empty.
pub fn building_level(slot: Option<&VillageBuilding>) -> u8 {
    slot.map(|vb| vb.building.level).unwrap_or(0)
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

/// Converts academy research jobs into view models with countdown timers.
pub fn academy_queue_to_views(items: &[AcademyQueueItem]) -> Vec<AcademyResearchQueueItemView> {
    let now = Utc::now();
    items
        .iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            AcademyResearchQueueItemView {
                job_id: item.job_id,
                unit_name: unit_display_name(&item.unit),
                time_remaining: format_duration(remaining),
                time_seconds: remaining,
                is_processing: matches!(item.status, JobStatus::Processing),
            }
        })
        .collect()
}

/// Converts smithy upgrade jobs into view models with countdown timers and target levels.
pub fn smithy_queue_to_views(
    village: &Village,
    items: &[SmithyQueueItem],
) -> Vec<SmithyQueueItemView> {
    let now = Utc::now();
    let mut base_levels = smithy_levels_for_village(village);
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|item| item.finishes_at);

    sorted
        .into_iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            let entry = base_levels.entry(item.unit.clone()).or_insert(0);
            let target_level = (*entry + 1).min(20);
            *entry = target_level;

            SmithyQueueItemView {
                job_id: item.job_id,
                unit_name: unit_display_name(&item.unit),
                target_level,
                time_remaining: format_duration(remaining),
                time_seconds: remaining,
                is_processing: matches!(item.status, JobStatus::Processing),
            }
        })
        .collect()
}

fn smithy_levels_for_village(village: &Village) -> HashMap<UnitName, u8> {
    let mut levels = HashMap::new();
    let smithy = village.smithy();

    for (idx, unit) in village.tribe.units().iter().enumerate() {
        if idx >= smithy.len() {
            break;
        }

        levels.insert(unit.name.clone(), smithy[idx]);
    }

    levels
}

/// Returns the current server time information for the UI.
pub fn server_time() -> ServerTime {
    let now = Utc::now();
    ServerTime {
        formatted: now.format("%H:%M:%S").to_string(),
        timestamp: now.timestamp(),
    }
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

/// Returns the localized description for a building.
pub fn building_description(name: &BuildingName) -> String {
    match name {
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
        BuildingName::Smithy => t!("game.buildings.blacksmith.description"),
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
        _ => return "Description not available.".to_string(),
    }
    .to_string()
}
