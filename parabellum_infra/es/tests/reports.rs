use parabellum_app::villages::{
    AttackVillage, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
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

use crate::es::VillageEsService;

use super::fixtures::{
    EsScenario, academy, barracks, deployed_units, granary, main_building, process_due_until,
    rally_point, research_and_complete, resources, setup_village, setup_village_for_player,
    train_and_complete, warehouse, with_test_pool,
};

#[tokio::test]
async fn village_es_service_attack_projects_single_audience_report_for_same_player() {
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
                    warehouse(20),
                    granary(20),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;
        let second_village_id = scenario
            .village_for_player(
                player_id,
                "Target",
                Position { x: 2, y: 2 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        scenario
            .train_and_complete(
                village_id,
                player_id,
                0,
                BuildingName::Barracks,
                1,
                1,
                chrono::Utc::now() + chrono::Duration::hours(3),
                50,
            )
            .await;

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

        scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;

        let reports = service
            .list_reports_for_player(player_id, 10)
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].report_type, "battle");
        assert_eq!(reports[0].actor_village_id, Some(village_id));
        assert_eq!(reports[0].target_village_id, Some(second_village_id));

        scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;

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
        let scenario = EsScenario::new(&pool, &service);

        let (_attacker_user_id, attacker_player_id, source_village_id) = scenario
            .village(
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

        let (_defender_user_id, defender_player_id, target_village_id) = scenario
            .village(
                "Target",
                Position { x: 2, y: 2 },
                parabellum_types::tribe::Tribe::Teuton,
                vec![main_building(1), warehouse(20), granary(20)],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        scenario
            .train_and_complete(
                source_village_id,
                attacker_player_id,
                0,
                BuildingName::Barracks,
                1,
                1,
                chrono::Utc::now() + chrono::Duration::hours(3),
                50,
            )
            .await;

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

        scenario
            .process_until(arrives_at + chrono::Duration::seconds(1), 10)
            .await;

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

        scenario
            .process_until(returns_at + chrono::Duration::seconds(1), 10)
            .await;

        let attacker_reports_after_return = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        let defender_reports_after_return = service
            .list_reports_for_player(defender_player_id, 10)
            .await
            .unwrap();
        assert_eq!(attacker_reports_after_return.len(), 1);
        assert_eq!(defender_reports_after_return.len(), 1);

        assert_eq!(
            deployed_units(&pool, source_village_id, 0).await,
            0,
            "attack return must not remain projected as deployed reinforcement",
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_projects_reinforcement_owner_audience() {
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

        train_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            200,
        )
        .await;
        train_and_complete(
            &service,
            reinforcer_village_id,
            reinforcer_player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            200,
        )
        .await;

        let reinforcement_arrival = chrono::Utc::now() + chrono::Duration::seconds(2);
        service
            .send_reinforcement(
                reinforcer_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: reinforcer_player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: reinforcement_arrival,
                },
            )
            .await
            .unwrap();
        process_due_until(
            &service,
            reinforcement_arrival + chrono::Duration::seconds(1),
            50,
        )
        .await;

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
        process_due_until(&service, arrives_at + chrono::Duration::seconds(1), 50).await;

        let attacker_reports = service
            .list_reports_for_player(attacker_player_id, 10)
            .await
            .unwrap();
        let defender_reports = service
            .list_reports_for_player(defender_player_id, 10)
            .await
            .unwrap();
        let reinforcer_reports = service
            .list_reports_for_player(reinforcer_player_id, 10)
            .await
            .unwrap();

        let attacker_battle = attacker_reports
            .iter()
            .find(|report| report.report_type == "battle")
            .expect("attacker must receive battle report");
        let defender_battle = defender_reports
            .iter()
            .find(|report| report.report_type == "battle")
            .expect("defender must receive battle report");
        let reinforcer_battle = reinforcer_reports
            .iter()
            .find(|report| report.report_type == "battle")
            .expect("reinforcement owner must receive battle report");
        assert_eq!(attacker_battle.id, defender_battle.id);
        assert_eq!(attacker_battle.id, reinforcer_battle.id);
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

        train_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            50,
        )
        .await;

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

        process_due_until(&service, arrives_at + chrono::Duration::seconds(1), 10).await;

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

        train_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            50,
        )
        .await;

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
        process_due_until(
            &service,
            reinforcement_arrives_at + chrono::Duration::seconds(1),
            10,
        )
        .await;

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
        process_due_until(
            &service,
            merchants_arrive_at + chrono::Duration::seconds(1),
            10,
        )
        .await;

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

        train_and_complete(
            &service,
            source_village_id,
            player_id,
            0,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            50,
        )
        .await;

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
        process_due_until(&service, arrives_at + chrono::Duration::seconds(1), 10).await;

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

        research_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            UnitName::Scout,
            1,
            chrono::Utc::now() + chrono::Duration::hours(2),
            200,
        )
        .await;
        train_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            3,
            BuildingName::Barracks,
            1,
            1,
            chrono::Utc::now() + chrono::Duration::hours(3),
            50,
        )
        .await;

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

        process_due_until(&service, arrives_at + chrono::Duration::seconds(1), 10).await;

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
        let defender_reports = service
            .list_reports_for_player(defender_player_id, 10)
            .await
            .unwrap();
        assert_eq!(defender_reports.len(), 0);

        process_due_until(&service, returns_at + chrono::Duration::seconds(1), 10).await;

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

        let research_due_by = chrono::Utc::now() + chrono::Duration::hours(2);
        research_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            UnitName::Scout,
            1,
            research_due_by,
            200,
        )
        .await;
        research_and_complete(
            &service,
            target_village_id,
            defender_player_id,
            UnitName::Scout,
            1,
            research_due_by,
            200,
        )
        .await;

        let train_due_by = chrono::Utc::now() + chrono::Duration::hours(3);
        train_and_complete(
            &service,
            source_village_id,
            attacker_player_id,
            3,
            BuildingName::Barracks,
            1,
            1,
            train_due_by,
            200,
        )
        .await;
        train_and_complete(
            &service,
            target_village_id,
            defender_player_id,
            3,
            BuildingName::Barracks,
            1,
            1,
            train_due_by,
            200,
        )
        .await;

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
        process_due_until(&service, arrives_at + chrono::Duration::seconds(1), 20).await;

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
