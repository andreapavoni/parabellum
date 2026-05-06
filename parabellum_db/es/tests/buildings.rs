use parabellum_app::villages::{
    CompleteAddBuilding, CompleteDowngradeBuilding, CompleteUpgradeBuilding,
};
use parabellum_types::{buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{found_village_cmd, seed_user_and_player, with_test_pool};

fn building_level(
    village: &parabellum_app::villages::models::VillageModel,
    slot_id: u8,
    building_name: BuildingName,
) -> Option<u8> {
    village
        .buildings
        .iter()
        .find(|building| building.slot_id == slot_id && building.building.name == building_name)
        .map(|building| building.building.level)
}

#[tokio::test]
async fn village_es_service_projects_building_lifecycle_on_rm_village() {
    with_test_pool(|pool| async move {
        let (_user_id, player_id) = seed_user_and_player(&pool).await;
        let position = Position { x: 0, y: 0 };
        let village_id = position.to_id(100);

        let service = VillageEsService::new(pool.clone());
        service
            .found_village(
                village_id,
                &found_village_cmd(player_id, "Village A", Position { x: 0, y: 0 }),
            )
            .await
            .unwrap();

        service
            .complete_add_building(
                village_id,
                &CompleteAddBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after_add = service.get_village_model(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_add, 22, BuildingName::Cranny),
            Some(1)
        );

        service
            .complete_upgrade_building(
                village_id,
                &CompleteUpgradeBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after_upgrade = service.get_village_model(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_upgrade, 22, BuildingName::Cranny),
            Some(2)
        );

        service
            .complete_downgrade_building(
                village_id,
                &CompleteDowngradeBuilding {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    level: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();

        let after_downgrade = service.get_village_model(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_downgrade, 22, BuildingName::Cranny),
            Some(1)
        );
    })
    .await;
}
