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
                attacker_reports.iter().any(|r| r.read_at.is_some()),
                "Attacker report should be marked read"
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
}
