use mini_cqrs_es::{Cqrs, CqrsError};

use crate::villages::{
    AcceptMarketplaceOffer, AddBuilding, AttackVillage, CancelMarketplaceOffer,
    CompleteAcademyResearch, CompleteAddBuilding, CompleteAttackArrival, CompleteAttackReturn,
    CompleteDowngradeBuilding, CompleteMerchantsArrival, CompleteMerchantsReturn,
    CompleteReinforcementsReturn, CompleteScoutArrival, CompleteScoutReturn,
    CompleteSettlersArrival, CompleteSmithyResearch, CompleteTrainUnit, CompleteUpgradeBuilding,
    ConquerVillage, CreateMarketplaceOffer, DowngradeBuilding, FoundVillage, RecallReinforcements,
    ReinforcementArrived, ReleaseReinforcements, ResearchAcademy, ResearchSmithy, ScoutVillage,
    SendMerchantsTransfer, SendReinforcement, SendSettlers, SetVillageResources, TrainUnits,
    UpgradeBuilding,
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

    pub async fn reinforcement_arrived(
        &self,
        village_id: u32,
        command: &ReinforcementArrived,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ReinforcementArrived>(&village_id, command)
            .await
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

    pub async fn complete_reinforcements_return(
        &self,
        village_id: u32,
        command: &CompleteReinforcementsReturn,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteReinforcementsReturn>(&village_id, command)
            .await
    }

    pub async fn complete_attack_arrival(
        &self,
        village_id: u32,
        command: &CompleteAttackArrival,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteAttackArrival>(&village_id, command)
            .await
    }

    pub async fn complete_attack_return(
        &self,
        village_id: u32,
        command: &CompleteAttackReturn,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteAttackReturn>(&village_id, command)
            .await
    }

    pub async fn complete_scout_arrival(
        &self,
        village_id: u32,
        command: &CompleteScoutArrival,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteScoutArrival>(&village_id, command)
            .await
    }

    pub async fn complete_scout_return(
        &self,
        village_id: u32,
        command: &CompleteScoutReturn,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteScoutReturn>(&village_id, command)
            .await
    }

    pub async fn complete_settlers_arrival(
        &self,
        village_id: u32,
        command: &CompleteSettlersArrival,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteSettlersArrival>(&village_id, command)
            .await
    }

    pub async fn conquer_village(
        &self,
        village_id: u32,
        command: &ConquerVillage,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ConquerVillage>(&village_id, command)
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

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<DowngradeBuilding>(&village_id, command)
            .await
    }

    pub async fn complete_add_building(
        &self,
        village_id: u32,
        command: &CompleteAddBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteAddBuilding>(&village_id, command)
            .await
    }

    pub async fn complete_upgrade_building(
        &self,
        village_id: u32,
        command: &CompleteUpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteUpgradeBuilding>(&village_id, command)
            .await
    }

    pub async fn complete_downgrade_building(
        &self,
        village_id: u32,
        command: &CompleteDowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteDowngradeBuilding>(&village_id, command)
            .await
    }

    pub async fn complete_merchant_arrival(
        &self,
        village_id: u32,
        command: &CompleteMerchantsArrival,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute(&village_id, command).await
    }

    pub async fn complete_merchant_return(
        &self,
        village_id: u32,
        command: &CompleteMerchantsReturn,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute(&village_id, command).await
    }

    pub async fn train_units(
        &self,
        village_id: u32,
        command: &TrainUnits,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<TrainUnits>(&village_id, command).await
    }

    pub async fn complete_train_unit(
        &self,
        village_id: u32,
        command: &CompleteTrainUnit,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteTrainUnit>(&village_id, command)
            .await
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

    pub async fn complete_academy_research(
        &self,
        village_id: u32,
        command: &CompleteAcademyResearch,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteAcademyResearch>(&village_id, command)
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

    pub async fn complete_smithy_research(
        &self,
        village_id: u32,
        command: &CompleteSmithyResearch,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<CompleteSmithyResearch>(&village_id, command)
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
}
