use parabellum_app::villages::{
    AttackVillage, ResearchAcademy, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
    TrainUnits,
};
use parabellum_types::{
    army::{TroopSet, UnitName},
    battle::{AttackType, ScoutingTarget},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
    reports::ReportPayload,
};
use uuid::Uuid;

use crate::es::{VillageEsService, tests::fixtures::setup_village_for_player};

use super::fixtures::{
    academy, barracks, granary, main_building, rally_point, resources, setup_village, warehouse,
    with_test_pool,
};

#[tokio::test]
async fn village_es_service_attack_projects_single_audience_report_for_same_player() {
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
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let second_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 2, y: 2 },
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_attack(
                village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: second_village_id,
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
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let reports = service
            .list_reports_for_player(player_id, 10)
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].report_type, "battle");
        assert_eq!(reports[0].actor_village_id, Some(village_id));
        assert_eq!(reports[0].target_village_id, Some(second_village_id));

        service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let reports_after_return = service
            .list_reports_for_player(player_id, 10)
            .await
            .unwrap();
        assert_eq!(reports_after_return.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_projects_two_audiences_for_cross_player() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
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

        let (_defender_user_id, defender_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
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

        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let attacker_reports = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports.len(), 1);
        assert_eq!(attacker_reports[0].report_type, "battle");
        assert_eq!(
            attacker_reports[0].actor_village_id,
            Some(source_village_id)
        );
        assert_eq!(
            attacker_reports[0].target_village_id,
            Some(target_village_id)
        );

        let defender_reports = service
            .list_reports_for_player(defender_player_id, 10)
            .await
            .unwrap();
        assert_eq!(defender_reports.len(), 1);
        assert_eq!(defender_reports[0].id, attacker_reports[0].id);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_reports_query_and_mark_read_use_rm_tables() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
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
        let (_defender_user_id, _defender_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
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

        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let reports = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].read_at.is_none());

        let report_id = reports[0].id;
        let report = service
            .get_report_for_player(report_id, attacker_player_id)
            .await
            .unwrap();
        assert!(report.is_some());
        assert_eq!(report.unwrap().report_type, "battle");

        service
            .mark_report_as_read(report_id, attacker_player_id)
            .await
            .unwrap();

        let reread = service
            .get_report_for_player(report_id, attacker_player_id)
            .await
            .unwrap()
            .unwrap();
        assert!(reread.read_at.is_some());

        assert_eq!(reports[0].report_type, "battle");
    })
    .await;
}

#[tokio::test]
async fn village_es_service_reinforcement_and_merchant_reports_are_projected() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(1),
                rally_point(1),
                barracks(1),
                warehouse(20),
                granary(20),
                super::fixtures::marketplace(1),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_defender_user_id, _defender_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let reinforcement_arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: reinforcement_arrives_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(reinforcement_arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let merchants_arrive_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_resources(
                source_village_id,
                &SendMerchantsTransfer {
                    player_id: attacker_player_id,
                    target_village_id,
                    resources: ResourceGroup::new(200, 50, 25, 10),
                    arrives_at: merchants_arrive_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(merchants_arrive_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let player_reports = service
            .list_reports_for_player(attacker_player_id, 20)
            .await
            .unwrap();
        let reinforcement_reports = player_reports
            .iter()
            .filter(|r| r.report_type == "reinforcement")
            .count();
        let merchant_reports = player_reports
            .iter()
            .filter(|r| r.report_type == "marketplace_delivery")
            .count();
        assert_eq!(reinforcement_reports, 1);
        assert_eq!(merchant_reports, 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_reinforcement_reports_one_audience_for_same_player() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
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
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let reports = service
            .list_reports_for_player(player_id, 10)
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].report_type, "reinforcement");
        assert_eq!(reports[0].actor_village_id, Some(source_village_id));
        assert_eq!(reports[0].target_village_id, Some(target_village_id));
    })
    .await;
}

#[tokio::test]
async fn village_es_service_scout_projects_battle_report() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
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
        let (_defender_user_id, _defender_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .research_academy(
                source_village_id,
                &ResearchAcademy {
                    player_id: attacker_player_id,
                    unit: UnitName::Scout,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 200)
            .await
            .unwrap();
        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: attacker_player_id,
                    unit_idx: 3,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(3), 50)
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_scout(
                source_village_id,
                &ScoutVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 1, 0, 0, 0, 0, 0, 0]),
                    target: ScoutingTarget::Resources,
                    attack_type: AttackType::Raid,
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

        let attacker_reports = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports.len(), 1);
        assert_eq!(attacker_reports[0].report_type, "battle");
        let ReportPayload::Battle(payload) = &attacker_reports[0].payload else {
            panic!("expected scouting battle report");
        };
        assert!(payload.scouting.is_some());
        assert_eq!(
            payload.scouting.as_ref().unwrap().target,
            ScoutingTarget::Resources
        );

        service
            .process_due_actions(returns_at + chrono::Duration::seconds(1), 10)
            .await
            .unwrap();

        let attacker_reports_after_return = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports_after_return.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_detected_scout_reports_to_defender_player() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_attacker_user_id, attacker_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
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
        let (_defender_user_id, defender_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Teuton,
            vec![
                main_building(20),
                barracks(1),
                academy(20),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .research_academy(
                source_village_id,
                &ResearchAcademy {
                    player_id: attacker_player_id,
                    unit: UnitName::Scout,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .research_academy(
                target_village_id,
                &ResearchAcademy {
                    player_id: defender_player_id,
                    unit: UnitName::Scout,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        let source_academy_due_at = service
            .get_village_academy_queue(source_village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("source academy research should be queued");
        let target_academy_due_at = service
            .get_village_academy_queue(target_village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("target academy research should be queued");
        service
            .process_due_actions(
                source_academy_due_at.max(target_academy_due_at) + chrono::Duration::seconds(1),
                200,
            )
            .await
            .unwrap();

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: attacker_player_id,
                    unit_idx: 3,
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
                    player_id: defender_player_id,
                    unit_idx: 3,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        let source_training_due_at = service
            .get_village_training_queue(source_village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("source scout training should be queued");
        let target_training_due_at = service
            .get_village_training_queue(target_village_id)
            .await
            .unwrap()
            .iter()
            .map(|a| a.execute_at)
            .max()
            .expect("target scout training should be queued");
        service
            .process_due_actions(
                source_training_due_at.max(target_training_due_at) + chrono::Duration::seconds(1),
                200,
            )
            .await
            .unwrap();

        let arrives_at = chrono::Utc::now() + chrono::Duration::seconds(2);
        let returns_at = chrono::Utc::now() + chrono::Duration::seconds(4);
        service
            .send_scout(
                source_village_id,
                &ScoutVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: attacker_player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 1, 0, 0, 0, 0, 0, 0]),
                    target: ScoutingTarget::Defenses,
                    attack_type: AttackType::Raid,
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(arrives_at + chrono::Duration::seconds(1), 20)
            .await
            .unwrap();

        let attacker_reports = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports.len(), 1);
        let ReportPayload::Battle(payload) = &attacker_reports[0].payload else {
            panic!("expected scouting battle report");
        };
        assert!(payload.success);
        assert!(payload.scouting.as_ref().is_some_and(|s| s.was_detected));
        assert!(payload.defender.is_some());

        let defender_reports = service
            .list_reports_for_player(defender_player_id, 10)
            .await
            .unwrap();
        assert_eq!(defender_reports.len(), 1);
        assert_eq!(defender_reports[0].id, attacker_reports[0].id);
    })
    .await;
}
