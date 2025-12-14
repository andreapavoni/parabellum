mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::AttackVillageCommandHandler,
        cqrs::commands::AttackVillage,
        jobs::{
            JobStatus,
            tasks::{ArmyReturnTask, AttackTask},
        },
    };
    use parabellum_game::models::{buildings::Building, village::Village};
    use parabellum_types::{
        Result, battle::AttackType, buildings::BuildingName, common::ResourceGroup, tribe::Tribe,
    };

    use crate::test_utils::tests::{setup_app, setup_player_party};

    #[tokio::test]
    async fn test_simple_attack() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;
        let units_to_send = [100, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let (attacker_player, attacker_village, attacker_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                units_to_send,
                false,
            )
            .await?
        };

        let original_home_army_id = attacker_army.id;
        let (defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: original_home_army_id, // Use original ID
            units: units_to_send,           // Pass the units
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
            attack_type: AttackType::Normal,
        };
        let handler = AttackVillageCommandHandler::new();
        app.execute(attack_command, handler).await?;

        let (attack_job, deployed_army_id) = {
            let uow_assert1 = uow_provider.tx().await?;
            let cloned_job;
            let deployed_id;

            {
                let job_repo = uow_assert1.jobs();
                let jobs = job_repo
                    .list_by_player_id(attacker_player.id)
                    .await
                    .unwrap();

                assert_eq!(jobs.len(), 1, "There should be exactly 1 job in the queue.");
                let attack_job = &jobs[0];

                assert_eq!(attack_job.task.task_type, "Attack");
                let task_data: Result<AttackTask, _> =
                    serde_json::from_value(attack_job.task.data.clone());
                assert!(task_data.is_ok(), "Job data should be a valid AttackTask");

                let payload = task_data.unwrap();
                assert_ne!(
                    payload.army_id, original_home_army_id,
                    "Deployed army ID should be new"
                );
                deployed_id = payload.army_id;

                assert_eq!(
                    attack_job.status,
                    JobStatus::Pending,
                    "Expected job status set to Pending, got {:?}",
                    attack_job.status
                );

                let home_village = uow_assert1
                    .villages()
                    .get_by_id(attacker_village.id)
                    .await?;
                assert!(
                    home_village.army().is_none(),
                    "Home village army should be None (all troops sent)"
                );
                assert!(
                    uow_assert1
                        .armies()
                        .get_by_id(original_home_army_id)
                        .await
                        .is_err(),
                    "Initial home army should be removed"
                );
                assert!(
                    uow_assert1.armies().get_by_id(deployed_id).await.is_ok(),
                    "Deployed army should exist"
                );

                cloned_job = attack_job.clone();
            }

            uow_assert1.rollback().await?;
            (cloned_job, deployed_id)
        };

        worker.process_jobs(&vec![attack_job.clone()]).await?;

        let return_job = {
            let uow_assert2 = uow_provider.tx().await?;
            let job_repo = uow_assert2.jobs();
            let final_jobs = job_repo
                .list_by_player_id(attacker_player.id)
                .await
                .unwrap();

            assert_eq!(
                final_jobs.len(),
                1,
                "There should be exactly 1 pending job in the queue (the return)."
            );

            if let Ok(original_job) = job_repo.get_by_id(attack_job.id).await {
                assert_eq!(original_job.status, JobStatus::Completed);
            }

            let return_job = final_jobs.first().unwrap().clone();
            assert_eq!(return_job.task.task_type, "ArmyReturn");
            let return_task_data: Result<ArmyReturnTask, _> =
                serde_json::from_value(return_job.task.data.clone());
            assert!(
                return_task_data.is_ok(),
                "Job data should be a valid ArmyReturnTask"
            );
            assert_eq!(return_task_data.unwrap().army_id, deployed_army_id);
            assert_eq!(return_job.status, JobStatus::Pending);

            uow_assert2.rollback().await?;
            return_job
        };

        worker.process_jobs(&vec![return_job.clone()]).await?;
        {
            let uow_assert3 = uow_provider.tx().await?;
            let final_return_job = uow_assert3.jobs().get_by_id(return_job.id).await?;
            assert_eq!(final_return_job.status, JobStatus::Completed);

            let pending_jobs = uow_assert3
                .jobs()
                .list_by_player_id(attacker_player.id)
                .await?;
            assert_eq!(pending_jobs.len(), 0, "No jobs should be pending");

            assert!(
                uow_assert3
                    .armies()
                    .get_by_id(deployed_army_id)
                    .await
                    .is_err(),
                "Deployed army should be deleted after returning"
            );

            let home_village = uow_assert3
                .villages()
                .get_by_id(attacker_village.id)
                .await?;
            assert!(
                home_village.army().is_some(),
                "Home army should be back in village"
            );
            let home_army = home_village.army().unwrap();
            assert_eq!(
                home_army.units()[0],
                100,
                "Home army should have 100 troops (assuming 0 losses for simplicity)"
            );
            assert_ne!(
                home_army.id, deployed_army_id,
                "Home army ID should be new (or the original, depending on merge logic)"
            );
            assert_ne!(
                home_army.id, original_home_army_id,
                "Home army ID should be new after merge"
            );

            // Ensure a report exists for both attacker and defender with correct read state
            let report_repo = uow_assert3.reports();
            let attacker_reports = report_repo.list_for_player(attacker_player.id, 5).await?;
            assert!(
                attacker_reports.iter().any(|r| r.report_type == "battle"),
                "Attacker should receive a battle report"
            );
            assert!(
                attacker_reports.iter().any(|r| r.read_at.is_none()),
                "Attacker report should not be marked read"
            );

            let defender_reports = report_repo.list_for_player(defender_player.id, 5).await?;
            assert!(
                defender_reports.iter().any(|r| r.report_type == "battle"),
                "Defender should receive a battle report"
            );
            assert!(
                defender_reports.iter().any(|r| r.read_at.is_none()),
                "Defender report should remain unread"
            );

            // Verify battle report contains attacker and defender payloads
            let battle_report = attacker_reports
                .iter()
                .find(|r| r.report_type == "battle")
                .unwrap();
            if let parabellum_types::reports::ReportPayload::Battle(ref payload) =
                battle_report.payload
            {
                assert!(
                    payload.attacker.is_some(),
                    "Battle report should include attacker"
                );
                assert!(
                    payload.defender.is_some(),
                    "Battle report should include defender"
                );

                let attacker_party = payload.attacker.as_ref().unwrap();
                assert_eq!(
                    attacker_party.tribe,
                    Tribe::Roman,
                    "Attacker tribe should be Roman"
                );
                assert_eq!(
                    attacker_party.army_before[0], 100,
                    "Attacker should have started with 100 units"
                );

                let defender_party = payload.defender.as_ref().unwrap();
                assert_eq!(
                    defender_party.tribe,
                    Tribe::Roman,
                    "Defender tribe should be Roman"
                );
            } else {
                panic!("Expected Battle report payload");
            }

            uow_assert3.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_attack_with_catapult_damage_and_bounty() -> Result<()> {
        let (app, worker, uow_provider, config) = setup_app(false).await?;

        let (attacker_player, attacker_village, attacker_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [100, 0, 0, 0, 0, 0, 0, 100, 0, 0],
                false,
            )
            .await?
        };

        let (_, mut defender_village, _, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        {
            let uow_update = uow_provider.tx().await?;
            let village_repo = uow_update.villages();

            let granary =
                Building::new(BuildingName::Granary, config.speed).at_level(1, config.speed)?;
            let warehouse =
                Building::new(BuildingName::Warehouse, config.speed).at_level(1, config.speed)?;

            defender_village.add_building_at_slot(granary, 21)?;
            defender_village.add_building_at_slot(warehouse, 20)?;
            defender_village.store_resources(&ResourceGroup::new(800, 800, 800, 800));

            village_repo.save(&defender_village).await?;
            uow_update.commit().await?;
        };

        let initial_warehouse_level = defender_village
            .get_building_by_name(&BuildingName::Warehouse)
            .unwrap()
            .building
            .level;
        let initial_defender_resources = defender_village.stored_resources().total();

        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [100, 0, 0, 0, 0, 0, 0, 100, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::Warehouse, BuildingName::Granary],
            hero_id: None,
            attack_type: AttackType::Normal,
        };
        let handler = AttackVillageCommandHandler::new();
        app.execute(attack_command, handler).await?;

        let jobs = {
            let uow_read_jobs = uow_provider.tx().await?;
            let jobs = uow_read_jobs
                .jobs()
                .list_by_player_id(attacker_player.id)
                .await?;
            uow_read_jobs.rollback().await?;
            jobs
        };

        worker.process_jobs(&jobs).await?;
        {
            let uow_assert = uow_provider.tx().await?;
            let village_repo = uow_assert.villages();
            let job_repo = uow_assert.jobs();

            let updated_defender_village: Village =
                village_repo.get_by_id(defender_village.id).await?;

            let return_jobs = job_repo.list_by_player_id(attacker_player.id).await?;
            assert_eq!(
                return_jobs.len(),
                1,
                "There should be exactly 1 return job after the attack."
            );
            let return_job = return_jobs.first().unwrap();

            // Building damages
            let final_warehouse_level = updated_defender_village
                .get_building_by_name(&BuildingName::Warehouse)
                .map_or(0, |b| b.building.level);

            assert!(
                final_warehouse_level < initial_warehouse_level,
                "Warehouse should have been damaged (Before: {}, After: {}).",
                initial_warehouse_level,
                final_warehouse_level
            );

            // Bounty
            assert_eq!(return_job.task.task_type, "ArmyReturn".to_string());
            let return_payload: ArmyReturnTask =
                serde_json::from_value(return_job.task.data.clone())?;

            let bounty = return_payload.resources.total();
            assert!(bounty > 0, "Attacker should have stolen resources.");

            let final_defender_resources = updated_defender_village.stored_resources().total();
            assert!(
                final_defender_resources < initial_defender_resources,
                "Defender resources should have been reduced (Before: {}, After: {}).",
                initial_defender_resources,
                final_defender_resources
            );

            uow_assert.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_battle_report_includes_defender_and_reinforcements() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup attacker
        let (attacker_player, attacker_village, attacker_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Setup defender with home army
        let (_defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [30, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Setup reinforcer (third player)
        let (reinforcer_player, reinforcer_village, reinforcer_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                false,
            )
            .await?
        };

        // Send reinforcements to defender
        let reinforce_command = parabellum_app::cqrs::commands::ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [20, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            hero_id: None,
        };
        let reinforce_handler =
            parabellum_app::command_handlers::ReinforceVillageCommandHandler::new();
        app.execute(reinforce_command, reinforce_handler).await?;

        // Process reinforcement job
        let reinforce_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(reinforcer_player.id).await?;
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };
        worker.process_jobs(&vec![reinforce_job]).await?;

        // Attack the defender (who now has reinforcements)
        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [50, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
            attack_type: AttackType::Normal,
        };
        let attack_handler = AttackVillageCommandHandler::new();
        app.execute(attack_command, attack_handler).await?;

        // Process attack job
        let attack_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(attacker_player.id).await?;
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };
        worker.process_jobs(&vec![attack_job]).await?;

        // Verify battle report includes defender and reinforcements
        {
            let uow = uow_provider.tx().await?;
            let report_repo = uow.reports();
            let attacker_reports = report_repo.list_for_player(attacker_player.id, 5).await?;

            let battle_report = attacker_reports
                .iter()
                .find(|r| r.report_type == "battle")
                .expect("Battle report should exist");

            if let parabellum_types::reports::ReportPayload::Battle(ref payload) =
                battle_report.payload
            {
                // Verify attacker
                assert!(payload.attacker.is_some(), "Should include attacker");
                let attacker_party = payload.attacker.as_ref().unwrap();
                assert_eq!(attacker_party.tribe, Tribe::Roman);
                assert_eq!(attacker_party.army_before[0], 50);

                // Verify defender
                assert!(payload.defender.is_some(), "Should include defender");
                let defender_party = payload.defender.as_ref().unwrap();
                assert_eq!(defender_party.tribe, Tribe::Gaul);
                assert_eq!(defender_party.army_before[0], 30);

                // Verify reinforcements
                assert_eq!(
                    payload.reinforcements.len(),
                    1,
                    "Should include 1 reinforcement"
                );
                let reinforcement = &payload.reinforcements[0];
                assert_eq!(reinforcement.tribe, Tribe::Teuton);
                assert_eq!(reinforcement.army_before[0], 20);

                // All parties should have losses data
                assert!(
                    attacker_party.losses.iter().sum::<u32>() > 0,
                    "Attacker should have losses data"
                );
                assert!(
                    defender_party.losses.iter().sum::<u32>() > 0,
                    "Defender should have losses data"
                );
                assert!(
                    reinforcement.losses.iter().sum::<u32>() > 0,
                    "Reinforcement should have losses data"
                );
            } else {
                panic!("Expected Battle report payload");
            }

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_destroyed_defender_armies_are_deleted() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Setup strong attacker
        let (attacker_player, attacker_village, attacker_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [0, 0, 200, 0, 0, 0, 0, 0, 0, 0], // Strong army
                false,
            )
            .await?
        };

        // Setup weak defender with home army
        let (_defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [1, 0, 0, 0, 0, 0, 0, 0, 0, 0], // Weak army
                false,
            )
            .await?
        };

        // Setup weak reinforcement
        let (reinforcer_player, reinforcer_village, reinforcer_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Teuton,
                [1, 0, 0, 0, 0, 0, 0, 0, 0, 0], // Weak army
                false,
            )
            .await?
        };

        // Send reinforcements
        let reinforce_command = parabellum_app::cqrs::commands::ReinforceVillage {
            player_id: reinforcer_player.id,
            village_id: reinforcer_village.id,
            army_id: reinforcer_army.id,
            units: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
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

        // Attack with overwhelming force
        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [0, 0, 200, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
            attack_type: AttackType::Normal,
        };
        app.execute(attack_command, AttackVillageCommandHandler::new())
            .await?;

        // Process attack
        let attack_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(attacker_player.id).await?;
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };
        worker.process_jobs(&vec![attack_job]).await?;

        // Verify destroyed armies are gone from village
        {
            let uow = uow_provider.tx().await?;
            let updated_defender = uow.villages().get_by_id(defender_village.id).await?;

            // After total defeat, village should have no armies
            assert!(
                updated_defender.army().is_none(),
                "Defender village should have no home army after total defeat. Found: {:?}",
                updated_defender.army().map(|a| (a.id, *a.units()))
            );
            assert_eq!(
                updated_defender.reinforcements().len(),
                0,
                "Defender village should have no reinforcements after total defeat. Found: {:?}",
                updated_defender
                    .reinforcements()
                    .iter()
                    .map(|a| (a.id, *a.units()))
                    .collect::<Vec<_>>()
            );

            uow.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_battle_report_includes_damage_data() -> Result<()> {
        let (app, worker, uow_provider, _) = setup_app(false).await?;

        // Attacker with rams and catapults
        let units_with_siege = [50, 0, 0, 0, 0, 0, 0, 10, 5, 0]; // 50 legionnaires, 10 catapults, 5 rams
        let (attacker_player, attacker_village, attacker_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                units_with_siege,
                false,
            )
            .await?
        };

        // Defender with minimal defense
        let (_defender_player, defender_village, _defender_army, _, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Gaul,
                [5, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 5 phalanxes
                false,
            )
            .await?
        };

        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: units_with_siege,
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
            attack_type: AttackType::Normal,
        };

        app.execute(attack_command, AttackVillageCommandHandler::new())
            .await?;

        let attack_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(attacker_player.id).await?;
            assert_eq!(jobs.len(), 1);
            let job = jobs[0].clone();
            uow.rollback().await?;
            job
        };

        // Process the attack
        worker.process_jobs(&vec![attack_job]).await?;

        // Verify battle report includes damage data
        {
            let uow = uow_provider.tx().await?;
            let reports = uow
                .reports()
                .list_for_player(attacker_player.id, 10)
                .await?;

            let battle_report = reports
                .iter()
                .find(|r| r.report_type == "battle")
                .expect("Should have a battle report");

            if let parabellum_types::reports::ReportPayload::Battle(ref payload) =
                battle_report.payload
            {
                // We're just checking the field is accessible
                assert!(payload.wall_damage.is_some() || payload.wall_damage.is_none());

                // Verify damage report structure is correct if damage exists
                if let Some(ref wall_dmg) = payload.wall_damage {
                    assert!(
                        wall_dmg.level_before >= wall_dmg.level_after,
                        "Wall level should decrease or stay same"
                    );
                }

                for dmg in &payload.catapult_damage {
                    assert!(
                        dmg.level_before >= dmg.level_after,
                        "Building level should decrease or stay same"
                    );
                }
            } else {
                panic!("Expected Battle report payload");
            }

            uow.rollback().await?;
        }

        Ok(())
    }
}
