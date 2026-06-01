use chrono::{Duration, Utc};
use parabellum_app::villages::repositories::HeroRepository;
use parabellum_app::villages::{
    AttackVillage, CreateHero, ReviveHero, SendReinforcement, TrainUnits,
    models::ScheduledActionType,
};
use parabellum_game::models::{buildings::Building, hero::Hero, village::VillageBuilding};
use parabellum_types::army::TroopSet;
use parabellum_types::{battle::AttackType, buildings::BuildingName, map::Position};
use uuid::Uuid;

use crate::es::{PostgresHeroRepository, VillageEsService};

use super::fixtures::{
    barracks, deployed_units, granary, main_building, rally_point, resources, setup_village,
    setup_village_for_player, stationed_units, warehouse, with_test_pool,
};

fn hero_mansion(level: u8) -> VillageBuilding {
    let building = Building::new(BuildingName::HeroMansion, 1)
        .at_level(level, 1)
        .expect("hero mansion building data should be available for fixture");
    VillageBuilding {
        slot_id: 25,
        building,
    }
}

#[tokio::test]
async fn village_es_service_create_hero_projects_rm_heroes() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 0, y: 0 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let hero = service.get_hero(hero_id).await.unwrap();
        assert_eq!(hero.id, hero_id);
        assert_eq!(hero.player_id, player_id);
        assert_eq!(hero.village_id, village_id);
        assert_eq!(hero.health, 100);

        assert!(service.player_has_alive_hero(player_id).await.unwrap());
    })
    .await;
}

#[tokio::test]
async fn village_es_service_revive_hero_schedules_and_completes_action() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 2, y: 2 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let mut dead_hero = service.get_hero(hero_id).await.unwrap();
        dead_hero.apply_battle_damage(1.0);
        assert_eq!(dead_hero.health, 0);
        let heroes = PostgresHeroRepository::new(pool.clone());
        heroes
            .upsert(&dead_hero, village_id, village_id, "home")
            .await
            .unwrap();

        let action_id = Uuid::new_v4();
        let revive_at = Utc::now() + Duration::minutes(1);
        service
            .revive_hero(
                village_id,
                &ReviveHero {
                    action_id,
                    player_id,
                    village_id,
                    hero: dead_hero,
                    reset: true,
                    speed: 1,
                    revive_at,
                },
            )
            .await
            .unwrap();

        let counts_before = service
            .get_village_scheduled_action_status_counts(
                village_id,
                ScheduledActionType::HeroRevival,
                None,
            )
            .await
            .unwrap();
        assert_eq!(counts_before.pending, 1);

        let processed = service
            .process_due_actions(Utc::now() + Duration::hours(2), 10)
            .await
            .unwrap();
        assert_eq!(processed, 1);

        let hero_after = service.get_hero(hero_id).await.unwrap();
        assert!(hero_after.health > 0);
        assert_eq!(hero_after.village_id, village_id);

        let counts = service
            .get_village_scheduled_action_status_counts(
                village_id,
                ScheduledActionType::HeroRevival,
                None,
            )
            .await
            .unwrap();
        assert_eq!(counts.completed, 1);
        assert_eq!(counts.pending, 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_revive_hero_rejects_without_hero_mansion() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 3, y: 3 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), rally_point(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let mut dead_hero = Hero::new(
            Some(Uuid::new_v4()),
            village_id,
            player_id,
            parabellum_types::tribe::Tribe::Roman,
            Some(5),
        );
        dead_hero.apply_battle_damage(1.0);
        PostgresHeroRepository::new(pool.clone())
            .upsert(&dead_hero, village_id, village_id, "home")
            .await
            .unwrap();

        let result = service
            .revive_hero(
                village_id,
                &ReviveHero {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    hero: dead_hero,
                    reset: false,
                    speed: 1,
                    revive_at: Utc::now() + Duration::minutes(1),
                },
            )
            .await;

        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn village_es_service_hero_alone_reinforcement_transfers_home_when_both_have_hero_mansion() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 4, y: 4 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Village B",
            Position { x: 5, y: 5 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::default(),
                    hero_id: Some(hero_id),
                    arrives_at: Utc::now() + Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(Utc::now() + Duration::minutes(10), 10)
            .await
            .unwrap();

        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        let target = service.get_village(target_village_id).await.unwrap();
        assert_eq!(
            target.army.as_ref().and_then(|a| a.hero()).map(|h| h.id),
            Some(hero_id)
        );

        let hero_after = service.get_hero(hero_id).await.unwrap();
        assert_eq!(hero_after.village_id, target_village_id);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_hero_with_troops_reinforcement_does_not_transfer_home() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 6, y: 6 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
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
            "Village B",
            Position { x: 7, y: 7 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(Utc::now() + Duration::hours(2), 10)
            .await
            .unwrap();

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: Some(hero_id),
                    arrives_at: Utc::now() + Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(Utc::now() + Duration::minutes(10), 10)
            .await
            .unwrap();

        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 1);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 1);
        let target = service.get_village(target_village_id).await.unwrap();
        assert_eq!(target.reinforcements[0].hero().map(|h| h.id), Some(hero_id));

        let hero_after = service.get_hero(hero_id).await.unwrap();
        assert_eq!(hero_after.village_id, source_village_id);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_hero_alone_to_village_without_hero_mansion_does_not_transfer_home() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 8, y: 8 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let target_village_id = setup_village_for_player(
            &service,
            player_id,
            "Village B",
            Position { x: 9, y: 9 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::default(),
                    hero_id: Some(hero_id),
                    arrives_at: Utc::now() + Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(Utc::now() + Duration::minutes(10), 10)
            .await
            .unwrap();

        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        let target = service.get_village(target_village_id).await.unwrap();
        assert_eq!(target.reinforcements[0].hero().map(|h| h.id), Some(hero_id));

        let hero_after = service.get_hero(hero_id).await.unwrap();
        assert_eq!(hero_after.village_id, source_village_id);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_create_hero_rejects_when_alive_hero_exists() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 30, y: 30 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let second = service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await;
        assert!(second.is_err());
    })
    .await;
}

#[tokio::test]
async fn village_es_service_revive_hero_rejects_when_alive_hero_exists() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 31, y: 31 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let hero = service.get_hero(hero_id).await.unwrap();
        let result = service
            .revive_hero(
                village_id,
                &ReviveHero {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    hero,
                    reset: false,
                    speed: 1,
                    revive_at: Utc::now() + Duration::minutes(1),
                },
            )
            .await;
        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn village_es_service_revive_hero_rejects_when_pending_revival_exists() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, village_id) = setup_village(
            &pool,
            &service,
            "Village A",
            Position { x: 32, y: 32 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                hero_mansion(1),
                warehouse(20),
                granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let mut dead_hero = service.get_hero(hero_id).await.unwrap();
        dead_hero.apply_battle_damage(1.0);
        let heroes = PostgresHeroRepository::new(pool.clone());
        heroes
            .upsert(&dead_hero, village_id, village_id, "home")
            .await
            .unwrap();

        let revive_at = Utc::now() + Duration::minutes(1);
        service
            .revive_hero(
                village_id,
                &ReviveHero {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    hero: dead_hero.clone(),
                    reset: false,
                    speed: 1,
                    revive_at,
                },
            )
            .await
            .unwrap();

        let second = service
            .revive_hero(
                village_id,
                &ReviveHero {
                    action_id: Uuid::new_v4(),
                    player_id,
                    village_id,
                    hero: dead_hero,
                    reset: false,
                    speed: 1,
                    revive_at: Utc::now() + Duration::minutes(2),
                },
            )
            .await;
        assert!(second.is_err());
    })
    .await;
}

#[tokio::test]
async fn village_es_service_attack_with_hero_returns_hero_home() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source",
            Position { x: 33, y: 33 },
            parabellum_types::tribe::Tribe::Roman,
            vec![
                main_building(1),
                rally_point(1),
                barracks(1),
                hero_mansion(1),
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
            Position { x: 34, y: 34 },
            parabellum_types::tribe::Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(Utc::now() + Duration::hours(2), 10)
            .await
            .unwrap();

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let now = Utc::now();
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: Some(hero_id),
                    attack_type: AttackType::Normal,
                    catapult_targets: [Some(BuildingName::MainBuilding), Some(BuildingName::Warehouse)],
                    arrives_at: now + Duration::seconds(2),
                    returns_at: now + Duration::seconds(4),
                },
            )
            .await
            .unwrap();

        service
            .process_due_actions(now + Duration::seconds(3), 10)
            .await
            .unwrap();
        service
            .process_due_actions(now + Duration::seconds(5), 10)
            .await
            .unwrap();

        let source_after = service.get_village(source_village_id).await.unwrap();
        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(
            source_after
                .army
                .as_ref()
                .and_then(|a| a.hero())
                .map(|h| h.id),
            Some(hero_id)
        );
    })
    .await;
}
