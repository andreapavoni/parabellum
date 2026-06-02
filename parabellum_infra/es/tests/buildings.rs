use parabellum_app::villages::{AddBuilding, DowngradeBuilding, UpgradeBuilding};
use parabellum_types::{buildings::BuildingName, map::Position};

use crate::es::VillageEsService;

use super::fixtures::{
    granary, main_building, resources, setup_village, warehouse, with_test_pool,
};

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
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(10), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .add_building(
                village_id,
                &AddBuilding {
                    player_id,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_add = service.get_village(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_add, 22, BuildingName::Cranny),
            Some(1)
        );

        service
            .upgrade_building(
                village_id,
                &UpgradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_upgrade = service.get_village(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_upgrade, 22, BuildingName::Cranny),
            Some(2)
        );

        service
            .downgrade_building(
                village_id,
                &DowngradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_downgrade = service.get_village(village_id).await.unwrap();
        assert_eq!(
            building_level(&after_downgrade, 22, BuildingName::Cranny),
            Some(1)
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_recomputes_culture_points_production_after_building_changes() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village B",
            Position { x: 1, y: 1 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(10), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .add_building(
                village_id,
                &AddBuilding {
                    player_id,
                    slot_id: 22,
                    building_name: BuildingName::Cranny,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();
        service
            .upgrade_building(
                village_id,
                &UpgradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_first_upgrade = service.get_village(village_id).await.unwrap();
        let hydrated_after_first_upgrade =
            parabellum_game::models::village::Village::try_from(after_first_upgrade.clone())
                .unwrap();
        assert_eq!(
            after_first_upgrade.culture_points_production,
            hydrated_after_first_upgrade.culture_points_production
        );

        service
            .upgrade_building(
                village_id,
                &UpgradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_upgrade = service.get_village(village_id).await.unwrap();
        let hydrated_after_upgrade =
            parabellum_game::models::village::Village::try_from(after_upgrade.clone()).unwrap();
        assert_eq!(
            after_upgrade.culture_points_production,
            hydrated_after_upgrade.culture_points_production
        );

        service
            .downgrade_building(
                village_id,
                &DowngradeBuilding {
                    player_id,
                    slot_id: 22,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();

        let after_downgrade = service.get_village(village_id).await.unwrap();
        let hydrated_after_downgrade =
            parabellum_game::models::village::Village::try_from(after_downgrade.clone()).unwrap();
        assert_eq!(
            after_downgrade.culture_points_production,
            hydrated_after_downgrade.culture_points_production
        );
        assert_ne!(
            after_upgrade.culture_points_production,
            after_downgrade.culture_points_production
        );
    })
    .await;
}
