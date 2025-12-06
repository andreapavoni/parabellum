use async_trait::async_trait;
use chrono::Utc;
use tracing::{info, instrument};

use parabellum_game::{battle::Battle, models::buildings::Building};
use parabellum_types::{
    common::ResourceGroup,
    errors::ApplicationError,
    reports::{BattleReportPayload, ReportPayload},
};

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{ArmyReturnTask, AttackTask},
};
use crate::repository::{NewReport, ReportAudience};

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
        let player_repo = ctx.uow.players();
        let report_repo = ctx.uow.reports();

        let mut atk_army = army_repo.get_by_id(self.payload.army_id).await?;
        let atk_village = village_repo.get_by_id(atk_army.village_id).await?;

        let mut def_village = village_repo
            .get_by_id(self.payload.target_village_id as u32)
            .await?;

        let attacker_player = player_repo.get_by_id(atk_village.player_id).await?;
        let defender_player = player_repo.get_by_id(def_village.player_id).await?;

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

        let battle = Battle::new(
            self.payload.attack_type.clone(),
            atk_army.clone(),
            atk_village.clone(),
            def_village.clone(),
            Some(catapult_targets),
        );
        let report = battle.calculate_battle();
        let bounty = report
            .bounty
            .clone()
            .unwrap_or(ResourceGroup::new(0, 0, 0, 0));

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
            army_id: atk_army.id,
            resources: bounty.clone(),
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

        let success = report
            .defender
            .as_ref()
            .map(|def| def.survivors.iter().copied().sum::<u32>() == 0)
            .unwrap_or(true);

        let battle_payload = BattleReportPayload {
            attacker_player: attacker_player.username.clone(),
            attacker_village: atk_village.name.clone(),
            defender_player: defender_player.username.clone(),
            defender_village: def_village.name.clone(),
            success,
            bounty,
        };

        let new_report = NewReport {
            report_type: "battle".to_string(),
            payload: ReportPayload::Battle(battle_payload),
            actor_player_id: atk_village.player_id,
            actor_village_id: Some(atk_village.id),
            target_player_id: Some(def_village.player_id),
            target_village_id: Some(def_village.id),
        };

        let audiences = vec![
            ReportAudience {
                player_id: atk_village.player_id,
                read_at: Some(Utc::now()),
            },
            ReportAudience {
                player_id: def_village.player_id,
                read_at: None,
            },
        ];

        report_repo.add(&new_report, &audiences).await?;

        Ok(())
    }
}
