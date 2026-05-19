use parabellum_app::villages::{SendSettlers, TrainUnits};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::buildings::BuildingName;
use parabellum_types::{map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    granary, main_building, rally_point, resources, setup_village, warehouse, with_test_pool,
};

fn residence(level: u8) -> VillageBuilding {
    VillageBuilding {
        slot_id: 25,
        building: Building::new(BuildingName::Residence, 1)
            .at_level(level, 1)
            .expect("residence should be buildable in tests"),
    }
}

async fn train_settlers(
    service: &VillageEsService,
    village_id: u32,
    player_id: Uuid,
    quantity: u8,
) {
    for _ in 0..quantity {
        service
            .train_units(
                village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 9,
                    building_name: parabellum_types::buildings::BuildingName::Residence,
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
}

#[tokio::test]
async fn village_es_service_send_settlers_schedules_arrival_and_withdraws_resources() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                residence(10),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        train_settlers(&service, source_village_id, player_id, 3).await;

        let source_before = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            source_before.army.as_ref().map(|a| a.units().get(9)),
            Some(3)
        );
        let before = source_before.stocks.clone();

        let target_position = Position { x: 20, y: 20 };
        let target_field_id = target_position.to_id(100);
        service
            .send_settlers(
                source_village_id,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: target_field_id,
                    target_position: target_position.clone(),
                    village_name: "New Village".to_string(),
                    tribe: Tribe::Roman,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(10),
                },
            )
            .await
            .unwrap();

        let source_after = service.get_village(source_village_id).await.unwrap();
        let after = source_after.stocks.clone();
        assert_eq!(
            source_after
                .army
                .as_ref()
                .map(|a| a.units().get(9))
                .unwrap_or(0),
            0
        );
        assert_eq!(before.lumber - after.lumber, 800);
        assert_eq!(before.clay - after.clay, 800);
        assert_eq!(before.iron - after.iron, 800);
        assert!(before.crop - after.crop >= 800);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_settlers_arrival_founds_new_village_with_default_stocks() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                residence(10),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        train_settlers(&service, source_village_id, player_id, 3).await;

        let target_position = Position { x: 30, y: 30 };
        let target_field_id = target_position.to_id(100);
        service
            .send_settlers(
                source_village_id,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: target_field_id,
                    target_position: target_position.clone(),
                    village_name: "Colony".to_string(),
                    tribe: Tribe::Roman,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();

        let founded = service.get_village(target_field_id).await.unwrap();
        assert_eq!(founded.player_id, player_id);
        assert_eq!(founded.village_name, "Colony");
        assert_eq!(founded.position, target_position);
        assert_eq!(founded.buildings.len(), 0);
        assert_eq!(founded.stocks.lumber, 800);
        assert_eq!(founded.stocks.clay, 800);
        assert_eq!(founded.stocks.iron, 800);
        assert_eq!(founded.stocks.crop, 800);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_settlers_arrival_on_occupied_target_is_cancelled() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                residence(10),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        train_settlers(&service, source_village_id, player_id, 3).await;

        let occupied_position = Position { x: 40, y: 40 };
        let occupied_village_id = setup_village(
            &pool,
            &service,
            "Occupied Village",
            occupied_position.clone(),
            Tribe::Roman,
            vec![main_building(1), rally_point(1)],
            resources(800, 800, 800, 800),
        )
        .await
        .2;

        let occupied_before = service.get_village(occupied_village_id).await.unwrap();

        service
            .send_settlers(
                source_village_id,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id: occupied_village_id,
                    target_position: occupied_position,
                    village_name: "Should Fail".to_string(),
                    tribe: Tribe::Roman,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        let processed = service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        assert_eq!(processed, 1);

        let occupied_after = service.get_village(occupied_village_id).await.unwrap();
        assert_eq!(occupied_after.player_id, occupied_before.player_id);
        assert_eq!(occupied_after.village_name, occupied_before.village_name);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_first_settlers_arrival_wins_when_two_players_target_same_valley() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, player_a, source_a) = setup_village(
            &pool,
            &service,
            "Source A",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                residence(10),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_, player_b, source_b) = setup_village(
            &pool,
            &service,
            "Source B",
            Position { x: 10, y: 10 },
            Tribe::Gaul,
            vec![
                main_building(1),
                rally_point(1),
                residence(10),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        for (player_id, source_village_id) in [(player_a, source_a), (player_b, source_b)] {
            train_settlers(&service, source_village_id, player_id, 3).await;
        }

        let target_position = Position { x: 45, y: -45 };
        let target_field_id = target_position.to_id(100);
        let now = chrono::Utc::now();
        service
            .send_settlers(
                source_a,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: player_a,
                    target_village_id: target_field_id,
                    target_position: target_position.clone(),
                    village_name: "First Colony".to_string(),
                    tribe: Tribe::Roman,
                    arrives_at: now + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .send_settlers(
                source_b,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: player_b,
                    target_village_id: target_field_id,
                    target_position: target_position.clone(),
                    village_name: "Second Colony".to_string(),
                    tribe: Tribe::Gaul,
                    arrives_at: now + chrono::Duration::minutes(8),
                },
            )
            .await
            .unwrap();

        let processed_first = service
            .process_due_actions(now + chrono::Duration::minutes(6), 10)
            .await
            .unwrap();
        assert_eq!(processed_first, 1);

        let founded = service.get_village(target_field_id).await.unwrap();
        assert_eq!(founded.player_id, player_a);
        assert_eq!(founded.village_name, "First Colony");
        assert_eq!(founded.tribe, Tribe::Roman);

        let processed_second = service
            .process_due_actions(now + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        assert_eq!(processed_second, 1);

        let founded_after_second = service.get_village(target_field_id).await.unwrap();
        assert_eq!(founded_after_second.player_id, player_a);
        assert_eq!(founded_after_second.village_name, "First Colony");
        assert_eq!(founded_after_second.tribe, Tribe::Roman);

        let map_field = service.get_map_field(target_field_id).await.unwrap();
        assert_eq!(map_field.id, target_field_id);
        assert_eq!(map_field.village_id, Some(target_field_id));
        assert_eq!(map_field.player_id, Some(player_a));

        let map_tile = service
            .get_map_region_tile_by_field_id(target_field_id)
            .await
            .unwrap()
            .expect("founded village map tile should exist");
        assert_eq!(map_tile.field.id, target_field_id);
        assert_eq!(map_tile.field.village_id, Some(target_field_id));
        assert_eq!(map_tile.field.player_id, Some(player_a));
        assert_eq!(map_tile.village_name.as_deref(), Some("First Colony"));
        assert_eq!(map_tile.tribe, Some(Tribe::Roman));
    })
    .await;
}
