use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::battle::Battle;
use parabellum_types::{
    common::ResourceGroup,
    errors::ApplicationError,
    reports::{BattlePartyPayload, BattleReportPayload, ReportPayload},
};

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::{ArmyReturnTask, ScoutTask},
};
use crate::repository::{NewReport, ReportAudience};

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
            self.payload.attack_type.clone(),
            attacker_army.clone(),
            attacker_village.clone(),
            defender_village.clone(),
            None,
        );
        let battle_report = battle.calculate_scout_battle(self.payload.target.clone());

        info!(?battle_report, "Scouting battle report calculated.");
        info!(?battle_report.scouting, "Scouting info from battle report");

        // Create and save battle report
        let attacker_player = ctx
            .uow
            .players()
            .get_by_id(attacker_village.player_id)
            .await?;
        let defender_player = ctx
            .uow
            .players()
            .get_by_id(defender_village.player_id)
            .await?;

        let attacker_payload = BattlePartyPayload {
            tribe: attacker_army.tribe.clone(),
            army_before: battle_report.attacker.army_before.units().clone(),
            survivors: battle_report.attacker.survivors,
            losses: battle_report.attacker.losses,
        };

        let scouting_success = battle_report
            .scouting
            .as_ref()
            .is_some_and(|_| battle_report.attacker.survivors.iter().any(|&u| u > 0));

        let battle_payload = BattleReportPayload {
            attack_type: self.payload.attack_type.clone(),
            attacker_player: attacker_player.username.clone(),
            attacker_village: attacker_village.name.clone(),
            attacker_position: attacker_village.position.clone(),
            defender_player: defender_player.username.clone(),
            defender_village: defender_village.name.clone(),
            defender_position: defender_village.position.clone(),
            success: battle_report.attacker.survivors.iter().any(|&u| u > 0),
            bounty: ResourceGroup::new(0, 0, 0, 0),
            attacker: Some(attacker_payload),
            defender: if scouting_success {
                Some(BattlePartyPayload {
                    tribe: defender_village.tribe.clone(),
                    army_before: defender_village
                        .army()
                        .map(|a| *a.units())
                        .unwrap_or_default(),
                    survivors: defender_village
                        .army()
                        .map(|a| *a.units())
                        .unwrap_or_default(),
                    losses: [0; 10],
                })
            } else {
                None
            },
            reinforcements: if scouting_success {
                defender_village
                    .reinforcements()
                    .iter()
                    .map(|r| BattlePartyPayload {
                        tribe: r.tribe.clone(),
                        army_before: *r.units(),
                        survivors: *r.units(),
                        losses: [0; 10],
                    })
                    .collect()
            } else {
                vec![]
            },
            scouting: battle_report.scouting.clone(),
            wall_damage: None,
            catapult_damage: vec![],
        };

        let new_report = NewReport {
            report_type: "battle".to_string(),
            payload: ReportPayload::Battle(battle_payload),
            actor_player_id: attacker_village.player_id,
            actor_village_id: Some(attacker_village.id),
            target_player_id: Some(defender_village.player_id),
            target_village_id: Some(defender_village.id),
        };

        let mut audiences = vec![ReportAudience {
            player_id: attacker_village.player_id,
            read_at: None,
        }];

        // If scouts were detected, defender also gets a report
        if let Some(ref scouting) = battle_report.scouting {
            if scouting.was_detected {
                audiences.push(ReportAudience {
                    player_id: defender_village.player_id,
                    read_at: None,
                });
            }
        }

        ctx.uow.reports().add(&new_report, &audiences).await?;

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
