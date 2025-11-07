// File: parabellum_app/src/job_handlers/reinforcement.rs
use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_core::ApplicationError;

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

        // 1. Carica l'armata
        let mut army = army_repo.get_by_id(self.payload.army_id).await?;

        // 2. Aggiorna il suo 'current_map_field_id' con il villaggio target
        //    (preso dal payload del task)
        army.current_map_field_id = Some(self.payload.village_id as u32);

        // 3. Salva l'armata.
        //    Quando il villaggio target (village_id) verrà caricato,
        //    la logica in `village.rs` (TryFrom<VillageAggregate>)
        //    la identificherà correttamente come rinforzo.
        army_repo.save(&army).await?;

        info!(
            army_id = %army.id,
            new_location_id = %self.payload.village_id,
            "Army reinforcement has arrived and is now stationed at new location."
        );

        // Come da piano, nessun job di ritorno viene creato.
        Ok(())
    }
}
