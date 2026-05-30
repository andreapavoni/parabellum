use parabellum_app::villages::models::{
    self, ScheduledAction, ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType,
};
use parabellum_app::ports::queries::TroopMovementType;
use parabellum_app::villages::repositories::ScheduledActionRepository;
use parabellum_app::villages::{
    AttackVillage, ResearchAcademy, ResearchSmithy, ScoutVillage, SendMerchantsTransfer,
    SendReinforcement, TrainUnits, UpgradeBuilding,
};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::{
    army::{TroopSet, UnitName},
    battle::AttackType,
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
};
use uuid::Uuid;

use crate::es::VillageEsService;
use crate::es::repositories::PostgresScheduledActionRepository;
use crate::es::tests::fixtures::setup_village_for_player;

use super::fixtures::{
    EsScenario, academy, barracks, deployed_units, granary, home_units,
    insert_corrupt_scheduled_action, main_building, marketplace, process_due_until, rally_point,
    refill_resources, research_and_complete, resources, scheduled_action_status_count,
    setup_village, smithy, stationed_units, train_and_complete, village_busy_merchants,
    village_owner, village_stocks, warehouse, with_test_pool,
};

fn minus(
    before: &parabellum_game::models::village::VillageStocks,
    cost: ResourceGroup,
) -> ResourceGroup {
    ResourceGroup::new(
        before.lumber.saturating_sub(cost.lumber()),
        before.clay.saturating_sub(cost.clay()),
        before.iron.saturating_sub(cost.iron()),
        (before.crop.max(0) as u32).saturating_sub(cost.crop()),
    )
}

#[tokio::test]
async fn village_es_service_scheduler_is_idempotent_and_lists_player_villages() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(1),
                rally_point(1),
                barracks(1),
                academy(20),
                warehouse(20),
                granary(20),
            ],
            resources(800_000, 800_000, 800_000, 800_000),
        )
        .await;
        let second_village_id = setup_village_for_player(
            &service,
            player_id,
            "Village B",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        train_and_complete(
            &service,
            village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            10,
        )
        .await;

        service
            .send_reinforcement(
                village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: second_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        let first_processed = process_due_until(
            &service,
            chrono::Utc::now() + chrono::Duration::minutes(10),
            10,
        )
        .await;
        assert_eq!(first_processed, 1);

        let second_processed = process_due_until(
            &service,
            chrono::Utc::now() + chrono::Duration::minutes(10),
            10,
        )
        .await;
        assert_eq!(second_processed, 0);

        let models = service.list_villages_by_player_id(player_id).await.unwrap();
        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|v| v.village_id == village_id));
        assert!(models.iter().any(|v| v.village_id == second_village_id));
    })
    .await;
}

#[tokio::test]
async fn scheduler_batch_failure_does_not_leave_actions_processing() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 41, y: 9 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), barracks(1), warehouse(20), granary(20)],
            resources(20_000, 20_000, 20_000, 20_000),
        )
        .await;

        service
            .train_units(
                village_id,
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

        insert_corrupt_scheduled_action(&pool, ScheduledActionStatus::Pending).await;

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 100)
            .await
            .unwrap();

        assert_eq!(
            service
                .get_village_scheduled_action_status_count(
                    village_id,
                    ScheduledActionType::TrainUnit,
                    ScheduledActionStatus::Completed,
                )
                .await
                .unwrap(),
            1,
        );
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Failed).await,
            1,
        );
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Processing).await,
            0,
        );
    })
    .await;
}

#[tokio::test]
async fn scheduler_requeues_stale_processing_actions() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        insert_corrupt_scheduled_action(&pool, ScheduledActionStatus::Processing).await;

        service
            .process_due_actions(chrono::Utc::now(), 100)
            .await
            .unwrap();

        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Processing).await,
            0,
        );
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Failed).await,
            1,
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_trains_units_in_batched_sequence() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
                "Village A",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), barracks(1), warehouse(20), granary(20)],
                resources(2_000, 2_000, 2_000, 2_000),
            )
            .await;

        let village_before_schedule = service.get_village(village_id).await.unwrap();
        let before_schedule = village_before_schedule.stocks;
        let tribe = village_before_schedule.tribe;
        let expected_cost = tribe.units()[0].cost.resources.clone() * 2.0;

        service
            .train_units(
                village_id,
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

        let after_schedule = service.get_village(village_id).await.unwrap().stocks;
        let expected_after = minus(&before_schedule, expected_cost);
        assert_eq!(after_schedule.lumber, expected_after.lumber());
        assert_eq!(after_schedule.clay, expected_after.clay());
        assert_eq!(after_schedule.iron, expected_after.iron());
        assert!(after_schedule.crop <= expected_after.crop() as i64);

        let first_due = service
            .get_village_training_queue(village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should contain scheduled actions");
        let first = scenario
            .process_until(first_due + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(first, 1);

        let training_counts_after_first = service
            .get_village_scheduled_action_status_counts(
                village_id,
                models::ScheduledActionType::TrainUnit,
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

        let queue_after_first = service
            .get_village_training_queue(village_id)
            .await
            .unwrap();
        assert_eq!(queue_after_first.len(), 1);

        let second_due = queue_after_first
            .iter()
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should have one pending action after first completion");
        let second = scenario
            .process_until(second_due + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(second, 1);
        let completed_training_after_second = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let pending_training_after_second = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(completed_training_after_second, 2);
        assert_eq!(pending_training_after_second, 0);

        assert_eq!(home_units(&pool, village_id, 0).await, 2);
        let home_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rm_armies WHERE village_id = $1 AND state = 'home'",
        )
        .bind(village_id as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            home_rows, 1,
            "training projection must keep exactly one canonical home army row"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_smithy_research() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
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
                resources(800_000, 800_000, 800_000, 800_000),
            )
            .await;

        let before_schedule = service.get_village(village_id).await.unwrap().stocks;
        let expected_cost = parabellum_game::models::smithy::smithy_upgrade_cost_for_unit(
            &parabellum_types::army::UnitName::Maceman,
            0,
        )
        .unwrap()
        .resources;

        service
            .research_smithy(
                village_id,
                &ResearchSmithy {
                    player_id,
                    unit: parabellum_types::army::UnitName::Maceman,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after_schedule = service.get_village(village_id).await.unwrap().stocks;
        let expected_after = minus(&before_schedule, expected_cost);
        assert_eq!(after_schedule.lumber, expected_after.lumber());
        assert_eq!(after_schedule.clay, expected_after.clay());
        assert_eq!(after_schedule.iron, expected_after.iron());
        assert!(after_schedule.crop <= expected_after.crop() as i64);

        let smithy_queue = service.get_village_smithy_queue(village_id).await.unwrap();
        assert_eq!(smithy_queue.len(), 1);

        let due_at = smithy_queue
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("smithy queue should contain one scheduled action");
        let smithy_processed = scenario
            .process_until(due_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(smithy_processed, 1);

        let completed_smithy = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::ResearchSmithy,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        assert_eq!(completed_smithy, 1);

        let model = service.get_village(village_id).await.unwrap();
        let hydrated = parabellum_game::models::village::Village::try_from(model).unwrap();
        let idx = hydrated
            .tribe
            .get_unit_idx_by_name(&parabellum_types::army::UnitName::Maceman)
            .unwrap();
        assert_eq!(hydrated.smithy()[idx], 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_academy_research() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
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
                resources(800_000, 800_000, 800_000, 800_000),
            )
            .await;

        let village_before_schedule = service.get_village(village_id).await.unwrap();
        let before_schedule = village_before_schedule.stocks;
        let tribe = village_before_schedule.tribe;
        let expected_cost = tribe
            .get_unit_by_name(&parabellum_types::army::UnitName::Spearman)
            .unwrap()
            .research_cost
            .resources
            .clone();

        service
            .research_academy(
                village_id,
                &ResearchAcademy {
                    player_id,
                    unit: parabellum_types::army::UnitName::Spearman,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after_schedule = service.get_village(village_id).await.unwrap().stocks;
        let expected_after = minus(&before_schedule, expected_cost);
        assert_eq!(after_schedule.lumber, expected_after.lumber());
        assert_eq!(after_schedule.clay, expected_after.clay());
        assert_eq!(after_schedule.iron, expected_after.iron());
        assert!(after_schedule.crop <= expected_after.crop() as i64);

        let academy_queue = service.get_village_academy_queue(village_id).await.unwrap();
        assert_eq!(academy_queue.len(), 1);

        let due_at = academy_queue
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("academy queue should contain one scheduled action");
        let academy_processed = scenario
            .process_until(due_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(academy_processed, 1);

        let academy_completed = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::ResearchAcademy,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let academy_pending = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::ResearchAcademy,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(academy_completed, 1);
        assert_eq!(academy_pending, 0);

        let model = service.get_village(village_id).await.unwrap();
        let hydrated = parabellum_game::models::village::Village::try_from(model).unwrap();
        let idx = hydrated
            .tribe
            .get_unit_idx_by_name(&parabellum_types::army::UnitName::Spearman)
            .unwrap();
        assert!(hydrated.academy_research().get(idx));
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_merchant_trip() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
                "Village A",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), marketplace(2), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;
        let (_user_id, _player_id, target_village_id) = scenario
            .village(
                "Village B",
                Position { x: 1, y: 1 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(10_000, 10_000, 10_000, 10_000),
            )
            .await;

        let send = parabellum_types::common::ResourceGroup(200, 50, 120, 100);
        service
            .send_resources(
                village_id,
                &SendMerchantsTransfer {
                    player_id,
                    target_village_id,
                    resources: send,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                    speed: parabellum_app::config::Config::from_env().speed,
                },
            )
            .await
            .unwrap();

        let source_after_schedule = village_stocks(&service, village_id).await;
        assert_eq!(village_busy_merchants(&service, village_id).await, 1);
        assert_eq!(source_after_schedule.lumber, 79_800);
        assert_eq!(source_after_schedule.clay, 79_950);
        assert_eq!(source_after_schedule.iron, 79_880);
        assert!(source_after_schedule.crop <= 79_900);

        let arrival_actions = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::MerchantsArrival,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(arrival_actions, 1);

        let due_arrival = chrono::Utc::now() + chrono::Duration::minutes(6);
        let processed_arrival = scenario.process_until(due_arrival, 10).await;
        assert_eq!(processed_arrival, 1);

        let target_after_arrival = village_stocks(&service, target_village_id).await;
        assert_eq!(target_after_arrival.lumber, 10_200);
        assert_eq!(target_after_arrival.clay, 10_050);
        assert_eq!(target_after_arrival.iron, 10_120);
        assert!(target_after_arrival.crop <= 10_100);

        let due_return = chrono::Utc::now() + chrono::Duration::minutes(15);
        let processed_return = scenario.process_until(due_return, 10).await;
        assert_eq!(processed_return, 1);
        assert_eq!(village_busy_merchants(&service, village_id).await, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_scheduler_respects_due_time_and_avoids_duplicate_execution() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
                "Timing Village",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), barracks(1), warehouse(20), granary(20)],
                resources(2_000, 2_000, 2_000, 2_000),
            )
            .await;

        service
            .train_units(
                village_id,
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
            .get_village_training_queue(village_id)
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
                village_id,
                models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Completed,
            )
            .await
            .unwrap();
        let pending = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::TrainUnit,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(completed, 1);
        assert_eq!(pending, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_chained_scheduling_deducts_cumulative_exact_costs() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);
        let (_user_id, player_id, village_id) = scenario
            .village(
                "Cost Chain Village",
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

        let before = service.get_village(village_id).await.unwrap();
        let maceman_train_cost = before.tribe.units()[0].cost.resources.clone();
        let academy_cost = before
            .tribe
            .get_unit_by_name(&UnitName::Spearman)
            .unwrap()
            .research_cost
            .resources
            .clone();
        let total_cost = ResourceGroup::new(
            maceman_train_cost.lumber() + academy_cost.lumber(),
            maceman_train_cost.clay() + academy_cost.clay(),
            maceman_train_cost.iron() + academy_cost.iron(),
            maceman_train_cost.crop() + academy_cost.crop(),
        );

        service
            .train_units(
                village_id,
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
            .research_academy(
                village_id,
                &ResearchAcademy {
                    player_id,
                    unit: UnitName::Spearman,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after = service.get_village(village_id).await.unwrap();
        let expected_after = minus(&before.stocks, total_cost);
        assert_eq!(after.stocks.lumber, expected_after.lumber());
        assert_eq!(after.stocks.clay, expected_after.clay());
        assert_eq!(after.stocks.iron, expected_after.iron());
        assert!(after.stocks.crop <= expected_after.crop() as i64);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_queued_upgrades_use_incremental_levels_and_exact_cumulative_cost() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let actions = PostgresScheduledActionRepository::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, village_id) = scenario
            .village(
                "Upgrade Queue Village",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Roman,
                vec![
                    main_building(20),
                    VillageBuilding {
                        slot_id: 22,
                        building: Building::new(BuildingName::Cranny, 1)
                            .at_level(1, 1)
                            .unwrap(),
                    },
                    warehouse(20),
                    granary(20),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        let before = service.get_village(village_id).await.unwrap();
        let cost_l2 = Building::new(BuildingName::Cranny, 1)
            .at_level(2, 1)
            .unwrap()
            .cost()
            .resources;
        let cost_l3 = Building::new(BuildingName::Cranny, 1)
            .at_level(3, 1)
            .unwrap()
            .cost()
            .resources;

        service
            .upgrade_building(
                village_id,
                &UpgradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .upgrade_building(
                village_id,
                &UpgradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let pending = actions
            .list_by_village_and_type(village_id, ScheduledActionType::UpgradeBuilding)
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let mut levels: Vec<u8> = pending
            .iter()
            .map(|a| {
                match serde_json::from_value::<ScheduledActionPayload>(a.payload.clone()).unwrap() {
                    ScheduledActionPayload::UpgradeBuilding { level, .. } => level,
                    _ => panic!("expected UpgradeBuilding payload"),
                }
            })
            .collect();
        levels.sort_unstable();
        assert_eq!(levels, vec![2, 3]);

        let after = service.get_village(village_id).await.unwrap();
        let total_cost = ResourceGroup::new(
            cost_l2.lumber() + cost_l3.lumber(),
            cost_l2.clay() + cost_l3.clay(),
            cost_l2.iron() + cost_l3.iron(),
            cost_l2.crop() + cost_l3.crop(),
        );
        let expected_after = minus(&before.stocks, total_cost);
        assert_eq!(after.stocks.lumber, expected_after.lumber());
        assert_eq!(after.stocks.clay, expected_after.clay());
        assert_eq!(after.stocks.iron, expected_after.iron());
        assert!(after.stocks.crop <= expected_after.crop() as i64);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_attack_arrival_and_return() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, village_id) = scenario
            .village(
                "Source",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![
                    main_building(1),
                    rally_point(1),
                    barracks(1),
                    academy(20),
                    warehouse(20),
                    granary(20),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;
        let target_village_id = scenario
            .village_for_player(
                player_id,
                "Target",
                Position { x: 3, y: 3 },
                parabellum_types::tribe::Tribe::Roman,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        train_and_complete(
            &service,
            village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            10,
        )
        .await;

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);
        let movement_id = Uuid::new_v4();
        let arrival_action_id = Uuid::new_v4();
        let return_action_id = Uuid::new_v4();

        service
            .send_attack(
                village_id,
                &AttackVillage {
                    movement_id,
                    arrival_action_id,
                    return_action_id,
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();

        assert_eq!(home_units(&pool, village_id, 0).await, 0);
        assert_eq!(deployed_units(&pool, village_id, 0).await, 0);

        let first_processed = scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(first_processed, 1);

        assert_eq!(home_units(&pool, village_id, 0).await, 0);
        assert_eq!(
            deployed_units(&pool, village_id, 0).await,
            0,
            "returning attack army must not be projected as deployed reinforcement"
        );
        let movements_after_arrival = service
            .get_village_troop_movements(village_id)
            .await
            .unwrap();
        assert_eq!(movements_after_arrival.incoming.len(), 1);
        assert_eq!(
            movements_after_arrival.incoming[0].tribe,
            parabellum_types::tribe::Tribe::Teuton,
            "returning attack army must keep the attacker's tribe, not the target village tribe"
        );
        assert!(movements_after_arrival.outgoing.is_empty());

        let second_processed = scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(second_processed, 1);

        assert_eq!(home_units(&pool, village_id, 0).await, 1);
        assert_eq!(deployed_units(&pool, village_id, 0).await, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_arrival_processes_and_schedules_return_action() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(20),
                rally_point(1),
                barracks(1),
                academy(20),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        service
            .train_units(
                village_id,
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
        let (_user_id, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(12), 500)
            .await
            .unwrap();

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);
        let source = service.get_village(village_id).await.unwrap();
        let mut arriving_army = source.army.clone().unwrap();
        arriving_army.update_units(&TroopSet::default());
        let arrival_action_id = Uuid::new_v4();
        let return_action_id = Uuid::new_v4();

        let payload = ScheduledActionPayload::AttackArrival {
            action_id: arrival_action_id,
            movement_id: Uuid::new_v4(),
            army_id: Uuid::new_v4(),
            return_action_id,
            village_id: village_id,
            source_village_id: village_id,
            target_village_id,
            player_id,
            army: arriving_army,
            attack_type: AttackType::Normal,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            arrives_at,
            returns_at,
        };
        let repo = PostgresScheduledActionRepository::new(pool.clone());
        repo.add(&ScheduledAction {
            id: arrival_action_id,
            action_type: ScheduledActionType::AttackArrival,
            execute_at: arrives_at,
            payload: serde_json::to_value(payload).unwrap(),
            status: ScheduledActionStatus::Pending,
        })
        .await
        .unwrap();

        let processed_arrival = service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(processed_arrival, 1);

        let pending_returns = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::ArmyReturn,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(pending_returns, 0);

        let processed_return_window = service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(processed_return_window, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_battle_keeps_reinforcement_owner_deployed_snapshot_aligned() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, attacker_village_id) = setup_village(
            &pool,
            &service,
            "Attacker",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
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
        let (_defender_user_id, _defender_player_id, defender_village_id) = setup_village(
            &pool,
            &service,
            "Defender",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_reinforcer_user_id, reinforcer_player_id, reinforcer_village_id) = setup_village(
            &pool,
            &service,
            "Reinforcer",
            Position { x: 4, y: 4 },
            parabellum_types::tribe::Tribe::Teuton,
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

        service
            .train_units(
                attacker_village_id,
                &TrainUnits {
                    player_id: attacker_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .train_units(
                reinforcer_village_id,
                &TrainUnits {
                    player_id: reinforcer_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 200)
            .await
            .unwrap();

        let reinforcement_arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        service
            .send_reinforcement(
                reinforcer_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: reinforcer_player_id,
                    target_village_id: defender_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: reinforcement_arrives_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(reinforcement_arrives_at + chrono::Duration::seconds(1), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_attack(
                attacker_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
                    target_village_id: defender_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 50)
            .await
            .unwrap();

        assert_eq!(
            deployed_units(&pool, reinforcer_village_id, 0).await,
            stationed_units(&pool, defender_village_id, 0).await,
            "owner deployed troops must match stationed troops on the target read model",
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_return_clamps_bounty_to_storage_capacity() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, village_id) = scenario
            .village(
                "Source",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), rally_point(1), barracks(1)],
                resources(800, 800, 800, 800),
            )
            .await;
        let target_village_id = scenario
            .village_for_player(
                player_id,
                "Target",
                Position { x: 1, y: 1 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1)],
                resources(800, 800, 800, 800),
            )
            .await;

        train_and_complete(
            &service,
            village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            20,
        )
        .await;

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);

        service
            .send_attack(
                village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Raid,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::MainBuilding],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();

        refill_resources(
            &service,
            village_id,
            player_id,
            resources(799, 799, 799, 799),
        )
        .await;

        scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;
        scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;

        let source_after_return = service.get_village(village_id).await.unwrap();
        assert!(source_after_return.stocks.lumber <= source_after_return.stocks.warehouse_capacity);
        assert!(source_after_return.stocks.clay <= source_after_return.stocks.warehouse_capacity);
        assert!(source_after_return.stocks.iron <= source_after_return.stocks.warehouse_capacity);
        assert!(
            source_after_return.stocks.crop.max(0) as u32
                <= source_after_return.stocks.granary_capacity
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_wipeout_skips_return_scheduling() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                barracks(1),
                academy(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), barracks(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                village_id,
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
        for _ in 0..100 {
            refill_resources(
                &service,
                target_village_id,
                player_id,
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;
            service
                .train_units(
                    target_village_id,
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
                .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(12), 20)
                .await
                .unwrap();
        }
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(12), 200)
            .await
            .unwrap();

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);
        service
            .send_attack(
                village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();

        let arrival_processed = service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(arrival_processed, 1);

        let pending_returns = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::ArmyReturn,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(pending_returns, 0);

        let movements_after_arrival = service
            .get_village_troop_movements(village_id)
            .await
            .unwrap();
        assert!(
            movements_after_arrival.outgoing.is_empty()
                && movements_after_arrival.incoming.is_empty(),
            "wipeout should not leave lingering troop movements"
        );

        assert_eq!(
            deployed_units(&pool, village_id, 0).await,
            0,
            "wipeout should not leave deployed clubs"
        );
        assert_eq!(
            deployed_units(&pool, village_id, 8).await,
            0,
            "wipeout should not leave deployed senators"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_bounty_respects_source_capacity() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, village_id) = scenario
            .village(
                "Source",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), rally_point(1), barracks(1)],
                resources(799, 799, 799, 799),
            )
            .await;
        let target_village_id = scenario
            .village_for_player(
                player_id,
                "Target",
                Position { x: 2, y: 2 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1)],
                resources(800, 800, 800, 800),
            )
            .await;

        train_and_complete(
            &service,
            village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(24),
            500,
        )
        .await;

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);
        service
            .send_attack(
                village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Raid,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();
        scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;
        scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;

        let after = service.get_village(village_id).await.unwrap();
        assert!(after.stocks.lumber <= 800);
        assert!(after.stocks.clay <= 800);
        assert!(after.stocks.iron <= 800);
        assert!(after.stocks.crop <= 800);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_scout_arrival_and_return() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, village_id) = scenario
            .village(
                "Scout Source",
                Position { x: 0, y: 0 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![
                    main_building(20),
                    rally_point(10),
                    barracks(1),
                    academy(20),
                    warehouse(20),
                    granary(20),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;
        let target_village_id = scenario
            .village_for_player(
                player_id,
                "Scout Target",
                Position { x: 3, y: 3 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        research_and_complete(
            &service,
            village_id,
            player_id,
            UnitName::Scout,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            20,
        )
        .await;
        train_and_complete(
            &service,
            village_id,
            player_id,
            3,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            20,
        )
        .await;

        let now = chrono::Utc::now();
        let arrives_at = now + chrono::Duration::seconds(2);
        let returns_at = now + chrono::Duration::seconds(4);

        service
            .send_scout(
                village_id,
                &ScoutVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 1, 0, 0, 0, 0, 0, 0]),
                    target: parabellum_types::battle::ScoutingTarget::Resources,
                    attack_type: AttackType::Raid,
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();

        let movements_after_scout_send = service.get_village_troop_movements(village_id).await.unwrap();
        assert_eq!(movements_after_scout_send.outgoing.len(), 1);
        assert_eq!(
            movements_after_scout_send.outgoing[0].movement_type,
            TroopMovementType::Scout
        );

        let first_processed = scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(first_processed, 1);
        assert_eq!(
            deployed_units(&pool, village_id, 3).await,
            0,
            "returning scout army must not be projected as deployed reinforcement"
        );
        let movements_after_scout_arrival = service
            .get_village_troop_movements(village_id)
            .await
            .unwrap();
        assert_eq!(movements_after_scout_arrival.incoming.len(), 1);
        assert!(movements_after_scout_arrival.outgoing.is_empty());

        let second_processed = scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;
        assert_eq!(second_processed, 1);

        assert_eq!(home_units(&pool, village_id, 3).await, 1);
        assert_eq!(deployed_units(&pool, village_id, 3).await, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_conquer_is_blocked_without_expansion_prerequisites() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "No-Prereq Source",
            Position { x: 4, y: 4 },
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
        let (_target_user_id, target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "No-Prereq Target",
            Position { x: 6, y: 6 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        train_and_complete(
            &service,
            source_village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            20,
        )
        .await;

        let now = chrono::Utc::now();
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at: now + chrono::Duration::seconds(2),
                    returns_at: now + chrono::Duration::seconds(5),
                },
            )
            .await
            .unwrap();

        service
            .process_due_actions(now + chrono::Duration::seconds(3), 20)
            .await
            .unwrap();

        assert_eq!(
            village_owner(&service, target_village_id).await,
            target_player_id
        );
        assert_ne!(village_owner(&service, target_village_id).await, player_id);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_conquer_consumes_only_one_surviving_chief_unit() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let scenario = EsScenario::new(&pool, &service);

        let (_user_id, player_id, source_village_id) = scenario
            .village(
                "Conquer Source",
                Position { x: 10, y: 10 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![
                    main_building(1),
                    rally_point(10),
                    academy(20),
                    VillageBuilding {
                        slot_id: 26,
                        building: Building::new(BuildingName::Palace, 1)
                            .at_level(20, 1)
                            .unwrap(),
                    },
                    warehouse(20),
                    granary(20),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        let (_target_user_id, _target_player_id, target_village_id) = scenario
            .village(
                "Conquer Target",
                Position { x: 12, y: 10 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        sqlx::query("UPDATE rm_village SET is_capital = FALSE WHERE village_id = $1")
            .bind(target_village_id as i32)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE players SET culture_points = 5000 WHERE id = $1")
            .bind(player_id)
            .execute(&pool)
            .await
            .unwrap();

        research_and_complete(
            &service,
            source_village_id,
            player_id,
            UnitName::Chief,
            1,
            chrono::Utc::now() + chrono::Duration::days(7),
            400,
        )
        .await;
        refill_resources(
            &service,
            source_village_id,
            player_id,
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        train_and_complete(
            &service,
            source_village_id,
            player_id,
            8,
            BuildingName::Palace,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::days(7),
            400,
        )
        .await;
        refill_resources(
            &service,
            source_village_id,
            player_id,
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        train_and_complete(
            &service,
            source_village_id,
            player_id,
            8,
            BuildingName::Palace,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::days(7),
            400,
        )
        .await;
        assert_eq!(home_units(&pool, source_village_id, 8).await, 2);

        let first_now = chrono::Utc::now();
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 2, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at: first_now + chrono::Duration::seconds(2),
                    returns_at: first_now + chrono::Duration::seconds(4),
                },
            )
            .await
            .unwrap();
        scenario
            .process_until(first_now + chrono::Duration::seconds(3), 20)
            .await;
        scenario
            .process_until(first_now + chrono::Duration::seconds(5), 20)
            .await;

        let second_now = chrono::Utc::now();
        sqlx::query("UPDATE rm_village SET loyalty = 30 WHERE village_id = $1")
            .bind(target_village_id as i32)
            .execute(&pool)
            .await
            .unwrap();
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 2, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at: second_now + chrono::Duration::seconds(2),
                    returns_at: second_now + chrono::Duration::seconds(4),
                },
            )
            .await
            .unwrap();
        scenario
            .process_until(second_now + chrono::Duration::seconds(3), 20)
            .await;

        assert_eq!(village_owner(&service, target_village_id).await, player_id);
        assert_eq!(
            stationed_units(&pool, target_village_id, 8).await,
            1,
            "exactly one surviving chief unit must be consumed on successful conquer"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_loyalty_regenerates_with_residence_over_time() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let action_repo = PostgresScheduledActionRepository::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Loyalty Regen",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(10),
                VillageBuilding {
                    slot_id: 22,
                    building: Building::new(BuildingName::Residence, 1)
                        .at_level(1, 1)
                        .unwrap(),
                },
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        sqlx::query("UPDATE rm_village SET loyalty = 80 WHERE village_id = $1")
            .bind(village_id as i32)
            .execute(&pool)
            .await
            .unwrap();

        let action_id = Uuid::new_v4();
        let execute_at = chrono::Utc::now();
        let payload = ScheduledActionPayload::LoyaltyRegen {
            action_id,
            village_id,
            player_id,
            execute_at,
        };
        action_repo
            .add(&ScheduledAction {
                id: action_id,
                action_type: payload.action_type(),
                execute_at,
                payload: serde_json::to_value(payload).unwrap(),
                status: ScheduledActionStatus::Pending,
            })
            .await
            .unwrap();

        let pending_regen = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::LoyaltyRegen,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(pending_regen, 1);

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(24), 50)
            .await
            .unwrap();

        let after_tick = service.get_village(village_id).await.unwrap();
        assert_eq!(after_tick.loyalty, 82);

        let pending_next = service
            .get_village_scheduled_action_status_count(
                village_id,
                models::ScheduledActionType::LoyaltyRegen,
                ScheduledActionStatus::Pending,
            )
            .await
            .unwrap();
        assert_eq!(pending_next, 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_horse_drinking_trough_thresholds_reduce_cavalry_upkeep() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        async fn run_threshold_case(
            pool: &sqlx::PgPool,
            service: &VillageEsService,
            unit: UnitName,
            trough_before: u8,
            x: i32,
        ) -> (u32, u32) {
            let (_user_id, player_id, village_id) = setup_village(
                pool,
                service,
                "Roman Cavalry",
                Position { x, y: 30 },
                parabellum_types::tribe::Tribe::Roman,
                vec![
                    main_building(10),
                    rally_point(10),
                    academy(20),
                    VillageBuilding {
                        slot_id: 21,
                        building: Building::new(BuildingName::Stable, 1)
                            .at_level(20, 1)
                            .unwrap(),
                    },
                    VillageBuilding {
                        slot_id: 20,
                        building: Building::new(BuildingName::HorseDrinkingTrough, 1)
                            .at_level(trough_before, 1)
                            .unwrap(),
                    },
                    warehouse(20),
                    granary(20),
                ],
                resources(800_000, 800_000, 800_000, 800_000),
            )
            .await;

            research_and_complete(
                service,
                village_id,
                player_id,
                unit.clone(),
                1,
                chrono::Utc::now() + chrono::Duration::days(3),
                200,
            )
            .await;
            refill_resources(
                service,
                village_id,
                player_id,
                resources(800_000, 800_000, 800_000, 800_000),
            )
            .await;

            let tribe = service.get_village(village_id).await.unwrap().tribe;
            let unit_idx = tribe.get_unit_idx_by_name(&unit).unwrap() as u8;
            train_and_complete(
                service,
                village_id,
                player_id,
                unit_idx,
                BuildingName::Stable,
                1,
                1,
                chrono::Utc::now() + chrono::Duration::days(3),
                200,
            )
            .await;

            let before = service.get_village(village_id).await.unwrap();
            let upkeep_before = before.production.upkeep.saturating_sub(before.population);

            refill_resources(
                service,
                village_id,
                player_id,
                resources(800_000, 800_000, 800_000, 800_000),
            )
            .await;
            service
                .upgrade_building(
                    village_id,
                    &UpgradeBuilding {
                        player_id,
                        slot_id: 20,
                        speed: 1,
                    },
                )
                .await
                .unwrap();
            service
                .process_due_actions(chrono::Utc::now() + chrono::Duration::days(5), 200)
                .await
                .unwrap();

            let after = service.get_village(village_id).await.unwrap();
            let upkeep_after = after.production.upkeep.saturating_sub(after.population);
            (upkeep_before, upkeep_after)
        }

        let (legati_before, legati_after) =
            run_threshold_case(&pool, &service, UnitName::EquitesLegati, 9, 30).await;
        assert_eq!(legati_before, legati_after + 1);
    })
    .await;
}
