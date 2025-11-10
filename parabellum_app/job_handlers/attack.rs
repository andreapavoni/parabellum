use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_core::ApplicationError;
use parabellum_game::{
    battle::{AttackType, Battle},
    models::buildings::Building,
};
use parabellum_types::common::ResourceGroup;

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{ArmyReturnTask, AttackTask},
};

pub struct AttackJobHandler {
    payload: AttackTask,
}

impl AttackJobHandler {
    pub fn new(payload: AttackTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for AttackJobHandler {
    #[instrument(skip_all, fields(
          task_type = "Attack",
          attacker_army_id = %self.payload.army_id,
          attacker_village_id = %self.payload.attacker_village_id,
          target_village_id = %self.payload.target_village_id
      ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Execute Attack Job");

        let mut atk_army = ctx.uow.armies().get_by_id(self.payload.army_id).await?;
        let atk_village = ctx.uow.villages().get_by_id(atk_army.village_id).await?;

        let mut def_village = ctx
            .uow
            .villages()
            .get_by_id(self.payload.target_village_id as u32)
            .await?;

        // Find catapult targets on target village by looking for their name, or return random buildings
        let mut catapult_targets: Vec<Building> = Vec::new();

        for ct in &self.payload.catapult_targets {
            match def_village.get_building_by_name(&ct) {
                Some(b) => catapult_targets.push(b.building.clone()),
                None => {
                    def_village
                        .get_random_buildings(1)
                        .pop()
                        .map(|b| catapult_targets.push(b.clone()));
                }
            };
        }

        let catapult_targets: [Building; 2] = catapult_targets.try_into().unwrap();

        let battle = Battle::new(
            AttackType::Normal,
            atk_army.clone(),
            atk_village.clone(),
            def_village.clone(),
            Some(catapult_targets),
        );
        let report = battle.calculate_battle();

        atk_army.update_units(&report.attacker.survivors);
        ctx.uow.armies().save(&atk_army).await?;

        def_village.apply_battle_report(&report, ctx.config.speed)?;
        ctx.uow.villages().save(&def_village).await?;

        // Update armies
        if let Some(army) = def_village.army() {
            ctx.uow.armies().save(&army).await?;
        }

        for reinforcement_army in def_village.reinforcements() {
            ctx.uow.armies().save(&reinforcement_army).await?;
        }

        // --- 4. Return job ---
        let return_travel_time = atk_village.position.calculate_travel_time_secs(
            def_village.position,
            atk_army.speed(),
            ctx.config.world_size as i32,
            ctx.config.speed as u8,
        ) as i64;

        let player_id = atk_village.player_id;
        let village_id = atk_village.id as i32;
        let defender_village_id = def_village.id as i32;

        let return_payload = ArmyReturnTask {
            army_id: atk_army.id, // Attacking army ID
            resources: report.bounty.unwrap_or(ResourceGroup::new(0, 0, 0, 0)), // Resources to bring back
            destination_player_id: player_id,
            destination_village_id: village_id,
            from_village_id: defender_village_id,
        };

        let job_payload = JobPayload::new("ArmyReturn", serde_json::to_value(&return_payload)?);
        let return_job = Job::new(player_id, village_id, return_travel_time, job_payload);

        ctx.uow.jobs().add(&return_job).await?;

        info!(
            return_job_id = %return_job.id,
            arrival_at = %return_job.completed_at,
            "Army return job planned."
        );

        Ok(())
    }
}

// No unit tests here due to too much complexities in the setup.
// However, you can check `tests/attack_flow.rs` to see how this code is tested.
