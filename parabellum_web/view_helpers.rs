use chrono::Utc;
use parabellum_app::{
    cqrs::queries::{BuildingQueueItem, TrainingQueueItem},
    jobs::JobStatus,
};
use parabellum_types::{army::UnitName, buildings::BuildingName, common::ResourceGroup};
use rust_i18n::t;
use uuid::Uuid;

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
