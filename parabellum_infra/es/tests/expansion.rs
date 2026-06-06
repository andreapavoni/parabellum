use std::sync::Arc;

use chrono::{Duration, Utc};
use parabellum_app::ports::identity::PlayerRepository;
use parabellum_app::{config::Config, ports::queries::VillageQueryPort};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::buildings::BuildingName;
use parabellum_types::{common::Speed, map::Position, tribe::Tribe};

use crate::identity::repositories::PostgresPlayerRepository;
use crate::{adapters::VillageEsAdapter, es::VillageEsService};

use super::fixtures::{
    main_building, rally_point, resources, seed_user_and_player, setup_village,
    setup_village_for_player, with_test_pool,
};

#[tokio::test]
async fn village_query_port_returns_expansion_culture_info() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Capital",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1), rally_point(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let village = service.get_village(village_id).await.unwrap();
        let player_repo = PostgresPlayerRepository::new(pool.clone());
        let player = player_repo.get_by_id(player_id).await.unwrap();
        let player_cp_production = player_repo
            .get_total_culture_points_production(player_id)
            .await
            .unwrap();

        let adapter = VillageEsAdapter::new(
            service,
            Arc::new(Config {
                world_size: 100,
                speed: 1,
                access_token_ttl_secs: 900,
                refresh_token_ttl_secs: 2_592_000,
                token_signing_key: "test-key".to_string(),
            }),
        );

        let info = adapter
            .get_expansion_culture_info(player_id, village_id, 1)
            .await
            .unwrap();

        assert_eq!(info.player_culture_points, player.culture_points as u32);
        assert_eq!(info.player_culture_points_production, player_cp_production);
        assert_eq!(
            info.village_culture_points_production,
            village.culture_points_production
        );
        assert_eq!(
            info.next_cp_required,
            parabellum_game::models::culture_points::required_cp(Speed::X1, 2)
        );
    })
    .await;
}

#[tokio::test]
async fn expansion_culture_info_ticks_player_cp_from_elapsed_time_single_village() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Capital",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                VillageBuilding {
                    slot_id: 26,
                    building: Building::new(BuildingName::Residence, 1)
                        .at_level(10, 1)
                        .unwrap(),
                },
            ],
            resources(800, 800, 800, 800),
        )
        .await;
        let expected_cpp = service
            .get_village(village_id)
            .await
            .unwrap()
            .culture_points_production;

        let cp_anchor = Utc::now() - Duration::days(2);
        sqlx::query(
            r#"
            UPDATE players
            SET culture_points = 0,
                culture_points_updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(player_id)
        .bind(cp_anchor)
        .execute(&pool)
        .await
        .unwrap();

        let adapter = VillageEsAdapter::new(
            service,
            Arc::new(Config {
                world_size: 100,
                speed: 1,
                access_token_ttl_secs: 900,
                refresh_token_ttl_secs: 2_592_000,
                token_signing_key: "test-key".to_string(),
            }),
        );

        let info = adapter
            .get_expansion_culture_info(player_id, village_id, 1)
            .await
            .unwrap();

        assert!(expected_cpp > 0);
        assert!(info.player_culture_points > 0);
        assert_eq!(info.player_culture_points_production, expected_cpp);
    })
    .await;
}

#[tokio::test]
async fn expansion_culture_info_ticks_player_cp_from_elapsed_time_multi_village_sum() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id) = seed_user_and_player(&pool).await;
        let village_a_id = setup_village_for_player(
            &service,
            player_id,
            "Village A",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                VillageBuilding {
                    slot_id: 26,
                    building: Building::new(BuildingName::Residence, 1)
                        .at_level(10, 1)
                        .unwrap(),
                },
            ],
            resources(800, 800, 800, 800),
        )
        .await;
        let village_b_id = setup_village_for_player(
            &service,
            player_id,
            "Village B",
            Position { x: 1, y: 0 },
            Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                VillageBuilding {
                    slot_id: 26,
                    building: Building::new(BuildingName::Residence, 1)
                        .at_level(10, 1)
                        .unwrap(),
                },
            ],
            resources(800, 800, 800, 800),
        )
        .await;
        let expected_cpp = service
            .get_village(village_a_id)
            .await
            .unwrap()
            .culture_points_production
            + service
                .get_village(village_b_id)
                .await
                .unwrap()
                .culture_points_production;

        let cp_anchor = Utc::now() - Duration::days(2);
        sqlx::query(
            r#"
            UPDATE players
            SET culture_points = 0,
                culture_points_updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(player_id)
        .bind(cp_anchor)
        .execute(&pool)
        .await
        .unwrap();

        let adapter = VillageEsAdapter::new(
            service,
            Arc::new(Config {
                world_size: 100,
                speed: 1,
                access_token_ttl_secs: 900,
                refresh_token_ttl_secs: 2_592_000,
                token_signing_key: "test-key".to_string(),
            }),
        );

        let info = adapter
            .get_expansion_culture_info(player_id, village_a_id, 1)
            .await
            .unwrap();

        assert!(expected_cpp > 0);
        assert!(info.player_culture_points > 0);
        assert_eq!(info.player_culture_points_production, expected_cpp);
    })
    .await;
}
