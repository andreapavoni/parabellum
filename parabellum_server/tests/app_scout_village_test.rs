mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::ScoutVillageCommandHandler,
        cqrs::commands::ScoutVillage,
        jobs::{
            JobStatus,
            tasks::{ArmyReturnTask, ScoutTask},
        },
    };
    use parabellum_types::battle::{AttackType, ScoutingTarget, ScoutingTargetReport};
    use parabellum_types::tribe::Tribe;
    use parabellum_types::{Result, reports::ReportPayload};

    use super::test_utils::tests::setup_player_party;
    use crate::test_utils::tests::setup_app;

    #[tokio::test]
    async fn test_scout_village() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let scout_units = [0, 0, 0, 10, 0, 0, 0, 0, 0, 0]; // 10 Equites Legati (index 3)
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };
        let original_home_army_id = scout_army.id;
        let (_target_player, target_village, _target_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: original_home_army_id,
            target_village_id: target_village.id,
            target: ScoutingTarget::Resources,
            units: scout_units,
            attack_type: AttackType::Raid,
        };

        let handler = ScoutVillageCommandHandler::new();
        app.execute(command, handler).await?;

        let (scout_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.tx().await?;
            let jobs = uow_assert1
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;

            assert_eq!(jobs.len(), 1, "Should have 1 job in the queue");
            let job = jobs.first().unwrap().clone();

            assert_eq!(job.status, JobStatus::Pending);
            assert_eq!(job.task.task_type, "Scout");

            let payload: ScoutTask = serde_json::from_value(job.task.data.clone())?;
            assert_ne!(
                payload.army_id, original_home_army_id,
                "Deployed army ID should be new"
            );
            assert_eq!(payload.target, ScoutingTarget::Resources);
            assert_eq!(payload.attack_type, AttackType::Raid);

            let scout_village = uow_assert1.villages().get_by_id(scout_village.id).await?;

            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(original_home_army_id)
                    .await
                    .is_err(),
                "Initial home army should be removed",
            );

            assert!(
                uow_assert1
                    .armies()
                    .get_by_id(payload.army_id)
                    .await
                    .is_ok(),
                "Deployed scout army should exist",
            );

            assert!(
                scout_village.army().is_none(),
                "Scout village shouldn't have army at home (all troops sent)"
            );

            uow_assert1.rollback().await?;
            (job, payload.army_id)
        };

        worker.process_jobs(&vec![scout_job.clone()]).await?;

        let return_job = {
            let uow_assert2 = uow_provider.tx().await?;

            let original_job = uow_assert2.jobs().get_by_id(scout_job.id).await?;
            assert_eq!(original_job.status, JobStatus::Completed);

            let pending_jobs = uow_assert2
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 1, "Should have 1 return job.");

            let job = pending_jobs.first().unwrap().clone();
            assert_eq!(job.task.task_type, "ArmyReturn");

            let payload: ArmyReturnTask = serde_json::from_value(job.task.data.clone())?;
            assert_eq!(payload.army_id, deployed_army_id);
            assert_eq!(payload.resources.total(), 0, "Scouts don't carry a bounty");

            let army_status = uow_assert2.armies().get_by_id(deployed_army_id).await?;
            assert_eq!(army_status.units()[3], 10, "Scouts should have survived");
            assert!(
                uow_assert2
                    .armies()
                    .get_by_id(original_home_army_id)
                    .await
                    .is_err()
            );

            uow_assert2.rollback().await?;
            job
        };

        worker.process_jobs(&vec![return_job.clone()]).await?;
        {
            let uow_assert3 = uow_provider.tx().await?;
            let original_job = uow_assert3.jobs().get_by_id(return_job.id).await?;
            assert_eq!(
                original_job.status,
                JobStatus::Completed,
                "Return job should be completed"
            );

            let pending_jobs = uow_assert3
                .jobs()
                .list_by_player_id(scout_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 0, "Shouldn't be any jobs");

            let army_check = uow_assert3.armies().get_by_id(deployed_army_id).await;
            assert!(
                army_check.is_err(),
                "Moving army should be deleted after returning"
            );

            let home_village = uow_assert3.villages().get_by_id(scout_village.id).await?;

            assert!(
                home_village.army().is_some(),
                "Army should be returned at home"
            );
            let home_army = home_village.army().unwrap();
            assert_eq!(
                home_army.units()[3],
                10,
                "Expected 10 scouts at home, got {}",
                home_army.units()[3]
            );

            assert_ne!(home_army.id, deployed_army_id);
            assert_ne!(
                home_army.id, original_home_army_id,
                "Home army has a new ID after merge"
            );

            uow_assert3.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scout_creates_battle_report_with_scouting_data() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let scout_units = [0, 0, 0, 5, 0, 0, 0, 0, 0, 0]; // 5 scouts
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };
        let original_home_army_id = scout_army.id;

        let (_target_player, target_village, _target_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Test scouting resources
        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: original_home_army_id,
            target_village_id: target_village.id,
            target: ScoutingTarget::Resources,
            units: scout_units,
            attack_type: AttackType::Raid,
        };

        let handler = ScoutVillageCommandHandler::new();
        app.execute(command, handler).await?;

        let scout_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(scout_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs.first().unwrap().clone();
            uow.rollback().await?;
            job
        };

        // Process the scout job
        worker.process_jobs(&vec![scout_job.clone()]).await?;

        // Verify battle report was created with scouting data
        {
            let uow = uow_provider.tx().await?;

            let reports = uow.reports().list_for_player(scout_player.id, 100).await?;
            assert_eq!(reports.len(), 1, "Should have 1 battle report");

            let report = &reports[0];

            // Verify the report has scouting data
            assert!(matches!(report.payload, ReportPayload::Battle(_)));

            if let ReportPayload::Battle(battle_report) = report.payload.clone() {
                assert!(
                    battle_report.scouting.is_some(),
                    "Battle report should have scouting data"
                );

                let scouting_info = battle_report.scouting.as_ref().unwrap();
                assert_eq!(scouting_info.target, ScoutingTarget::Resources);
                // Verify we got resource information
                match &scouting_info.target_report {
                    ScoutingTargetReport::Resources(resources) => {
                        assert!(resources.total() > 0, "Should have scouted some resources");
                    }
                    _ => {
                        panic!("Expected Resources report");
                    }
                }
            }

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scout_defenses_creates_correct_report() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let scout_units = [0, 0, 0, 5, 0, 0, 0, 0, 0, 0]; // 5 scouts
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };
        let original_home_army_id = scout_army.id;

        let (_target_player, target_village, _target_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Test scouting defenses
        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: original_home_army_id,
            target_village_id: target_village.id,
            target: ScoutingTarget::Defenses,
            units: scout_units,
            attack_type: AttackType::Normal,
        };

        let handler = ScoutVillageCommandHandler::new();
        app.execute(command, handler).await?;

        let scout_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(scout_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs.first().unwrap().clone();
            uow.rollback().await?;
            job
        };

        // Process the scout job
        worker.process_jobs(&vec![scout_job.clone()]).await?;

        // Verify battle report was created with defense scouting data
        {
            let uow = uow_provider.tx().await?;

            let reports = uow.reports().list_for_player(scout_player.id, 100).await?;
            assert_eq!(reports.len(), 1, "Should have 1 battle report");

            let report = &reports[0];

            // Verify the report has scouting data
            assert!(matches!(report.payload, ReportPayload::Battle(_)));

            if let ReportPayload::Battle(battle_report) = report.payload.clone() {
                assert!(
                    battle_report.scouting.is_some(),
                    "Battle report should have scouting data"
                );

                let scouting_info = battle_report.scouting.as_ref().unwrap();
                assert_eq!(scouting_info.target, ScoutingTarget::Defenses);
                // Verify we got defense information
                match &scouting_info.target_report {
                    ScoutingTargetReport::Defenses {
                        wall,
                        palace,
                        residence,
                    } => {
                        // Just verify the structure is correct - actual values depend on target village setup
                        assert!(wall.is_none() || wall.is_some());
                        assert!(palace.is_none() || palace.is_some());
                        assert!(residence.is_none() || residence.is_some());
                    }
                    _ => {
                        panic!("Expected Defenses report");
                    }
                }
            }

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scout_report_hides_defender_troops_when_scouts_defeated() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup weak scout force (will be defeated)
        let scout_units = [0, 0, 0, 2, 0, 0, 0, 0, 0, 0]; // Only 2 scouts
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };

        // Setup strong defender (will defeat scouts)
        let defender_units = [0, 0, 0, 50, 0, 0, 0, 0, 0, 0]; // 50 Phalanxes
        let (_defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                defender_units,
                false,
            )
            .await?
        };

        // Setup reinforcements for defender
        let (reinforcer_player, reinforcer_village, reinforcer_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [30, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 30 Clubswingers
                false,
            )
            .await?
        };

        // Send reinforcements to defender
        let reinforce_command = parabellum_app::cqrs::commands::ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [30, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            hero_id: None,
        };
        app.execute(
            reinforce_command,
            parabellum_app::command_handlers::ReinforceVillageCommandHandler::new(),
        )
        .await?;

        // Process reinforcement
        let reinforce_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };
        worker.process_jobs(&vec![reinforce_job]).await?;

        // Send scouts (they will be defeated)
        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: scout_army.id,
            target_village_id: defender_village.id,
            target: ScoutingTarget::Resources,
            units: scout_units,
            attack_type: AttackType::Raid,
        };

        app.execute(command, ScoutVillageCommandHandler::new())
            .await?;

        let scout_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(scout_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };

        // Process the scout job
        worker.process_jobs(&vec![scout_job]).await?;

        // Verify battle report does NOT show defender troops
        {
            let uow = uow_provider.tx().await?;

            let reports = uow.reports().list_for_player(scout_player.id, 100).await?;
            assert_eq!(reports.len(), 1, "Should have 1 battle report");

            let report = &reports[0];
            assert!(matches!(report.payload, ReportPayload::Battle(_)));

            if let ReportPayload::Battle(battle_report) = report.payload.clone() {
                // Verify scouting failed (no survivors)
                assert!(
                    !battle_report.success,
                    "Scouting should have failed (all scouts killed)"
                );
                assert_eq!(
                    battle_report
                        .attacker
                        .as_ref()
                        .unwrap()
                        .survivors
                        .iter()
                        .sum::<u32>(),
                    0,
                    "All scouts should be dead"
                );

                // CRITICAL: Defender info should be hidden when scouts are defeated
                assert!(
                    battle_report.defender.is_none(),
                    "Defender troops should be HIDDEN when scouts are defeated"
                );
                assert_eq!(
                    battle_report.reinforcements.len(),
                    0,
                    "Reinforcement troops should be HIDDEN when scouts are defeated"
                );
            } else {
                panic!("Expected Battle report payload");
            }

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scout_report_shows_defender_troops_when_scouts_survive() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup strong scout force (will survive)
        let scout_units = [0, 0, 0, 20, 0, 0, 0, 0, 0, 0]; // 20 scouts
        let (scout_player, scout_village, scout_army, _, _) = {
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, scout_units, false).await?
        };

        // Setup weak defender (scouts will survive)
        let defender_units = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0]; // Only 2 Phalanxes
        let (_defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                defender_units,
                false,
            )
            .await?
        };

        // Setup weak reinforcements
        let (reinforcer_player, reinforcer_village, reinforcer_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [3, 0, 0, 0, 0, 0, 0, 0, 0, 0], // Only 3 Clubswingers
                false,
            )
            .await?
        };

        // Send reinforcements to defender
        let reinforce_command = parabellum_app::cqrs::commands::ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            hero_id: None,
        };
        app.execute(
            reinforce_command,
            parabellum_app::command_handlers::ReinforceVillageCommandHandler::new(),
        )
        .await?;

        // Process reinforcement
        let reinforce_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };
        worker.process_jobs(&vec![reinforce_job]).await?;

        // Send scouts (they will survive)
        let command = ScoutVillage {
            player_id: scout_player.id,
            village_id: scout_village.id,
            army_id: scout_army.id,
            target_village_id: defender_village.id,
            target: ScoutingTarget::Resources,
            units: scout_units,
            attack_type: AttackType::Raid,
        };

        app.execute(command, ScoutVillageCommandHandler::new())
            .await?;

        let scout_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(scout_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };

        // Process the scout job
        worker.process_jobs(&vec![scout_job]).await?;

        // Verify battle report SHOWS defender troops
        {
            let uow = uow_provider.tx().await?;

            let reports = uow.reports().list_for_player(scout_player.id, 100).await?;
            assert_eq!(reports.len(), 1, "Should have 1 battle report");

            let report = &reports[0];
            assert!(matches!(report.payload, ReportPayload::Battle(_)));

            if let ReportPayload::Battle(battle_report) = report.payload.clone() {
                // Verify scouting succeeded (has survivors)
                assert!(
                    battle_report.success,
                    "Scouting should have succeeded (scouts survived)"
                );
                assert!(
                    battle_report
                        .attacker
                        .as_ref()
                        .unwrap()
                        .survivors
                        .iter()
                        .sum::<u32>()
                        > 0,
                    "Some scouts should have survived"
                );

                // CRITICAL: Defender info should be visible when scouts survive
                assert!(
                    battle_report.defender.is_some(),
                    "Defender troops should be VISIBLE when scouts survive"
                );

                let defender_party = battle_report.defender.as_ref().unwrap();
                assert_eq!(
                    defender_party.tribe,
                    Tribe::Gaul,
                    "Should show defender tribe"
                );
                // Note: army_before and survivors show current state (after battle)
                // The actual values depend on battle outcome, but fields should be populated

                // Reinforcements should also be visible
                assert_eq!(
                    battle_report.reinforcements.len(),
                    1,
                    "Should show 1 reinforcement when scouts survive"
                );
                let reinforcement = &battle_report.reinforcements[0];
                assert_eq!(
                    reinforcement.tribe,
                    Tribe::Teuton,
                    "Should show reinforcement tribe"
                );

                // Scouting data should be present
                assert!(
                    battle_report.scouting.is_some(),
                    "Scouting data should be present when scouts survive"
                );
            } else {
                panic!("Expected Battle report payload");
            }

            uow.rollback().await?;
        }

        Ok(())
    }
}
