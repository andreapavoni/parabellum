use parabellum_app::villages::{
    CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding,
};
use parabellum_types::{buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{found_village_cmd, seed_user_and_player, with_test_pool};

#[tokio::test]
async fn village_es_service_projects_building_lifecycle_on_rm_village() {
    with_test_pool(|pool| async move {
        let player_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        seed_user_and_player(&pool, player_id, user_id).await;

        let service = VillageEsService::new(pool.clone());
        service
            .found_village(
                100,
                &found_village_cmd(player_id, "Village A", Position { x: 0, y: 0 }),
            )
            .await
            .unwrap();

        service
            .complete_add_building(
                100,
                &CompleteAddBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id: 100,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let events_after_add: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM es_events WHERE aggregate_type = $1 AND aggregate_id = $2",
        )
        .bind("parabellum_app::villages::aggregate::VillageAggregate")
        .bind("100")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(events_after_add, 2);

        service
            .complete_upgrade_building(
                100,
                &CompleteUpgradeBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id: 100,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let events_after_upgrade: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM es_events WHERE aggregate_type = $1 AND aggregate_id = $2",
        )
        .bind("parabellum_app::villages::aggregate::VillageAggregate")
        .bind("100")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(events_after_upgrade, 3);

        service
            .complete_downgrade_building(
                100,
                &CompleteDowngradeBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id: 100,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let events_after_downgrade: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM es_events WHERE aggregate_type = $1 AND aggregate_id = $2",
        )
        .bind("parabellum_app::villages::aggregate::VillageAggregate")
        .bind("100")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(events_after_downgrade, 4);
    })
    .await;
}
