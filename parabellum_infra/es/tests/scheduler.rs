use parabellum_app::villages::models::{
    self, ScheduledAction, ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType,
};
use parabellum_app::villages::repositories::ScheduledActionRepository;
use parabellum_app::villages::{
    AttackVillage, ResearchAcademy, ResearchSmithy, ScoutVillage, SendMerchantsTransfer,
    SendReinforcement, SetVillageResources, TrainUnits, UpgradeBuilding,
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
    academy, barracks, granary, main_building, marketplace, rally_point, resources, setup_village,
    smithy, warehouse, with_test_pool,
};

fn troops_sum(armies: &[parabellum_game::models::army::Army], idx: usize) -> u32 {
    armies.iter().map(|a| a.units().get(idx)).sum()
}

fn army_units(v: &models::VillageModel, idx: usize) -> u32 {
    v.army.as_ref().map(|a| a.units().get(idx)).unwrap_or(0)
}

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
            resources(80_000, 80_000, 80_000, 80_000),
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

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

        let models = service.list_villages_by_player_id(player_id).await.unwrap();
        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|v| v.village_id == village_id));
        assert!(models.iter().any(|v| v.village_id == second_village_id));
    })
    .await;
}

#[tokio::test]
async fn village_es_service_trains_units_in_batched_sequence() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), barracks(1), warehouse(20), granary(20)],
            resources(2_000, 2_000, 2_000, 2_000),
        )
        .await;

        let before_schedule = service.get_village(village_id).await.unwrap().stocks;
        let tribe = service.get_village(village_id).await.unwrap().tribe;
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
        assert_eq!(after_schedule.crop, expected_after.crop() as i64);

        let first_due = service
            .get_village_training_queue(village_id)
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
        let second = service
            .process_due_actions(second_due + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
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

        let village = service.get_village(village_id).await.unwrap();
        assert_eq!(army_units(&village, 0), 2);
        let army_view = service
            .get_village_army_state_view(village_id)
            .await
            .unwrap();
        assert_eq!(
            army_view
                .home_army
                .as_ref()
                .map(|a| a.units().get(0))
                .unwrap_or(0),
            2
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_and_completes_smithy_research() {
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
                barracks(1),
                smithy(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
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
        assert_eq!(after_schedule.crop, expected_after.crop() as i64);

        let smithy_queue = service.get_village_smithy_queue(village_id).await.unwrap();
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
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
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

        let before_schedule = service.get_village(village_id).await.unwrap().stocks;
        let tribe = service.get_village(village_id).await.unwrap().tribe;
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
        assert_eq!(after_schedule.crop, expected_after.crop() as i64);

        let academy_queue = service.get_village_academy_queue(village_id).await.unwrap();
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
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), marketplace(2), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_user_id, _player_id, target_village_id) = setup_village(
            &pool,
            &service,
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
                },
            )
            .await
            .unwrap();

        let source_after_schedule = service.get_village(village_id).await.unwrap();
        assert_eq!(source_after_schedule.busy_merchants, 1);
        assert_eq!(source_after_schedule.stocks.lumber, 79_800);
        assert_eq!(source_after_schedule.stocks.clay, 79_950);
        assert_eq!(source_after_schedule.stocks.iron, 79_880);
        assert_eq!(source_after_schedule.stocks.crop, 79_900);

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
        let processed_arrival = service.process_due_actions(due_arrival, 10).await.unwrap();
        assert_eq!(processed_arrival, 1);

        let target_after_arrival = service.get_village(target_village_id).await.unwrap();
        assert_eq!(target_after_arrival.stocks.lumber, 10_200);
        assert_eq!(target_after_arrival.stocks.clay, 10_050);
        assert_eq!(target_after_arrival.stocks.iron, 10_120);
        assert_eq!(target_after_arrival.stocks.crop, 10_100);

        let due_return = chrono::Utc::now() + chrono::Duration::minutes(15);
        let processed_return = service.process_due_actions(due_return, 10).await.unwrap();
        assert_eq!(processed_return, 1);
        let source_after_return = service.get_village(village_id).await.unwrap();
        assert_eq!(source_after_return.busy_merchants, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_scheduler_respects_due_time_and_avoids_duplicate_execution() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
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
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Cost Chain Village",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(1),
                barracks(3),
                academy(1),
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
        assert_eq!(after.stocks.crop, expected_after.crop() as i64);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_queued_upgrades_use_incremental_levels_and_exact_cumulative_cost() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let actions = PostgresScheduledActionRepository::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
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
        assert_eq!(after.stocks.crop, expected_after.crop() as i64);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_schedules_attack_arrival_and_return() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
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
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 3, y: 3 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
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
        let first_due = service
            .get_village_training_queue(village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .min()
            .expect("training queue should contain scheduled actions");
        service
            .process_due_actions(first_due + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

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

        let before_arrival = service.get_village(village_id).await.unwrap();
        assert_eq!(army_units(&before_arrival, 0), 0);
        assert_eq!(troops_sum(&before_arrival.deployed_armies, 0), 0);

        let first_processed = service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(first_processed, 1);

        let after_arrival = service.get_village(village_id).await.unwrap();
        assert_eq!(army_units(&after_arrival, 0), 0);
        assert_eq!(troops_sum(&after_arrival.deployed_armies, 0), 1);

        let second_processed = service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(second_processed, 1);

        let after_return = service.get_village(village_id).await.unwrap();
        assert_eq!(army_units(&after_return, 0), 1);
        assert_eq!(troops_sum(&after_return.deployed_armies, 0), 0);
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
async fn village_es_service_attack_return_clamps_bounty_to_storage_capacity() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), rally_point(1), barracks(1)],
            resources(800, 800, 800, 800),
        )
        .await;
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
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
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 20)
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
                    attack_type: AttackType::Raid,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::MainBuilding],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();

        service
            .set_village_resources(
                village_id,
                &SetVillageResources {
                    player_id,
                    resources: resources(799, 799, 799, 799),
                },
            )
            .await
            .unwrap();

        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

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
        service
            .train_units(
                target_village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 25,
                    speed: 1,
                },
            )
            .await
            .unwrap();
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
        assert_eq!(pending_returns, 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_bounty_respects_source_capacity() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), rally_point(1), barracks(1)],
            resources(799, 799, 799, 799),
        )
        .await;
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
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
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(24), 500)
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
                    attack_type: AttackType::Raid,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

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

        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Scout Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
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
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Scout Target",
            Position { x: 3, y: 3 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .research_academy(
                village_id,
                &ResearchAcademy {
                    player_id,
                    unit: UnitName::Scout,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 20)
            .await
            .unwrap();
        service
            .train_units(
                village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 3,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 20)
            .await
            .unwrap();

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

        let first_processed = service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(first_processed, 1);

        let second_processed = service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();
        assert_eq!(second_processed, 1);

        let after_return = service.get_village(village_id).await.unwrap();
        assert_eq!(army_units(&after_return, 3), 1);
        assert_eq!(troops_sum(&after_return.deployed_armies, 3), 0);
    })
    .await;
}
