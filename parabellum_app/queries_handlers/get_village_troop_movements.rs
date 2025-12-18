use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use parabellum_game::models::army::Army;
use parabellum_types::army::TroopSet;
use parabellum_types::battle::AttackType;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
    map::Position,
    tribe::Tribe,
};
use uuid::Uuid;

use crate::{
    config::Config,
    cqrs::{
        QueryHandler,
        queries::{
            GetVillageTroopMovements, TroopMovement, TroopMovementDirection, TroopMovementType,
            VillageTroopMovements,
        },
    },
    jobs::{
        Job,
        tasks::{ArmyReturnTask, AttackTask, ReinforcementTask, ScoutTask},
    },
    repository::{ArmyRepository, VillageRepository},
    uow::UnitOfWork,
};

pub struct GetVillageTroopMovementsHandler;

impl GetVillageTroopMovementsHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetVillageTroopMovements> for GetVillageTroopMovementsHandler {
    async fn handle(
        &self,
        query: GetVillageTroopMovements,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<Config>,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        let job_repo = uow.jobs();
        let outgoing_jobs = job_repo
            .list_active_jobs_by_village(query.village_id as i32)
            .await?;
        let incoming_jobs = job_repo
            .list_village_targeting_movements(query.village_id as i32)
            .await?;

        let village_repo = uow.villages();
        let army_repo = uow.armies();
        let mut village_cache = HashMap::new();
        let mut army_cache = HashMap::new();

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();

        for job in &outgoing_jobs {
            match job.task.task_type.as_str() {
                "Attack" => {
                    if let Some(movement) = attack_movement(
                        job,
                        TroopMovementDirection::Outgoing,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        outgoing.push(movement);
                    }
                }
                "Reinforcement" => {
                    if let Some(movement) = reinforcement_movement(
                        job,
                        TroopMovementDirection::Outgoing,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        outgoing.push(movement);
                    }
                }
                "Scout" => {
                    if let Some(movement) = scout_movement(
                        job,
                        TroopMovementDirection::Outgoing,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        outgoing.push(movement);
                    }
                }
                "ArmyReturn" => {
                    if let Some(movement) = return_movement(
                        job,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        incoming.push(movement);
                    }
                }
                _ => {}
            }
        }

        for job in &incoming_jobs {
            match job.task.task_type.as_str() {
                "Attack" => {
                    if let Some(movement) = attack_movement(
                        job,
                        TroopMovementDirection::Incoming,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        incoming.push(movement);
                    }
                }
                "Reinforcement" => {
                    if let Some(movement) = reinforcement_movement(
                        job,
                        TroopMovementDirection::Incoming,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        incoming.push(movement);
                    }
                }
                "Scout" => {
                    if let Some(movement) = scout_movement(
                        job,
                        TroopMovementDirection::Incoming,
                        &village_repo,
                        &army_repo,
                        &mut village_cache,
                        &mut army_cache,
                    )
                    .await?
                    {
                        incoming.push(movement);
                    }
                }
                _ => {}
            }
        }

        outgoing.sort_by_key(|movement| movement.arrives_at);
        incoming.sort_by_key(|movement| movement.arrives_at);

        Ok(VillageTroopMovements { outgoing, incoming })
    }
}

#[derive(Clone)]
struct VillageSnapshot {
    id: u32,
    name: String,
    player_id: Uuid,
    position: Position,
}

async fn snapshot_for(
    repo: &Arc<dyn VillageRepository + '_>,
    cache: &mut HashMap<u32, VillageSnapshot>,
    village_id: u32,
) -> Result<VillageSnapshot, ApplicationError> {
    if let Some(snapshot) = cache.get(&village_id) {
        return Ok(snapshot.clone());
    }

    let village = repo.get_by_id(village_id).await?;
    let snapshot = VillageSnapshot {
        id: village.id,
        name: village.name,
        player_id: village.player_id,
        position: village.position,
    };
    cache.insert(village_id, snapshot.clone());
    Ok(snapshot)
}

async fn army_for(
    repo: &Arc<dyn ArmyRepository + '_>,
    cache: &mut HashMap<Uuid, Army>,
    army_id: Uuid,
) -> Result<Option<Army>, ApplicationError> {
    if let Some(army) = cache.get(&army_id) {
        return Ok(Some(army.clone()));
    }

    match repo.get_by_id(army_id).await {
        Ok(army) => {
            cache.insert(army_id, army.clone());
            Ok(Some(army))
        }
        Err(ApplicationError::Db(DbError::ArmyNotFound(_))) => Ok(None),
        Err(err) => Err(err),
    }
}

async fn attack_movement(
    job: &Job,
    direction: TroopMovementDirection,
    village_repo: &Arc<dyn VillageRepository + '_>,
    army_repo: &Arc<dyn ArmyRepository + '_>,
    village_cache: &mut HashMap<u32, VillageSnapshot>,
    army_cache: &mut HashMap<Uuid, Army>,
) -> Result<Option<TroopMovement>, ApplicationError> {
    let payload: AttackTask = match serde_json::from_value(job.task.data.clone()) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };

    let origin_id = payload.attacker_village_id as u32;
    let target_id = payload.target_village_id as u32;

    let origin = snapshot_for(village_repo, village_cache, origin_id).await?;
    let target = snapshot_for(village_repo, village_cache, target_id).await?;
    let Some(army) = army_for(army_repo, army_cache, payload.army_id).await? else {
        tracing::warn!(
            job_id = %job.id,
            army_id = %payload.army_id,
            "Skipping attack movement because deployed army record is missing"
        );
        return Ok(None);
    };

    let movement_type = match payload.attack_type {
        AttackType::Raid => TroopMovementType::Raid,
        AttackType::Normal => TroopMovementType::Attack,
    };

    Ok(Some(build_movement(
        job,
        movement_type,
        direction,
        origin,
        target,
        army.units().clone(),
        army.tribe.clone(),
    )))
}

async fn scout_movement(
    job: &Job,
    direction: TroopMovementDirection,
    village_repo: &Arc<dyn VillageRepository + '_>,
    army_repo: &Arc<dyn ArmyRepository + '_>,
    village_cache: &mut HashMap<u32, VillageSnapshot>,
    army_cache: &mut HashMap<Uuid, Army>,
) -> Result<Option<TroopMovement>, ApplicationError> {
    let payload: ScoutTask = match serde_json::from_value(job.task.data.clone()) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };

    let origin_id = payload.attacker_village_id as u32;
    let target_id = payload.target_village_id as u32;

    let origin = snapshot_for(village_repo, village_cache, origin_id).await?;
    let target = snapshot_for(village_repo, village_cache, target_id).await?;
    let Some(army) = army_for(army_repo, army_cache, payload.army_id).await? else {
        tracing::warn!(
            job_id = %job.id,
            army_id = %payload.army_id,
            "Skipping scout movement because deployed army record is missing"
        );
        return Ok(None);
    };

    let movement_type = match payload.attack_type {
        AttackType::Raid => TroopMovementType::Raid,
        AttackType::Normal => TroopMovementType::Attack,
    };

    Ok(Some(build_movement(
        job,
        movement_type,
        direction,
        origin,
        target,
        army.units().clone(),
        army.tribe.clone(),
    )))
}

async fn reinforcement_movement(
    job: &Job,
    direction: TroopMovementDirection,
    village_repo: &Arc<dyn VillageRepository + '_>,
    army_repo: &Arc<dyn ArmyRepository + '_>,
    village_cache: &mut HashMap<u32, VillageSnapshot>,
    army_cache: &mut HashMap<Uuid, Army>,
) -> Result<Option<TroopMovement>, ApplicationError> {
    let payload: ReinforcementTask = match serde_json::from_value(job.task.data.clone()) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };

    let origin_id = job.village_id as u32;
    let target_id = payload.village_id as u32;
    let origin = snapshot_for(village_repo, village_cache, origin_id).await?;
    let target = snapshot_for(village_repo, village_cache, target_id).await?;
    let Some(army) = army_for(army_repo, army_cache, payload.army_id).await? else {
        tracing::warn!(
            job_id = %job.id,
            army_id = %payload.army_id,
            "Skipping reinforcement movement because deployed army record is missing"
        );
        return Ok(None);
    };

    Ok(Some(build_movement(
        job,
        TroopMovementType::Reinforcement,
        direction,
        origin,
        target,
        army.units().clone(),
        army.tribe.clone(),
    )))
}

async fn return_movement(
    job: &Job,
    village_repo: &Arc<dyn VillageRepository + '_>,
    army_repo: &Arc<dyn ArmyRepository + '_>,
    village_cache: &mut HashMap<u32, VillageSnapshot>,
    army_cache: &mut HashMap<Uuid, Army>,
) -> Result<Option<TroopMovement>, ApplicationError> {
    let payload: ArmyReturnTask = match serde_json::from_value(job.task.data.clone()) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };

    let origin = snapshot_for(village_repo, village_cache, payload.from_village_id as u32).await?;
    let target = snapshot_for(
        village_repo,
        village_cache,
        payload.destination_village_id as u32,
    )
    .await?;
    let Some(army) = army_for(army_repo, army_cache, payload.army_id).await? else {
        tracing::warn!(
            job_id = %job.id,
            army_id = %payload.army_id,
            "Skipping return movement because deployed army record is missing"
        );
        return Ok(None);
    };

    Ok(Some(build_movement(
        job,
        TroopMovementType::Return,
        TroopMovementDirection::Incoming,
        origin,
        target,
        army.units().clone(),
        army.tribe.clone(),
    )))
}

fn build_movement(
    job: &Job,
    movement_type: TroopMovementType,
    direction: TroopMovementDirection,
    origin: VillageSnapshot,
    target: VillageSnapshot,
    units: TroopSet,
    tribe: Tribe,
) -> TroopMovement {
    let now = Utc::now();
    let remaining = (job.completed_at - now).num_seconds().max(0) as u32;

    TroopMovement {
        job_id: job.id,
        movement_type,
        direction,
        origin_village_id: origin.id,
        origin_village_name: Some(origin.name),
        origin_player_id: origin.player_id,
        origin_position: origin.position,
        target_village_id: target.id,
        target_village_name: Some(target.name),
        target_player_id: target.player_id,
        target_position: target.position,
        arrives_at: job.completed_at,
        time_seconds: remaining,
        units,
        tribe,
    }
}
