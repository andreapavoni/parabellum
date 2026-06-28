//! Village economy helpers used before command dispatch.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::SetVillageResources;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::GameError;

use super::VillageEsService;

impl VillageEsService {
    pub(super) async fn materialize_current_resources_for_command(
        &self,
        village_id: u32,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let current = self.get_village(village_id).await?;
        if current.player_id != player_id {
            return Err(CqrsError::domain_source(GameError::VillageNotOwned {
                village_id,
                player_id,
            }));
        }
        let resources = ResourceGroup::new(
            current.stocks.lumber,
            current.stocks.clay,
            current.stocks.iron,
            current.stocks.crop.max(0) as u32,
        );
        self.set_village_resources(
            village_id,
            &SetVillageResources {
                player_id,
                resources,
            },
        )
        .await?;
        Ok(())
    }
}
