use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::{ResearchAcademy, ResearchSmithy, SendReinforcement, TrainUnits};
use parabellum_types::{army::TroopSet, buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    academy, barracks, found_village_cmd, granary, main_building, seed_player_and_village,
    setup_scheduler_village, smithy, stocks_for_research, stocks_for_training, warehouse,
    with_test_pool,
};

#[tokio::test]
async fn village_es_service_scheduler_is_idempotent_and_lists_player_villages() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        seed_player_and_village(&pool, player_id, user_id, 100, "Village A", 0, 0).await;
        seed_player_and_village(&pool, player_id, user_id, 101, "Village B", 1, 1).await;

        let service = VillageEsService::new(pool.clone());
        for (id, name, pos) in [
            (100u32, "Village A", Position { x: 0, y: 0 }),
            (101u32, "Village B", Position { x: 1, y: 1 }),
        ] {
            service
                .found_village(id, &found_village_cmd(player_id, name, pos))
                .await
                .unwrap();
        }

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
        setup_scheduler_village(
            &pool,
            &service,
            player_id,
            user_id,
            100,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), barracks(1)],
            parabellum_types::army::TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            stocks_for_training(),
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
        let training_counts_after_second = service
            .get_village_scheduled_action_status_counts(
                100,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                None,
            )
            .await
            .unwrap();
        assert_eq!(training_counts_after_second.completed, 2);
        assert_eq!(training_counts_after_second.pending, 0);

        let village = service.get_village_model(100).await.unwrap();
        assert_eq!(village.stationed_army.get(0), 2);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_smithy_research() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_scheduler_village(
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
            parabellum_types::army::TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            stocks_for_research(),
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

        let smithy_counts = service
            .get_village_scheduled_action_status_counts(
                100,
                parabellum_app::villages::models::ScheduledActionType::ResearchSmithy,
                None,
            )
            .await
            .unwrap();
        assert_eq!(smithy_counts.completed, 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_academy_research() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_scheduler_village(
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
            parabellum_types::army::TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            stocks_for_research(),
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

        let academy_completed_counts = service
            .get_village_scheduled_action_status_counts(
                100,
                parabellum_app::villages::models::ScheduledActionType::ResearchAcademy,
                Some(ScheduledActionStatus::Completed),
            )
            .await
            .unwrap();
        assert_eq!(academy_completed_counts.completed, 1);
        assert_eq!(academy_completed_counts.pending, 0);
    })
    .await;
}
