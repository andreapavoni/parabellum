use mini_cqrs_es::{Cqrs, CqrsError};

use crate::villages::{
    AddBuilding, CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding,
    DowngradeBuilding, FoundVillage, ReinforcementArrived, SendReinforcement, UpgradeBuilding,
};

pub struct VillageService<'a, C: Cqrs> {
    cqrs: &'a C,
}

impl<'a, C: Cqrs> VillageService<'a, C> {
    pub fn new(cqrs: &'a C) -> Self {
        Self { cqrs }
    }

    pub async fn found_village(&self, village_id: u32, command: &FoundVillage) -> Result<u32, CqrsError> {
        self.cqrs.execute::<FoundVillage>(&village_id, command).await
    }

    pub async fn send_reinforcement(
        &self,
        village_id: u32,
        command: &SendReinforcement,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<SendReinforcement>(&village_id, command).await
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

    pub async fn add_building(&self, village_id: u32, command: &AddBuilding) -> Result<u32, CqrsError> {
        self.cqrs.execute::<AddBuilding>(&village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<UpgradeBuilding>(&village_id, command).await
    }

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<DowngradeBuilding>(&village_id, command).await
    }

    pub async fn complete_add_building(
        &self,
        village_id: u32,
        command: &CompleteAddBuilding,
    ) -> Result<u32, CqrsError> {
        self.cqrs.execute::<CompleteAddBuilding>(&village_id, command).await
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
}
