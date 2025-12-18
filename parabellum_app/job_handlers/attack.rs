use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::{battle::Battle, models::buildings::Building};
use parabellum_types::{
    common::ResourceGroup,
    errors::ApplicationError,
    reports::{BattlePartyPayload, BattleReportPayload, ReportPayload},
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

        army_repo.save_or_remove(&atk_army).await?;

        // Save or remove defender's home army
        if let Some(army) = def_village.army() {
            if let Some(hero) = army.hero() {
                hero_repo.save(&hero).await?;
            }
            army_repo.save_or_remove(army).await?;

            // Clear village reference if army was destroyed
            if army.immensity() == 0 {
                def_village.set_army(None)?;
            }
        }

        // Save or remove reinforcements
        for reinforcement_army in def_village.reinforcements() {
            if let Some(hero) = reinforcement_army.hero() {
                hero_repo.save(&hero).await?;
            }
            army_repo.save_or_remove(reinforcement_army).await?;
        }

        village_repo.save(&def_village).await?;

        let return_travel_time = atk_village.position.calculate_travel_time_secs(
            def_village.position.clone(),
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
            .map(|def| def.survivors.immensity() == 0)
            .unwrap_or(true);

        let attacker_payload = BattlePartyPayload {
            tribe: report.attacker.army_before.tribe.clone(),
            army_before: report.attacker.army_before.units().clone(),
            survivors: report.attacker.survivors,
            losses: report.attacker.losses,
        };

        let defender_payload = report.defender.as_ref().map(|def| BattlePartyPayload {
            tribe: def.army_before.tribe.clone(),
            army_before: def.army_before.units().clone(),
            survivors: def.survivors.clone(),
            losses: def.losses.clone(),
        });

        let reinforcements_payload: Vec<BattlePartyPayload> = report
            .reinforcements
            .iter()
            .map(|reinf| BattlePartyPayload {
                tribe: reinf.army_before.tribe.clone(),
                army_before: reinf.army_before.units().clone(),
                survivors: reinf.survivors.clone(),
                losses: reinf.losses.clone(),
            })
            .collect();

        let battle_payload = BattleReportPayload {
            attack_type: report.attack_type.clone(),
            attacker_player: attacker_player.username.clone(),
            attacker_village: atk_village.name.clone(),
            attacker_position: atk_village.position.clone(),
            defender_player: defender_player.username.clone(),
            defender_village: def_village.name.clone(),
            defender_position: def_village.position.clone(),
            success,
            bounty,
            attacker: Some(attacker_payload),
            defender: defender_payload,
            reinforcements: reinforcements_payload,
            scouting: report.scouting,
            wall_damage: report.wall_damage,
            catapult_damage: report.catapult_damage,
        };

        let new_report = NewReport {
            report_type: "battle".to_string(),
            payload: ReportPayload::Battle(battle_payload),
            actor_player_id: atk_village.player_id,
            actor_village_id: Some(atk_village.id),
            target_player_id: Some(def_village.player_id),
            target_village_id: Some(def_village.id),
        };

        // TODO: add reinforcements owners in audiences
        let audiences = vec![
            ReportAudience {
                player_id: atk_village.player_id,
                read_at: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        jobs::handler::JobHandlerContext,
        repository::ReportRepository,
        test_utils::tests::{MockReportRepository, MockUnitOfWork},
        uow::UnitOfWork,
    };
    use parabellum_game::{
        models::map::Valley,
        test_utils::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    };
    use parabellum_types::{army::TroopSet, battle::AttackType};
    use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_attack_job_persists_reports_with_audience_state() {
        let uow = MockUnitOfWork::new();
        let report_repo: Arc<MockReportRepository> = uow.report_repo();
        let config = Arc::new(Config {
            world_size: 100,
            speed: 1,
            auth_cookie_secret: "test-secret".to_string(),
        });

        let attacker_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let defender_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let attacker_valley: Valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 0, y: 0 }),
            ..Default::default()
        });
        let defender_valley: Valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 5, y: 5 }),
            ..Default::default()
        });

        let attacker_village = village_factory(VillageFactoryOptions {
            player: Some(attacker_player.clone()),
            valley: Some(attacker_valley),
            ..Default::default()
        });
        let defender_village = village_factory(VillageFactoryOptions {
            player: Some(defender_player.clone()),
            valley: Some(defender_valley),
            ..Default::default()
        });

        let attacker_army = army_factory(ArmyFactoryOptions {
            player_id: Some(attacker_player.id),
            village_id: Some(attacker_village.id),
            units: Some(TroopSet::new([50, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
            ..Default::default()
        });

        uow.players().save(&attacker_player).await.unwrap();
        uow.players().save(&defender_player).await.unwrap();
        uow.villages().save(&attacker_village).await.unwrap();
        uow.villages().save(&defender_village).await.unwrap();
        uow.armies().save(&attacker_army).await.unwrap();

        let payload = AttackTask {
            army_id: attacker_army.id,
            attacker_village_id: attacker_village.id as i32,
            attacker_player_id: attacker_player.id,
            target_village_id: defender_village.id as i32,
            target_player_id: defender_player.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            attack_type: AttackType::Normal,
        };

        let handler = AttackJobHandler::new(payload);
        let dummy_job = Job::new(
            attacker_player.id,
            attacker_village.id as i32,
            0,
            JobPayload::new("Attack", json!({})),
        );
        let ctx = JobHandlerContext {
            uow: Box::new(uow),
            config,
        };

        handler.handle(&ctx, &dummy_job).await.unwrap();

        let attacker_reports = report_repo
            .list_for_player(attacker_player.id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports.len(), 1);
        assert!(
            attacker_reports[0].read_at.is_none(),
            "attacker report should not be marked read"
        );

        let defender_reports = report_repo
            .list_for_player(defender_player.id, 10)
            .await
            .unwrap();
        assert_eq!(defender_reports.len(), 1);
        assert!(
            defender_reports[0].read_at.is_none(),
            "defender report should be unread"
        );
    }
}
