use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::{ResearchAcademy, ResearchSmithy, SendReinforcement, TrainUnits};
use parabellum_types::{army::TroopSet, buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    academy, barracks, granary, main_building, rally_point, resources, setup_village, smithy,
    warehouse, with_test_pool,
};

#[tokio::test]
async fn village_es_service_scheduler_is_idempotent_and_lists_player_villages() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            100,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                barracks(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            101,
            "Village B",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        service
            .train_units(
                100,
                &TrainUnits {
                    player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        service
            .send_reinforcement(
                100,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: 101,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        let first_processed = service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        assert_eq!(first_processed, 1);

        let second_processed = service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        assert_eq!(second_processed, 0);

        let models = service
            .list_village_models_by_player_id(player_id)
            .await
            .unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].village_id, 100);
        assert_eq!(models[1].village_id, 101);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_trains_units_in_batched_sequence() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            100,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), barracks(1), warehouse(20), granary(20)],
            resources(2_000, 2_000, 2_000, 2_000),
        )
        .await;

        service
            .train_units(
                100,
                &TrainUnits {
                    player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let first_due = service
            .get_village_training_queue(100)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should contain scheduled actions");
        let first = service
            .process_due_actions(first_due + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(first, 1);

        let training_counts_after_first = service
            .get_village_scheduled_action_status_counts(
                100,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            training_counts_after_first.pending + training_counts_after_first.completed,
            2
        );
        assert_eq!(training_counts_after_first.completed, 1);
        assert_eq!(training_counts_after_first.pending, 1);

        let queue_after_first = service.get_village_training_queue(100).await.unwrap();
        assert_eq!(queue_after_first.len(), 2);
        assert!(queue_after_first[0].execute_at <= queue_after_first[1].execute_at);

        let second_due = queue_after_first
            .iter()
            .filter(|a| a.status == ScheduledActionStatus::Pending)
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should have one pending action after first completion");
        let second = service
            .process_due_actions(second_due + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(second, 1);
        let completed_training_after_second = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let pending_training_after_second = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(completed_training_after_second, 2);
        assert_eq!(pending_training_after_second, 0);

        let village = service.get_village_model(100).await.unwrap();
        assert_eq!(village.army.get(0), 2);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_smithy_research() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            100,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(1),
                barracks(1),
                smithy(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .research_smithy(
                100,
                &ResearchSmithy {
                    player_id,
                    unit: parabellum_types::army::UnitName::Maceman,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let smithy_queue = service.get_village_smithy_queue(100).await.unwrap();
        assert_eq!(smithy_queue.len(), 1);

        let due_at = smithy_queue
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("smithy queue should contain one scheduled action");
        let smithy_processed = service
            .process_due_actions(due_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(smithy_processed, 1);

        let completed_smithy = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::ResearchSmithy,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        assert_eq!(completed_smithy, 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_academy_research() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            100,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(1),
                barracks(3),
                academy(1),
                smithy(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .research_academy(
                100,
                &ResearchAcademy {
                    player_id,
                    unit: parabellum_types::army::UnitName::Spearman,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let academy_queue = service.get_village_academy_queue(100).await.unwrap();
        assert_eq!(academy_queue.len(), 1);

        let due_at = academy_queue
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("academy queue should contain one scheduled action");
        let academy_processed = service
            .process_due_actions(due_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(academy_processed, 1);

        let academy_completed = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::ResearchAcademy,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let academy_pending = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::ResearchAcademy,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(academy_completed, 1);
        assert_eq!(academy_pending, 0);
    })
    .await;
}
