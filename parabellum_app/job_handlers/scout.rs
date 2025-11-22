use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::battle::{AttackType, Battle};
use parabellum_types::{common::ResourceGroup, errors::ApplicationError};

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{ArmyReturnTask, ScoutTask},
};

pub struct ScoutJobHandler {
    payload: ScoutTask,
}

impl ScoutJobHandler {
    pub fn new(payload: ScoutTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ScoutJobHandler {
    #[instrument(skip_all, fields(
        task_type = "Scout",
        attacker_army_id = %self.payload.army_id,
        attacker_village_id = %self.payload.attacker_village_id,
        target_village_id = %self.payload.target_village_id,
        target = ?self.payload.target
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Execute Scout Job");

        // 1. Carica le entità
        let mut attacker_army = ctx.uow.armies().get_by_id(self.payload.army_id).await?;
        let attacker_village = ctx
            .uow
            .villages()
            .get_by_id(self.payload.attacker_village_id as u32)
            .await?;
        let defender_village = ctx
            .uow
            .villages()
            .get_by_id(self.payload.target_village_id as u32)
            .await?;

        let battle = Battle::new(
            AttackType::Raid,
            attacker_army.clone(),
            attacker_village.clone(),
            defender_village.clone(),
            None,
        );
        let battle_report = battle.calculate_scout_battle(self.payload.target.clone());

        info!(?battle_report, "Scouting battle report calculated.");
        // TODO: Store battle report for player(s)

        attacker_army.update_units(&battle_report.attacker.survivors);
        ctx.uow.armies().save(&attacker_army).await?;

        let return_travel_time = attacker_village.position.calculate_travel_time_secs(
            defender_village.position,
            attacker_army.speed(),
            ctx.config.world_size as i32,
            ctx.config.speed as u8,
        ) as i64;

        let return_payload = ArmyReturnTask {
            army_id: attacker_army.id,
            resources: ResourceGroup::new(0, 0, 0, 0),
            destination_player_id: attacker_village.player_id,
            destination_village_id: attacker_village.id as i32,
            from_village_id: defender_village.id as i32,
        };

        let job_payload = JobPayload::new("ArmyReturn", serde_json::to_value(&return_payload)?);
        let return_job = Job::new(
            attacker_village.player_id,
            attacker_village.id as i32,
            return_travel_time,
            job_payload,
        );

        ctx.uow.jobs().add(&return_job).await?;

        info!(
            return_job_id = %return_job.id,
            arrival_at = %return_job.completed_at,
            "Scout army return job planned."
        );

        Ok(())
    }
}
