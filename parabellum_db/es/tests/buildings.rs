use parabellum_app::villages::{CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding};
use parabellum_types::{buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{building_level, found_village_cmd, seed_player_and_village, setup_pool};

#[tokio::test]
async fn village_es_service_projects_building_lifecycle_on_rm_village() {
    let Some(pool) = setup_pool().await else {
        return;
    };

    let player_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_player_and_village(&pool, player_id, user_id, 100, "Village A", 0, 0).await;

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
                building_name: BuildingName::Barracks,
                level: 1,
                speed: 1,
            },
        )
        .await
        .unwrap();

    let model = service.get_village_model(100).await.unwrap();
    assert_eq!(
        building_level(&model.buildings, 22, BuildingName::Barracks),
        Some(1)
    );

    service
        .complete_upgrade_building(
            100,
            &CompleteUpgradeBuilding {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 100,
                slot_id: 22,
                building_name: BuildingName::Barracks,
                level: 2,
                speed: 1,
            },
        )
        .await
        .unwrap();

    let model = service.get_village_model(100).await.unwrap();
    assert_eq!(
        building_level(&model.buildings, 22, BuildingName::Barracks),
        Some(2)
    );

    service
        .complete_downgrade_building(
            100,
            &CompleteDowngradeBuilding {
                action_id: Uuid::new_v4(),
                player_id,
                village_id: 100,
                slot_id: 22,
                building_name: BuildingName::Barracks,
                level: 1,
                speed: 1,
            },
        )
        .await
        .unwrap();

    let model = service.get_village_model(100).await.unwrap();
    assert_eq!(
        building_level(&model.buildings, 22, BuildingName::Barracks),
        Some(1)
    );
}
