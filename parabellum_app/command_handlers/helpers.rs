use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    jobs::{
        Job,
        tasks::{AddBuildingTask, BuildingUpgradeTask},
    },
    repository::{ArmyRepository, HeroRepository, VillageRepository},
    uow::UnitOfWork,
};
use parabellum_game::models::{army::Army, village::Village};
use parabellum_types::{
    Result,
    army::TroopSet,
    common::ResourceGroup,
    errors::{AppError, ApplicationError, GameError},
    tribe::Tribe,
};

fn job_slot_id(job: &Job) -> Option<u8> {
    match job.task.task_type.as_str() {
        "AddBuilding" => serde_json::from_value::<AddBuildingTask>(job.task.data.clone())
            .ok()
            .map(|payload| payload.slot_id),
        "BuildingUpgrade" => serde_json::from_value::<BuildingUpgradeTask>(job.task.data.clone())
            .ok()
            .map(|payload| payload.slot_id),
        _ => None,
    }
}

fn slot_ready_time(jobs: &[Job], slot_id: u8) -> DateTime<Utc> {
    let now = Utc::now();
    let latest = jobs
        .iter()
        .filter_map(|job| {
            let target_slot = job_slot_id(job)?;
            if target_slot == slot_id {
                Some(job.completed_at)
            } else {
                None
            }
        })
        .max();

    match latest {
        Some(time) if time > now => time,
        _ => now,
    }
}

pub fn completion_time_for_slot(jobs: &[Job], slot_id: u8, duration_secs: i64) -> DateTime<Utc> {
    let start_time = slot_ready_time(jobs, slot_id);
    start_time
        .checked_add_signed(Duration::seconds(duration_secs))
        .unwrap_or(start_time)
}

fn queue_ready_time(jobs: &[Job]) -> DateTime<Utc> {
    let now = Utc::now();
    match jobs.iter().map(|job| job.completed_at).max() {
        Some(time) if time > now => time,
        _ => now,
    }
}

pub fn completion_time_for_queue(jobs: &[Job], duration_secs: i64) -> DateTime<Utc> {
    let start_time = queue_ready_time(jobs);
    start_time
        .checked_add_signed(Duration::seconds(duration_secs))
        .unwrap_or(start_time)
}

pub fn enforce_queue_capacity(queue_name: &'static str, jobs: &[Job], limit: usize) -> Result<()> {
    if jobs.len() >= limit {
        Err(AppError::QueueLimitReached { queue: queue_name }.into())
    } else {
        Ok(())
    }
}

pub fn building_queue_jobs(jobs: Vec<Job>) -> Vec<Job> {
    jobs.into_iter()
        .filter(|job| {
            matches!(
                job.task.task_type.as_str(),
                "AddBuilding" | "BuildingUpgrade"
            )
        })
        .collect()
}

/// Handles the logic of deploying an army from a village.
/// Returns the updated Village and the new deployed Army.
pub async fn deploy_army_from_village(
    uow: &Box<dyn UnitOfWork<'_> + '_>,
    mut village: Village, // Take ownership to modify
    home_army_id: Uuid,
    units_to_deploy: TroopSet,
    hero_id: Option<Uuid>,
) -> Result<(Village, Army), ApplicationError> {
    if units_to_deploy.immensity() == 0 && hero_id.is_none() {
        return Err(ApplicationError::Game(GameError::NoUnitsSelected));
    }
    let army_repo: Arc<dyn ArmyRepository + '_> = uow.armies();
    let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
    let hero_repo: Arc<dyn HeroRepository + '_> = uow.heroes();

    let mut home_army = army_repo.get_by_id(home_army_id).await?;
    let attacker_village = village_repo.get_by_id(village.id).await?;

    let hero = if let (Some(id), Some(home_hero)) = (hero_id, home_army.hero()) {
        let hero = hero_repo.get_by_id(id).await?;
        home_army.set_hero(None);

        if !(hero.village_id == attacker_village.id
            && hero.player_id == attacker_village.player_id
            && home_hero.id == hero.id)
        {
            return Err(ApplicationError::Game(GameError::HeroNotAtHome {
                hero_id: hero.id,
                village_id: attacker_village.id,
            }));
        }
        Some(hero)
    } else {
        None
    };

    let deployed_army = home_army.deploy(units_to_deploy, hero)?;
    if home_army.immensity() == 0 {
        army_repo.remove(home_army_id).await?;
        village.set_army(None)?;
    } else {
        army_repo.save(&home_army).await?;
        village.set_army(Some(&home_army))?;
    }

    village_repo.save(&village).await?;
    army_repo.save(&deployed_army).await?;

    Ok((village, deployed_army))
}

/// Calculates the number of merchants needed to transport the given amount of resources.
pub fn calculate_merchants_needed(tribe: &Tribe, resources_total: u32) -> Result<u8, GameError> {
    let capacity = tribe.merchant_stats().capacity;

    if capacity == 0 {
        return Err(GameError::NotEnoughMerchants);
    }

    let merchants = (resources_total as f64 / capacity as f64).ceil() as u8;
    if resources_total > 0 && merchants == 0 {
        Ok(1)
    } else {
        Ok(merchants)
    }
}

fn non_zero_resource_index(resources: &ResourceGroup) -> Option<usize> {
    let values = [
        resources.lumber(),
        resources.clay(),
        resources.iron(),
        resources.crop(),
    ];
    let mut idx = None;
    for (i, value) in values.iter().enumerate() {
        if *value > 0 {
            if idx.is_some() {
                return None;
            }
            idx = Some(i);
        }
    }
    idx
}

/// Marketplace offer rules:
/// - exactly one resource type on each side
/// - resource types must be different
/// - quantity ratio must be between 1:3 and 3:1
pub fn validate_marketplace_exchange_rules(
    offer_resources: &ResourceGroup,
    seek_resources: &ResourceGroup,
) -> Result<(), GameError> {
    if offer_resources.total() == 0 || seek_resources.total() == 0 {
        return Err(GameError::InvalidMarketplaceOffer);
    }

    let offer_idx =
        non_zero_resource_index(offer_resources).ok_or(GameError::InvalidMarketplaceOffer)?;
    let seek_idx =
        non_zero_resource_index(seek_resources).ok_or(GameError::InvalidMarketplaceOffer)?;

    if offer_idx == seek_idx {
        return Err(GameError::InvalidMarketplaceOffer);
    }

    let offer_total = offer_resources.total() as u64;
    let seek_total = seek_resources.total() as u64;
    let (max_side, min_side) = if offer_total >= seek_total {
        (offer_total, seek_total)
    } else {
        (seek_total, offer_total)
    };

    if min_side == 0 || max_side > min_side.saturating_mul(3) {
        return Err(GameError::InvalidMarketplaceOffer);
    }

    Ok(())
}
