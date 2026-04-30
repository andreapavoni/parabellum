use mini_cqrs_es::{Cqrs, CqrsError};

use crate::villages::{
    AddBuilding, CompleteAcademyResearch, CompleteAddBuilding, CompleteDowngradeBuilding,
    CompleteSmithyResearch, CompleteTrainUnit, CompleteUpgradeBuilding, DowngradeBuilding,
    FoundVillage, ReinforcementArrived, ResearchAcademy, ResearchSmithy, SendReinforcement,
    SetVillageResources, TrainUnits, UpgradeBuilding,
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

    pub async fn reinforcement_arrived(
        &self,
        village_id: u32,
        command: &ReinforcementArrived,
    ) -> Result<u32, CqrsError> {
        self.cqrs
            .execute::<ReinforcementArrived>(&village_id, command)
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
