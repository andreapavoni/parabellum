use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::models::army::Army;
use parabellum_types::{buildings::BuildingName, errors::ApplicationError};

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::ReinforcementTask,
};

pub struct ReinforcementJobHandler {
    payload: ReinforcementTask,
}

impl ReinforcementJobHandler {
    pub fn new(payload: ReinforcementTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ReinforcementJobHandler {
    #[instrument(skip_all, fields(
        task_type = "Reinforcement",
        army_id = %self.payload.army_id,
        target_village_id = %self.payload.village_id,
        player_id = %self.payload.player_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing Reinforcement job: Army arriving at village.");
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let hero_repo = ctx.uow.heroes();

        let mut target_village = village_repo
            .get_by_id(self.payload.village_id as u32)
            .await?;
        let mut reinforcement = army_repo.get_by_id(self.payload.army_id).await?;

        // To switch village, hero should be alone and target village should have HeroMansion
        if target_village.player_id == self.payload.player_id
            && reinforcement.units().iter().sum::<u32>() == 0
            && target_village
                .get_building_by_name(&BuildingName::HeroMansion)
                .is_some()
        {
            if let Some(mut hero) = reinforcement.hero() {
                reinforcement.set_hero(None);
                hero.village_id = target_village.id;
                hero_repo.save(&hero).await?;
                army_repo.save(&reinforcement).await?;

                if let Some(garrison) = target_village.army() {
                    let mut home_army = garrison.clone();
                    home_army.set_hero(Some(hero.clone()));
                    target_village.set_army(Some(&home_army))?;
                    army_repo.remove(reinforcement.id).await?;
                    army_repo.save(&home_army).await?;
                } else {
                    let mut new_army = Army::new_village_army(&target_village);
                    new_army.set_hero(Some(hero.clone()));
                    army_repo.save(&new_army).await?;
                    target_village.set_army(Some(&new_army))?;
                }
            }
        } else {
            // Or everything goes into target village reinforcements
            reinforcement.current_map_field_id = Some(target_village.id);
            army_repo.save(&reinforcement).await?;
        }

        village_repo.save(&target_village).await?;

        info!(
            army_id = %reinforcement.id,
            new_location_id = %self.payload.village_id,
            "Army reinforcement has arrived and is now stationed at new location."
        );

        Ok(())
    }
}
