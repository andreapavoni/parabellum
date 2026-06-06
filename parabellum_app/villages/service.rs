use mini_cqrs_es::{Cqrs, CqrsError};

use crate::villages::{
    AcceptMarketplaceOffer, AddBuilding, ApplyBattleOutcomeToVillage, AttackVillage,
    CancelMarketplaceOffer, CreateHero, CreateMarketplaceOffer, DowngradeBuilding, FoundVillage,
    MarkReportRead, RecallReinforcements, ReleaseReinforcements, RenameVillage, ResearchAcademy,
    ResearchSmithy, ResolveAttackBattle, ReviveHero, ScoutVillage, SendMerchantsTransfer,
    SendReinforcement, SendSettlers, SetVillageResources, TrainUnits, UpgradeBuilding,
};

pub struct VillageService<'a, C: Cqrs> {
    cqrs: &'a C,
}

impl<'a, C: Cqrs> VillageService<'a, C> {
    pub fn new(cqrs: &'a C) -> Self {
        Self { cqrs }
    }

    pub async fn found_village(
        &self,
        village_id: u32,
        command: &FoundVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<FoundVillage>(&village_id, command)
            .await
    }

    pub async fn send_reinforcement(
        &self,
        village_id: u32,
        command: &SendReinforcement,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<SendReinforcement>(&village_id, command)
            .await
    }

    pub async fn send_attack(
        &self,
        village_id: u32,
        command: &AttackVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<AttackVillage>(&village_id, command)
            .await
    }

    pub async fn send_scout(
        &self,
        village_id: u32,
        command: &ScoutVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ScoutVillage>(&village_id, command)
            .await
    }

    pub async fn send_settlers(
        &self,
        village_id: u32,
        command: &SendSettlers,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<SendSettlers>(&village_id, command)
            .await
    }

    pub async fn send_resources(
        &self,
        village_id: u32,
        command: &SendMerchantsTransfer,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute(&village_id, command).await
    }

    pub async fn create_hero(
        &self,
        village_id: u32,
        command: &CreateHero,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<CreateHero>(&village_id, command).await
    }

    pub async fn revive_hero(
        &self,
        village_id: u32,
        command: &ReviveHero,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<ReviveHero>(&village_id, command).await
    }

    pub async fn recall_reinforcements(
        &self,
        village_id: u32,
        command: &RecallReinforcements,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<RecallReinforcements>(&village_id, command)
            .await
    }

    pub async fn release_reinforcements(
        &self,
        village_id: u32,
        command: &ReleaseReinforcements,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ReleaseReinforcements>(&village_id, command)
            .await
    }

    pub async fn resolve_attack_battle(
        &self,
        village_id: u32,
        command: &ResolveAttackBattle,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ResolveAttackBattle>(&village_id, command)
            .await
    }

    pub async fn apply_battle_outcome_to_village(
        &self,
        village_id: u32,
        command: &ApplyBattleOutcomeToVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ApplyBattleOutcomeToVillage>(&village_id, command)
            .await
    }

    pub async fn create_marketplace_offer(
        &self,
        village_id: u32,
        command: &CreateMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CreateMarketplaceOffer>(&village_id, command)
            .await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        village_id: u32,
        command: &CancelMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CancelMarketplaceOffer>(&village_id, command)
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        village_id: u32,
        command: &AcceptMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<AcceptMarketplaceOffer>(&village_id, command)
            .await
    }

    pub async fn add_building(
        &self,
        village_id: u32,
        command: &AddBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<AddBuilding>(&village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<UpgradeBuilding>(&village_id, command)
            .await
    }

    pub async fn rename_village(
        &self,
        village_id: u32,
        command: &RenameVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<RenameVillage>(&village_id, command)
            .await
    }

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<DowngradeBuilding>(&village_id, command)
            .await
    }

    pub async fn train_units(
        &self,
        village_id: u32,
        command: &TrainUnits,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<TrainUnits>(&village_id, command).await
    }

    pub async fn research_academy(
        &self,
        village_id: u32,
        command: &ResearchAcademy,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ResearchAcademy>(&village_id, command)
            .await
    }

    pub async fn research_smithy(
        &self,
        village_id: u32,
        command: &ResearchSmithy,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ResearchSmithy>(&village_id, command)
            .await
    }

    /// Executes `SetVillageResources` on a village aggregate.
    ///
    /// Effective stored values are clamped by current storage capacities.
    pub async fn set_village_resources(
        &self,
        village_id: u32,
        command: &SetVillageResources,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<SetVillageResources>(&village_id, command)
            .await
    }

    pub async fn mark_report_read(
        &self,
        village_id: u32,
        command: &MarkReportRead,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<MarkReportRead>(&village_id, command)
            .await
    }
}
