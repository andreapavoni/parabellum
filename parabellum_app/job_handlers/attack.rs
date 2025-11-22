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
use crate::job_handlers::helpers::get_defender_alliance_metallurgy_bonus;

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

        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let hero_repo = ctx.uow.heroes();

        let mut atk_army = army_repo.get_by_id(self.payload.army_id).await?;
        let atk_village = village_repo.get_by_id(atk_army.village_id).await?;

        let mut def_village = village_repo
            .get_by_id(self.payload.target_village_id as u32)
            .await?;

        let mut catapult_targets: Vec<Building> = Vec::new();
        for ct in &self.payload.catapult_targets {
            match def_village.get_building_by_name(ct) {
                Some(b) => catapult_targets.push(b.building.clone()),
                None => {
                    if let Some(b) = def_village.get_random_buildings(1).pop() {
                        catapult_targets.push(b.clone())
                    }
                }
            };
        }
        let catapult_targets: [Building; 2] = catapult_targets.try_into().unwrap();

        let defender_alliance_bonus = get_defender_alliance_metallurgy_bonus(&ctx.uow, &def_village).await?;

        let battle = Battle::new(
            AttackType::Normal,
            atk_army.clone(),
            atk_village.clone(),
            def_village.clone(),
            Some(catapult_targets),
            defender_alliance_bonus,
        );
        let report = battle.calculate_battle();

        atk_army.apply_battle_report(&report.attacker);
        def_village.apply_battle_report(&report, ctx.config.speed)?;

        if let Some(hero) = atk_army.hero() {
            hero_repo.save(&hero).await?;
        }

        army_repo.save(&atk_army).await?;
        village_repo.save(&def_village).await?;

        if let Some(army) = def_village.army() {
            army_repo.save(army).await?;

            if let Some(hero) = army.hero() {
                hero_repo.save(&hero).await?;
            }
        }

        for reinforcement_army in def_village.reinforcements() {
            army_repo.save(reinforcement_army).await?;

            if let Some(hero) = reinforcement_army.hero() {
                hero_repo.save(&hero).await?;
            }
        }

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
