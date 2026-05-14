use std::sync::Arc;

use parabellum_app::{config::Config, ports::queries::VillageQueryPort};
use parabellum_types::{common::Speed, map::Position, tribe::Tribe};

use crate::identity::repositories::PostgresPlayerRepository;
use crate::{adapters::VillageEsAdapter, es::VillageEsService};
use parabellum_app::repository::PlayerRepository;

use super::fixtures::{main_building, rally_point, resources, setup_village, with_test_pool};

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
        assert_eq!(info.village_culture_points, village.culture_points);
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
