use parabellum_app::villages::AddBuilding;
use parabellum_types::{buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{found_village_cmd, seed_player_and_village, setup_pool};

#[tokio::test]
async fn village_es_service_scheduler_is_idempotent_and_lists_player_villages() {
    let Some(pool) = setup_pool().await else {
        return;
    };

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
        .add_building(
            100,
            &AddBuilding {
                player_id,
                slot_id: 22,
                building_name: BuildingName::Barracks,
                speed: 1,
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

    let barracks_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rm_village v, jsonb_array_elements(v.buildings) b \
         WHERE v.village_id = 100 AND b->'building'->>'name' = 'Barracks'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(barracks_count, 1);

    let models = service
        .list_village_models_by_player_id(player_id)
        .await
        .unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].village_id, 100);
    assert_eq!(models[1].village_id, 101);
}
