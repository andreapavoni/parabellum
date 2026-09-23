//! Army and movement read helpers for `VillageEsService`.
//!
//! These reads are backed by army, movement, and scheduled-action projections.
//! Command validation expects the same canonical army state exposed here, so the
//! methods do not read troop state from village rows.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::{
    MovementDirection, MovementType, ScheduledActionPayload, VillageModel, VillageMovement,
};
use parabellum_app::villages::projection_repositories::{
    ArmyRepository, VillageMovementFilter, VillageMovementRepository, VillageRepository,
};
use parabellum_app::villages::read_models::{
    TroopMovement, TroopMovementDirection, TroopMovementType, VillageArmyStateView,
    VillageTroopMovements,
};
use parabellum_types::errors::DbError;

use crate::es::{
    PostgresArmyRepository, PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository,
};

use super::super::{
    CancelTroopMovementContext, ReinforcementContext, TrappedArmyContext, VillageEsService,
};

fn troop_movement_type(movement_type: MovementType) -> TroopMovementType {
    match movement_type {
        MovementType::Attack => TroopMovementType::Attack,
        MovementType::Raid => TroopMovementType::Raid,
        MovementType::Scout => TroopMovementType::Scout,
        MovementType::Reinforcement => TroopMovementType::Reinforcement,
        MovementType::Return => TroopMovementType::Return,
        MovementType::FoundVillage => TroopMovementType::FoundVillage,
    }
}

fn troop_movement_direction(direction: MovementDirection) -> TroopMovementDirection {
    match direction {
        MovementDirection::Incoming => TroopMovementDirection::Incoming,
        MovementDirection::Outgoing => TroopMovementDirection::Outgoing,
    }
}

fn movement_fallback_village_ids(movements: &[VillageMovement]) -> Vec<u32> {
    movements
        .iter()
        .flat_map(|movement| {
            [
                needs_origin_fallback(movement).then_some(movement.origin_village_id),
                needs_target_fallback(movement).then_some(movement.target_village_id),
            ]
        })
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn needs_origin_fallback(movement: &VillageMovement) -> bool {
    movement.origin_village_name.is_none()
        || movement.origin_position.is_none()
        || (movement.tribe.is_none() && movement.movement_type != MovementType::Return)
}

fn needs_target_fallback(movement: &VillageMovement) -> bool {
    movement.target_village_name.is_none()
        || movement.target_player_id.is_none()
        || movement.target_position.is_none()
        || (movement.tribe.is_none() && movement.movement_type == MovementType::Return)
}

fn troop_movement_from_projection(
    movement: VillageMovement,
    fallback_villages: &HashMap<u32, VillageModel>,
) -> TroopMovement {
    let origin_model = fallback_villages.get(&movement.origin_village_id);
    let target_model = fallback_villages.get(&movement.target_village_id);

    let tribe = movement
        .tribe
        .or_else(|| {
            if movement.movement_type == MovementType::Return {
                target_model.map(|village| village.tribe.clone())
            } else {
                origin_model.map(|village| village.tribe.clone())
            }
        })
        .unwrap_or(parabellum_types::tribe::Tribe::Nature);

    TroopMovement {
        job_id: movement.movement_id,
        movement_type: troop_movement_type(movement.movement_type),
        direction: troop_movement_direction(movement.direction),
        origin_village_id: movement.origin_village_id,
        origin_village_name: movement
            .origin_village_name
            .or_else(|| origin_model.map(|village| village.village_name.clone())),
        origin_player_id: movement.origin_player_id,
        origin_position: movement
            .origin_position
            .or_else(|| origin_model.map(|village| village.position.clone()))
            .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
        target_village_id: movement.target_village_id,
        target_village_name: movement
            .target_village_name
            .or_else(|| target_model.map(|village| village.village_name.clone())),
        target_player_id: movement
            .target_player_id
            .or_else(|| target_model.map(|village| village.player_id))
            .unwrap_or(movement.origin_player_id),
        target_position: movement
            .target_position
            .or_else(|| target_model.map(|village| village.position.clone()))
            .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
        arrives_at: movement.arrives_at,
        time_seconds: movement.time_seconds.unwrap_or(0),
        units: movement.units,
        has_hero: movement.has_hero,
        tribe,
        bounty: movement.bounty,
    }
}

impl VillageEsService {
    /// Returns movement ids that can still be canceled from the source village.
    pub async fn list_cancelable_outgoing_movement_ids(
        &self,
        source_village_id: u32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<HashSet<uuid::Uuid>, CqrsError> {
        let rows =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()))
                .list_pending_troop_arrivals_by_source_village(source_village_id)
                .await
                .map_err(CqrsError::domain_source)?;
        let mut movement_ids = HashSet::new();
        for row in rows {
            let cancel_deadline = row.created_at() + chrono::Duration::seconds(60);
            if now > cancel_deadline || now >= row.execute_at() {
                continue;
            }
            movement_ids.insert(row.movement_id().map_err(CqrsError::domain_source)?);
        }
        Ok(movement_ids)
    }

    /// Returns incoming and outgoing troop movements visible from a village.
    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, CqrsError> {
        let repo =
            PostgresVillageMovementRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let movements = repo
            .list_movements(VillageMovementFilter::for_village(village_id))
            .await
            .map_err(CqrsError::domain_source)?;
        let fallback_villages = self.load_movement_fallback_villages(&movements).await?;

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for movement in movements {
            let mapped = troop_movement_from_projection(movement, &fallback_villages);
            match mapped.direction {
                TroopMovementDirection::Outgoing => outgoing.push(mapped),
                TroopMovementDirection::Incoming => incoming.push(mapped),
            };
        }
        outgoing.sort_by_key(|movement| movement.arrives_at);
        incoming.sort_by_key(|movement| movement.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    async fn load_movement_fallback_villages(
        &self,
        movements: &[VillageMovement],
    ) -> Result<HashMap<u32, VillageModel>, CqrsError> {
        let fallback_village_ids = movement_fallback_village_ids(movements);
        let village_repo =
            PostgresVillageRepository::new(crate::ProjectionDb::new(self.pool.clone()));

        village_repo
            .list_by_village_ids(&fallback_village_ids)
            .await
            .map_err(CqrsError::domain_source)
            .map(|villages| {
                villages
                    .into_iter()
                    .map(|village| (village.village_id, village))
                    .collect()
            })
    }

    /// Returns the stationed reinforcement context needed by recall/release commands.
    pub async fn find_reinforcement_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<ReinforcementContext, CqrsError> {
        let army_repo: Arc<dyn ArmyRepository> = Arc::new(PostgresArmyRepository::new(
            crate::ProjectionDb::new(self.pool.clone()),
        ));
        if let Some((stationed_village_id, army)) = army_repo
            .find_stationed_context_by_army_id(army_id)
            .await
            .map_err(CqrsError::domain_source)?
        {
            return Ok(ReinforcementContext {
                stationed_village_id,
                home_village_id: army.village_id,
                army,
            });
        }

        Err(CqrsError::EventStore(
            DbError::ArmyNotFound(army_id).to_string(),
        ))
    }

    /// Returns the trapped army context needed by release/disband commands.
    pub async fn find_trapped_army_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<TrappedArmyContext, CqrsError> {
        let army_repo: Arc<dyn ArmyRepository> = Arc::new(PostgresArmyRepository::new(
            crate::ProjectionDb::new(self.pool.clone()),
        ));
        if let Some((trapped_village_id, army)) = army_repo
            .find_trapped_context_by_army_id(army_id)
            .await
            .map_err(CqrsError::domain_source)?
        {
            return Ok(TrappedArmyContext {
                trapped_village_id,
                home_village_id: army.village_id,
                army,
            });
        }

        Err(CqrsError::EventStore(
            DbError::ArmyNotFound(army_id).to_string(),
        ))
    }

    /// Returns the scheduled-arrival context needed to cancel an outgoing troop movement.
    pub async fn find_cancel_troop_movement_context(
        &self,
        movement_id: uuid::Uuid,
    ) -> Result<CancelTroopMovementContext, CqrsError> {
        let repo =
            PostgresScheduledActionRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let Some(action) = repo
            .find_pending_troop_arrival_by_movement_id(movement_id)
            .await
            .map_err(CqrsError::domain_source)?
        else {
            return Err(CqrsError::EventStore(
                DbError::JobNotFound(movement_id).to_string(),
            ));
        };

        match action.payload().map_err(CqrsError::domain_source)? {
            ScheduledActionPayload::AttackArrival { workflow } => Ok(CancelTroopMovementContext {
                movement_id,
                arrival_action_id: action.id(),
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army,
                sent_at: action.created_at(),
                arrives_at: workflow.arrives_at,
            }),
            ScheduledActionPayload::ScoutArrival { workflow } => Ok(CancelTroopMovementContext {
                movement_id,
                arrival_action_id: action.id(),
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army,
                sent_at: action.created_at(),
                arrives_at: workflow.arrives_at,
            }),
            ScheduledActionPayload::ReinforcementArrival { workflow } => {
                Ok(CancelTroopMovementContext {
                    movement_id,
                    arrival_action_id: action.id(),
                    army_id: workflow.army_id,
                    player_id: workflow.player_id,
                    source_village_id: workflow.source_village_id,
                    target_village_id: workflow.target_village_id,
                    army: workflow.army,
                    sent_at: action.created_at(),
                    arrives_at: workflow.arrives_at,
                })
            }
            ScheduledActionPayload::SettlersArrival { workflow } => {
                let army_repo: Arc<dyn ArmyRepository> = Arc::new(PostgresArmyRepository::new(
                    crate::ProjectionDb::new(self.pool.clone()),
                ));
                let army = army_repo
                    .get_moving_army(workflow.army_id)
                    .await
                    .map_err(CqrsError::domain_source)?;
                Ok(CancelTroopMovementContext {
                    movement_id,
                    arrival_action_id: action.id(),
                    army_id: workflow.army_id,
                    player_id: workflow.player_id,
                    source_village_id: workflow.source_village_id,
                    target_village_id: workflow.target_village_id,
                    army,
                    sent_at: action.created_at(),
                    arrives_at: workflow.arrives_at,
                })
            }
            _ => Err(CqrsError::EventStore(
                "Scheduled action is not a troop arrival workflow".to_string(),
            )),
        }
    }

    /// Returns the canonical projected army state for a village.
    pub async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, CqrsError> {
        let repo = PostgresArmyRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        let armies = repo
            .army_context_for_village(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        Ok(VillageArmyStateView {
            home_army: armies.home,
            reinforcements: armies.stationed,
            deployed_armies: armies.deployed,
            trapped_here: armies.trapped_here,
            trapped_away: armies.trapped_away,
        })
    }
}
