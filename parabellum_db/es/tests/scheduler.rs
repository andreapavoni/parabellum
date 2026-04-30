use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::{
    ResearchAcademy, ResearchSmithy, SendMerchantsTransfer, SendReinforcement, TrainUnits,
};
use parabellum_types::{army::TroopSet, buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    academy, barracks, granary, main_building, marketplace, rally_point, resources, setup_village,
    smithy, warehouse, with_test_pool,
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

#[tokio::test]
async fn village_es_service_schedules_and_completes_merchant_trip() {
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
            vec![main_building(1), marketplace(2), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        setup_village(
            &pool,
            &service,
            Uuid::new_v4(),
            Uuid::new_v4(),
            101,
            "Village B",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(10_000, 10_000, 10_000, 10_000),
        )
        .await;

        let send = parabellum_types::common::ResourceGroup(200, 50, 120, 100);
        service
            .send_resources(
                100,
                &SendMerchantsTransfer {
                    player_id,
                    target_village_id: 101,
                    resources: send,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        let source_after_schedule = service.get_village_model(100).await.unwrap();
        assert_eq!(source_after_schedule.busy_merchants, 1);
        assert_eq!(source_after_schedule.stocks.lumber, 79_800);
        assert_eq!(source_after_schedule.stocks.clay, 79_950);
        assert_eq!(source_after_schedule.stocks.iron, 79_880);
        assert_eq!(source_after_schedule.stocks.crop, 79_900);

        let arrival_actions = service
            .get_village_scheduled_action_status_count(
                100,
                parabellum_app::villages::models::ScheduledActionType::MerchantsArrival,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(arrival_actions, 1);

        let due_arrival = chrono::Utc::now() + chrono::Duration::minutes(6);
        let processed_arrival = service.process_due_actions(due_arrival, 10).await.unwrap();
        assert_eq!(processed_arrival, 1);

        let target_after_arrival = service.get_village_model(101).await.unwrap();
        assert_eq!(target_after_arrival.stocks.lumber, 10_200);
        assert_eq!(target_after_arrival.stocks.clay, 10_050);
        assert_eq!(target_after_arrival.stocks.iron, 10_120);
        assert_eq!(target_after_arrival.stocks.crop, 10_100);

        let due_return = chrono::Utc::now() + chrono::Duration::minutes(15);
        let processed_return = service.process_due_actions(due_return, 10).await.unwrap();
        assert_eq!(processed_return, 1);
        let source_after_return = service.get_village_model(100).await.unwrap();
        assert_eq!(source_after_return.busy_merchants, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_scheduler_respects_due_time_and_avoids_duplicate_execution() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            player_id,
            user_id,
            900,
            "Timing Village",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), barracks(1), warehouse(20), granary(20)],
            resources(2_000, 2_000, 2_000, 2_000),
        )
        .await;

        service
            .train_units(
                900,
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

        let due_at = service
            .get_village_training_queue(900)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should contain one action");

        let processed_before_due = service
            .process_due_actions(due_at - chrono::Duration::milliseconds(1), 10)
            .await
            .unwrap();
        assert_eq!(
            processed_before_due, 0,
            "no actions must execute before execute_at"
        );

        let (first, second) = tokio::join!(
            service.process_due_actions(due_at + chrono::Duration::milliseconds(1), 10),
            service.process_due_actions(due_at + chrono::Duration::milliseconds(1), 10),
        );
        let processed_total = first.unwrap() + second.unwrap();
        assert_eq!(
            processed_total, 1,
            "concurrent schedulers must not execute same action twice"
        );

        let completed = service
            .get_village_scheduled_action_status_count(
                900,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let pending = service
            .get_village_scheduled_action_status_count(
                900,
                parabellum_app::villages::models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(completed, 1);
        assert_eq!(pending, 0);
    })
    .await;
}
