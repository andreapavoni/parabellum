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
use parabellum_game::models::{
    army::{Army, TroopSet},
    village::Village,
};
use parabellum_types::{
    Result,
    errors::{ApplicationError, GameError},
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

fn job_target_level(job: &Job) -> Option<u8> {
    match job.task.task_type.as_str() {
        "AddBuilding" => Some(1),
        "BuildingUpgrade" => serde_json::from_value::<BuildingUpgradeTask>(job.task.data.clone())
            .ok()
            .map(|payload| payload.level),
        _ => None,
    }
}

pub fn highest_target_level_for_slot(jobs: &[Job], slot_id: u8) -> Option<u8> {
    jobs.iter()
        .filter_map(|job| {
            let job_slot = job_slot_id(job)?;
            if job_slot == slot_id {
                job_target_level(job)
            } else {
                None
            }
        })
        .max()
}

pub fn completion_time_for_slot(jobs: &[Job], slot_id: u8, duration_secs: i64) -> DateTime<Utc> {
    let start_time = slot_ready_time(jobs, slot_id);
    start_time
        .checked_add_signed(Duration::seconds(duration_secs))
        .unwrap_or(start_time)
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
    if units_to_deploy.iter().sum::<u32>() == 0 && hero_id.is_none() {
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
