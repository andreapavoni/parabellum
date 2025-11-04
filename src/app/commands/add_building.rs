use std::sync::Arc;

use uuid::Uuid;

use crate::{
    Result,
    config::Config,
    cqrs::{Command, CommandHandler},
    game::{
        GameError,
        models::buildings::{Building, BuildingName},
    },
    jobs::{Job, JobPayload, tasks::AddBuildingTask},
    repository::uow::UnitOfWork,
};

#[derive(Debug, Clone)]
pub struct AddBuilding {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub name: BuildingName,
}

impl Command for AddBuilding {}

pub struct AddBuildingHandler {}

impl Default for AddBuildingHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AddBuildingHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AddBuilding> for AddBuildingHandler {
    async fn handle(
        &self,
        command: AddBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let villages_repo = uow.villages();

        let mut village = villages_repo.get_by_id(command.village_id).await?;

        if village.buildings.len() == 40 {
            return Err(GameError::VillageSlotsFull.into());
        }
        if village.get_building_by_slot_id(command.slot_id).is_some() {
            return Err(GameError::SlotOccupied {
                slot_id: command.slot_id,
            }
            .into());
        }

        let building = Building::new(command.name.clone());

        Building::validate_build(
            &building,
            &village.tribe,
            &village.buildings,
            village.is_capital,
        )?;

        let cost = building.cost();
        if !village.stocks.check_resources(&cost.resources) {
            return Err(GameError::NotEnoughResources.into());
        }

        village.stocks.remove_resources(&cost.resources);
        village.update_state();
        villages_repo.save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village.id as i32,
            slot_id: command.slot_id,
            name: building.clone().name,
        };

        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            building.calculate_build_time_secs(config.speed.clone() as u8) as i64,
            job_payload,
        );
        uow.jobs().add(&new_job).await?;

        Ok(())
    }
}
