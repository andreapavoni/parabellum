use mini_cqrs_es::CqrsError;
use sqlx::PgPool;

use parabellum_app::villages::models::{
    ScheduledActionPayload, ScheduledActionStatus, VillageModel, VillageTroopMovements,
};
use parabellum_app::villages::repositories::{
    ScheduledActionRepository, VillageModelRepository, VillageMovementRepository,
};
use parabellum_app::villages::{
    AddBuilding, CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding,
    DowngradeBuilding, FoundVillage, ReinforcementArrived, SendReinforcement, UpgradeBuilding,
    VillageService,
};

use crate::es::{
    PostgresScheduledActionRepository, PostgresVillageModelRepository,
    PostgresVillageMovementRepository, village_cqrs_runtime,
};

#[derive(Debug, Clone)]
pub struct VillageEsService {
    pool: PgPool,
}

impl VillageEsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn found_village(
        &self,
        village_id: u32,
        command: &FoundVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.found_village(village_id, command).await
    }

    pub async fn send_reinforcement(
        &self,
        village_id: u32,
        command: &SendReinforcement,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_reinforcement(village_id, command).await
    }

    pub async fn add_building(
        &self,
        village_id: u32,
        command: &AddBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.add_building(village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.upgrade_building(village_id, command).await
    }

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.downgrade_building(village_id, command).await
    }

    pub async fn complete_add_building(
        &self,
        village_id: u32,
        command: &CompleteAddBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_add_building(village_id, command).await
    }

    pub async fn complete_upgrade_building(
        &self,
        village_id: u32,
        command: &CompleteUpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_upgrade_building(village_id, command).await
    }

    pub async fn complete_downgrade_building(
        &self,
        village_id: u32,
        command: &CompleteDowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_downgrade_building(village_id, command).await
    }

    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        let actions = PostgresScheduledActionRepository::new(self.pool.clone())
            .take_due_pending(before_or_equal, limit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let mut processed = 0usize;
        for action in actions {
            let result = self.execute_action(&service, &action).await;
            let next_status = if result.is_ok() {
                ScheduledActionStatus::Completed
            } else {
                ScheduledActionStatus::Failed
            };
            repo.update_status(action.id, next_status)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            result?;
            processed += 1;
        }
        Ok(processed)
    }

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, CqrsError> {
        let repo = PostgresVillageMovementRepository::new(self.pool.clone());
        let movements = repo
            .list_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for movement in movements {
            match movement.direction {
                parabellum_app::villages::models::MovementDirection::Outgoing => outgoing.push(movement),
                parabellum_app::villages::models::MovementDirection::Incoming => incoming.push(movement),
            }
        }
        outgoing.sort_by_key(|m| m.arrives_at);
        incoming.sort_by_key(|m| m.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    pub async fn get_village_model(&self, village_id: u32) -> Result<VillageModel, CqrsError> {
        let repo = PostgresVillageModelRepository::new(self.pool.clone());
        repo.get_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_village_models_by_player_id(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageModelRepository::new(self.pool.clone());
        repo.list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn execute_action(
        &self,
        service: &VillageService<'_, crate::es::VillageCqrsRuntime>,
        action: &parabellum_app::villages::models::ScheduledAction,
    ) -> Result<(), CqrsError> {
        let payload: ScheduledActionPayload =
            serde_json::from_value(action.payload.clone()).map_err(CqrsError::Serialization)?;
        match payload {
            ScheduledActionPayload::ReinforcementArrival {
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                units,
                hero_id,
                arrives_at,
            } => {
                let command = ReinforcementArrived {
                    movement_id,
                    army_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    units,
                    hero_id,
                    arrives_at,
                };
                service
                    .reinforcement_arrived(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::AddBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteAddBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service.complete_add_building(village_id, &command).await?;
            }
            ScheduledActionPayload::UpgradeBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteUpgradeBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service.complete_upgrade_building(village_id, &command).await?;
            }
            ScheduledActionPayload::DowngradeBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteDowngradeBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service.complete_downgrade_building(village_id, &command).await?;
            }
        }
        Ok(())
    }
}
