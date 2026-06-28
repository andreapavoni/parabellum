//! Direct CQRS command dispatch helpers for `VillageEsService`.

use mini_cqrs_es::CqrsError;
use mini_cqrs_es::anyhow::Result;
use parabellum_app::map::MapReadPort;
use parabellum_app::villages::{
    AddBuilding, ApplyBattleOutcomeToVillage, AssignHeroPoints, AttackVillage, BuildTraps,
    CancelBuildingConstruction, CreateHero, DisbandTrappedTroops, DowngradeBuilding, FoundVillage,
    MarkReportRead, RecallReinforcements, ReleaseReinforcements, ReleaseTrappedTroops,
    RenameVillage, ResearchAcademy, ResearchSmithy, ResetHeroPoints, ResolveAttackBattle,
    ReviveHero, ScoutVillage, SendReinforcement, SendSettlers, SetHeroResourceFocus,
    SetVillageResources, TrainUnits, UpgradeBuilding, VillageService,
};
use parabellum_types::errors::GameError;

use crate::es::village_cqrs_runtime;
use crate::map::PostgresMapRepository;

use super::VillageEsService;

impl VillageEsService {
    pub async fn found_village(
        &self,
        village_id: u32,
        command: &FoundVillage,
    ) -> Result<(), CqrsError> {
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

    pub async fn send_attack(
        &self,
        village_id: u32,
        command: &AttackVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_attack(village_id, command).await
    }

    pub async fn recall_reinforcements(
        &self,
        village_id: u32,
        command: &RecallReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.recall_reinforcements(village_id, command).await
    }

    pub async fn release_reinforcements(
        &self,
        village_id: u32,
        command: &ReleaseReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.release_reinforcements(village_id, command).await
    }

    pub async fn release_trapped_troops(
        &self,
        village_id: u32,
        command: &ReleaseTrappedTroops,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.release_trapped_troops(village_id, command).await
    }

    pub async fn disband_trapped_troops(
        &self,
        village_id: u32,
        command: &DisbandTrappedTroops,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.disband_trapped_troops(village_id, command).await
    }

    pub async fn cancel_troop_movement(
        &self,
        village_id: u32,
        command: &parabellum_app::villages::CancelTroopMovement,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.cancel_troop_movement(village_id, command).await
    }

    pub async fn mark_report_read(
        &self,
        village_id: u32,
        command: &MarkReportRead,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.mark_report_read(village_id, command).await
    }

    pub async fn send_scout(
        &self,
        village_id: u32,
        command: &ScoutVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_scout(village_id, command).await
    }

    pub async fn send_settlers(
        &self,
        village_id: u32,
        command: &SendSettlers,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_settlers(village_id, command).await
    }

    pub async fn create_hero(
        &self,
        village_id: u32,
        command: &CreateHero,
    ) -> Result<u32, CqrsError> {
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::domain_source(GameError::HeroAlreadyExists));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_hero(village_id, command).await
    }

    pub async fn revive_hero(
        &self,
        village_id: u32,
        command: &ReviveHero,
    ) -> Result<u32, CqrsError> {
        if self
            .pending_hero_revival_at(command.player_id)
            .await?
            .is_some()
        {
            return Err(CqrsError::domain_source(
                GameError::HeroRevivalAlreadyPending,
            ));
        }
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::domain_source(GameError::HeroAlreadyExists));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.revive_hero(village_id, command).await
    }

    pub async fn assign_hero_points(
        &self,
        village_id: u32,
        command: &AssignHeroPoints,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.assign_hero_points(village_id, command).await
    }

    pub async fn reset_hero_points(
        &self,
        village_id: u32,
        command: &ResetHeroPoints,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.reset_hero_points(village_id, command).await
    }

    pub async fn set_hero_resource_focus(
        &self,
        village_id: u32,
        command: &SetHeroResourceFocus,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.set_hero_resource_focus(village_id, command).await
    }

    /// Returns whether a target map field is currently an unoccupied valley.
    pub async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, CqrsError> {
        let map_repo = PostgresMapRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        map_repo
            .is_unoccupied_valley(field_id as i32)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn add_building(
        &self,
        village_id: u32,
        command: &AddBuilding,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.add_building(village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.upgrade_building(village_id, command).await
    }

    pub async fn cancel_building_construction(
        &self,
        village_id: u32,
        command: &CancelBuildingConstruction,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .cancel_building_construction(village_id, command)
            .await
    }

    /// Renames a village for its owner.
    pub async fn rename_village(
        &self,
        village_id: u32,
        command: &RenameVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.rename_village(village_id, command).await
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

    pub async fn train_units(
        &self,
        village_id: u32,
        command: &TrainUnits,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.train_units(village_id, command).await
    }

    pub async fn build_traps(
        &self,
        village_id: u32,
        command: &BuildTraps,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.build_traps(village_id, command).await
    }

    pub async fn research_academy(
        &self,
        village_id: u32,
        command: &ResearchAcademy,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_academy(village_id, command).await
    }

    pub async fn research_smithy(
        &self,
        village_id: u32,
        command: &ResearchSmithy,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_smithy(village_id, command).await
    }

    pub async fn resolve_attack_battle(
        &self,
        village_id: u32,
        command: &ResolveAttackBattle,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.resolve_attack_battle(village_id, command).await
    }

    pub async fn apply_battle_outcome_to_village(
        &self,
        village_id: u32,
        command: &ApplyBattleOutcomeToVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .apply_battle_outcome_to_village(village_id, command)
            .await
    }

    /// Executes the village resource utility command through the ES runtime.
    pub async fn set_village_resources(
        &self,
        village_id: u32,
        command: &SetVillageResources,
    ) -> Result<(), CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.set_village_resources(village_id, command).await
    }
}
