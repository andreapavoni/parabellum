use std::sync::Arc;

use chrono::{Duration, Utc};

use parabellum_types::errors::{ApplicationError, GameError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::TrainUnits},
    jobs::{Job, JobPayload, tasks::TrainUnitsTask},
    uow::UnitOfWork,
};

pub struct TrainUnitsCommandHandler {}

impl Default for TrainUnitsCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainUnitsCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<TrainUnits> for TrainUnitsCommandHandler {
    async fn handle(
        &self,
        command: TrainUnits,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let mut village = village_repo.get_by_id(command.village_id).await?;

        if village.player_id != command.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: command.village_id,
                player_id: command.player_id,
            }));
        }

        let (slot_id, unit_name, time_per_unit) = village.init_unit_training(
            command.unit_idx,
            &command.building_name,
            command.quantity,
            config.speed,
        )?;
        village_repo.save(&village).await?;

        let active_jobs = job_repo
            .list_active_jobs_by_village(command.village_id as i32)
            .await?;
        let training_jobs: Vec<Job> = active_jobs
            .into_iter()
            .filter(|job| job.task.task_type == "TrainUnits")
            .collect();

        let payload = TrainUnitsTask {
            slot_id,
            unit: unit_name,
            quantity: command.quantity,
            time_per_unit: time_per_unit as i32,
        };

        let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&payload)?);

        let first_completion =
            Self::initial_completion_deadline(slot_id, time_per_unit, &training_jobs);

        let new_job = Job::with_deadline(
            command.player_id,
            command.village_id as i32,
            job_payload,
            first_completion,
        );

        job_repo.add(&new_job).await?;

        Ok(())
    }
}

impl TrainUnitsCommandHandler {
    fn initial_completion_deadline(
        slot_id: u8,
        time_per_unit: u32,
        jobs: &[Job],
    ) -> chrono::DateTime<Utc> {
        let mut slot_free_at = Utc::now();

        let mut slot_jobs = jobs
            .iter()
            .filter_map(|job| {
                if job.task.task_type != "TrainUnits" {
                    return None;
                }

                let payload: TrainUnitsTask = serde_json::from_value(job.task.data.clone()).ok()?;

                if payload.slot_id != slot_id {
                    return None;
                }

                Some((job.completed_at, payload.quantity, payload.time_per_unit))
            })
            .collect::<Vec<_>>();

        slot_jobs.sort_by_key(|(completed_at, ..)| *completed_at);

        for (completed_at, quantity, per_unit_time) in slot_jobs {
            if completed_at > slot_free_at {
                slot_free_at = completed_at;
            }

            let remaining = quantity.saturating_sub(1) as i64;
            if remaining > 0 {
                slot_free_at += Duration::seconds(remaining * per_unit_time as i64);
            }
        }

        if slot_free_at < Utc::now() {
            slot_free_at = Utc::now();
        }

        slot_free_at + Duration::seconds(time_per_unit as i64)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
            village_factory,
        },
    };
    use parabellum_types::Result;
    use parabellum_types::{
        army::UnitName,
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::test_utils::tests::{MockUnitOfWork, set_village_resources};

    fn setup_village_with_barracks() -> Result<(Player, Village, Arc<Config>)> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });
        village.set_academy_research_for_test(&UnitName::Legionnaire, true);

        let barracks =
            Building::new(BuildingName::Barracks, config.speed).at_level(10, config.speed)?;
        village.add_building_at_slot(barracks, 20)?;
        set_village_resources(&mut village, ResourceGroup(800, 800, 800, 800));

        Ok((player, village, config))
    }

    fn setup_village_with_stable() -> Result<(Player, Village, Arc<Config>)> {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(20, config.speed)?;
        village.add_building_at_slot(granary, 20)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(20, config.speed)?;
        village.add_building_at_slot(warehouse, 21)?;

        let stable = Building::new(BuildingName::Stable, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(stable, 22)?;
        village.set_academy_research_for_test(&UnitName::Pathfinder, true);
        set_village_resources(&mut village, ResourceGroup(10000, 10000, 10000, 10000));

        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_train_units_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        village.set_academy_research_for_test(&UnitName::Praetorian, true);
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 0,
            quantity: 5,
            building_name: BuildingName::Barracks,
        };

        handler.handle(command, &mock_uow, &config).await?;

        let saved_villages = village_repo.list_by_player_id(player.id).await?;
        assert_eq!(saved_villages.len(), 1, "Village should be saved once");
        let saved_village = &saved_villages[0];

        assert_eq!(
            saved_village.stored_resources().lumber(),
            800 - (120 * 5),
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            800 - (100 * 5),
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().iron(),
            800 - (150 * 5),
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stored_resources().crop(),
            800 - (30 * 5),
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        assert_eq!(
            job.task.task_type, "TrainUnits",
            "Job task is not TrainUnitsTask"
        );

        let task: TrainUnitsTask = serde_json::from_value(job.task.data.clone())
            .expect("Failed to deserialize job task data");

        assert_eq!(task.unit, UnitName::Legionnaire);
        assert_eq!(task.quantity, 5);
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_not_enough_resources() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        set_village_resources(&mut village, ResourceGroup(10, 0, 0, 0));
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 0,
            quantity: 10,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(job_repo.list_by_player_id(player.id).await?.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();
        let (player, mut village, config) = setup_village_with_barracks()?;

        village.remove_building_at_slot(20, config.speed)?;
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();
        let command = TrainUnits {
            player_id: player.id,
            village_id: village.id,
            unit_idx: 0,
            quantity: 1,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Barracks at level 1"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_research() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());

        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        village.set_academy_research_for_test(&UnitName::Praetorian, false);
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();
        let command = TrainUnits {
            player_id: player.id,
            village_id: village.id,
            unit_idx: 1,
            quantity: 1,
            building_name: BuildingName::Barracks,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Unit Praetorian not yet researched in Academy"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_stable_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, village, config) = setup_village_with_stable()?;
        let village_id = village.id;
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();

        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 2,
            quantity: 5,
            building_name: BuildingName::Stable,
        };

        // Pathfinder: 170, 150, 20, 40
        let unit_cost = ResourceGroup(170, 150, 20, 40);
        let total_cost = ResourceGroup(
            unit_cost.0 * 5,
            unit_cost.1 * 5,
            unit_cost.2 * 5,
            unit_cost.3 * 5,
        );
        let initial_lumber = village.stored_resources().lumber();

        handler.handle(command, &mock_uow, &config).await?;
        let saved_village = village_repo.get_by_id(village_id).await?;
        assert_eq!(
            saved_village.stored_resources().lumber(),
            initial_lumber - total_cost.0,
            "Lumber not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            10000 - total_cost.1
        );

        let added_jobs = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 1, "Expected a job for stable");

        let job = &added_jobs[0];
        assert_eq!(job.task.task_type, "TrainUnits");

        let task: TrainUnitsTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.unit, UnitName::Pathfinder, "Expected unit trained");
        assert_eq!(task.quantity, 5);
        assert_eq!(
            task.slot_id, 22,
            "Task should be linked to the right slot_id"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_allows_cross_slot_jobs() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_barracks()?;
        village.set_academy_research_for_test(&UnitName::Praetorian, true);
        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let base_payload = TrainUnitsTask {
            slot_id: 20,
            unit: UnitName::Legionnaire,
            quantity: 1,
            time_per_unit: 30,
        };
        let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&base_payload)?);
        let first_job = Job::new(player.id, village_id as i32, 30, job_payload.clone());
        job_repo.add(&first_job).await?;

        let second_payload = TrainUnitsTask {
            slot_id: 21,
            ..base_payload
        };
        let second_job_payload =
            JobPayload::new("TrainUnits", serde_json::to_value(&second_payload)?);
        let second_job = Job::new(player.id, village_id as i32, 30, second_job_payload);
        job_repo.add(&second_job).await?;

        let handler = TrainUnitsCommandHandler::new();
        let command = TrainUnits {
            player_id: player.id,
            village_id,
            unit_idx: 0,
            quantity: 1,
            building_name: BuildingName::Barracks,
        };

        handler.handle(command, &mock_uow, &config).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_allows_other_buildings() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, mut village, config) = setup_village_with_stable()?;
        let village_id = village.id;
        let barracks = Building::new(BuildingName::Barracks, config.speed)
            .at_level(5, config.speed)
            .unwrap();
        village.add_building_at_slot(barracks, 20)?;
        village.set_academy_research_for_test(&UnitName::Pathfinder, true);
        set_village_resources(&mut village, ResourceGroup(800, 800, 800, 800));
        village_repo.save(&village).await.unwrap();

        let payload = TrainUnitsTask {
            slot_id: 20,
            unit: UnitName::Legionnaire,
            quantity: 1,
            time_per_unit: 30,
        };
        let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&payload)?);
        let existing_job = Job::new(player.id, village_id as i32, 30, job_payload.clone());
        job_repo.add(&existing_job).await?;
        let second_job = Job::new(player.id, village_id as i32, 30, job_payload);
        job_repo.add(&second_job).await?;

        let handler = TrainUnitsCommandHandler::new();

        handler
            .handle(
                TrainUnits {
                    player_id: player.id,
                    village_id,
                    unit_idx: 2,
                    quantity: 1,
                    building_name: BuildingName::Stable,
                },
                &mock_uow,
                &config,
            )
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_handler_respects_existing_queue() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let job_repo = mock_uow.jobs();
        let village_repo = mock_uow.villages();

        let (player, village, config) = setup_village_with_barracks()?;
        let village_id = village.id;
        village_repo.save(&village).await?;

        let handler = TrainUnitsCommandHandler::new();

        handler
            .handle(
                TrainUnits {
                    player_id: player.id,
                    village_id,
                    unit_idx: 0,
                    quantity: 1,
                    building_name: BuildingName::Barracks,
                },
                &mock_uow,
                &config,
            )
            .await?;

        let jobs_after_first = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(jobs_after_first.len(), 1);
        let first_job = jobs_after_first[0].clone();
        let first_task: TrainUnitsTask = serde_json::from_value(first_job.task.data.clone())?;

        handler
            .handle(
                TrainUnits {
                    player_id: player.id,
                    village_id,
                    unit_idx: 0,
                    quantity: 2,
                    building_name: BuildingName::Barracks,
                },
                &mock_uow,
                &config,
            )
            .await?;

        let jobs_after_second = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(jobs_after_second.len(), 2);
        let second_job = jobs_after_second[1].clone();

        let delta = (second_job.completed_at - first_job.completed_at).num_seconds();
        let expected_offset = (first_task.time_per_unit as i64) * (first_task.quantity as i64);

        assert!(
            delta >= expected_offset,
            "Second training job should start after the queued duration ({} >= {})",
            delta,
            expected_offset
        );

        handler
            .handle(
                TrainUnits {
                    player_id: player.id,
                    village_id,
                    unit_idx: 0,
                    quantity: 1,
                    building_name: BuildingName::Barracks,
                },
                &mock_uow,
                &config,
            )
            .await?;
        let jobs_after_third = job_repo.list_by_player_id(player.id).await?;
        assert_eq!(
            jobs_after_third.len(),
            3,
            "Third queue item should be accepted"
        );

        Ok(())
    }
}
