use parabellum_app::villages::{SendReinforcement, TrainUnits};
use parabellum_types::{army::TroopSet, buildings::BuildingName, map::Position, tribe::Tribe};
use sqlx::Row;
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    barracks, granary, main_building, rally_point, resources, setup_village, warehouse,
    with_test_pool,
};

#[tokio::test]
async fn village_es_service_persists_events_and_projects_reinforcement() {
    with_test_pool(|pool| async move {

    let source_player_id = Uuid::new_v4();
    let source_user_id = Uuid::new_v4();
    let target_player_id = Uuid::new_v4();
    let target_user_id = Uuid::new_v4();

    let service = VillageEsService::new(pool.clone());
    setup_village(
        &pool,
        &service,
        source_player_id,
        source_user_id,
        100,
        "Source Village",
        Position { x: 0, y: 0 },
        Tribe::Roman,
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
        target_player_id,
        target_user_id,
        200,
        "Target Village",
        Position { x: 10, y: 10 },
        Tribe::Roman,
        vec![main_building(1), warehouse(20), granary(20)],
        resources(80_000, 80_000, 80_000, 80_000),
    )
    .await;
    service
        .train_units(
            100,
            &TrainUnits {
                player_id: source_player_id,
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

    let source_before_arrival = service.get_village_model(100).await.unwrap();
    assert_eq!(source_before_arrival.army.get(0), 1);
    assert_eq!(source_before_arrival.reinforcements.get(0), 0);
    assert_eq!(source_before_arrival.deployed_armies.get(0), 0);
    let target_before_arrival = service.get_village_model(200).await.unwrap();
    assert_eq!(target_before_arrival.army.get(0), 0);
    assert_eq!(target_before_arrival.reinforcements.get(0), 0);
    assert_eq!(target_before_arrival.deployed_armies.get(0), 0);

    let movement_id = Uuid::new_v4();
    let army_id = Uuid::new_v4();
    service
        .send_reinforcement(
            100,
            &SendReinforcement {
                movement_id,
                army_id,
                player_id: source_player_id,
                target_village_id: 200,
                units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                hero_id: None,
                arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            },
        )
        .await
        .unwrap();

    let events_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM es_events WHERE aggregate_type = $1 AND aggregate_id = $2",
    )
    .bind("parabellum_app::villages::aggregate::VillageAggregate")
    .bind("100")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events_count_before, 6);

    let movement_rows = sqlx::query(
        "SELECT village_id, direction::text AS direction FROM rm_village_movements WHERE movement_id = $1 ORDER BY village_id ASC",
    )
    .bind(movement_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(movement_rows.len(), 2);
    assert_eq!(movement_rows[0].get::<i32, _>("village_id"), 100);
    assert_eq!(movement_rows[0].get::<String, _>("direction"), "Outgoing");
    assert_eq!(movement_rows[1].get::<i32, _>("village_id"), 200);
    assert_eq!(movement_rows[1].get::<String, _>("direction"), "Incoming");

    let movements_view = service.get_village_troop_movements(100).await.unwrap();
    assert_eq!(movements_view.outgoing.len(), 1);
    assert_eq!(movements_view.incoming.len(), 0);
    assert_eq!(movements_view.outgoing[0].movement_id, movement_id);

    let village = service.get_village_model(100).await.unwrap();
    assert_eq!(village.player_id, source_player_id);
    assert_eq!(village.village_name, "Source Village");
    assert_eq!(village.army.get(0), 0);
    assert_eq!(village.buildings.len(), 5);

    let action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rm_scheduled_actions WHERE action_type = 'ReinforcementArrival'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(action_count, 1);

    let processed = service
        .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
        .await
        .unwrap();
    assert_eq!(processed, 1);

    let source_after_arrival = service.get_village_model(100).await.unwrap();
    assert_eq!(source_after_arrival.army.get(0), 0);
    assert_eq!(source_after_arrival.reinforcements.get(0), 0);
    assert_eq!(source_after_arrival.deployed_armies.get(0), 1);
    let target_after_arrival = service.get_village_model(200).await.unwrap();
    assert_eq!(target_after_arrival.army.get(0), 0);
    assert_eq!(target_after_arrival.reinforcements.get(0), 1);
    assert_eq!(target_after_arrival.deployed_armies.get(0), 0);

    let events_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM es_events WHERE aggregate_type = $1 AND aggregate_id = $2",
    )
    .bind("parabellum_app::villages::aggregate::VillageAggregate")
    .bind("100")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events_count_after, 7);

    let movement_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rm_village_movements WHERE movement_id = $1")
            .bind(movement_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(movement_count_after, 0);

    let completed_actions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rm_scheduled_actions WHERE action_type = 'ReinforcementArrival' AND status = 'completed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(completed_actions, 1);
    })
    .await;
}
