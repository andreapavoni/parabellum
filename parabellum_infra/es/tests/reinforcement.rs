use parabellum_app::villages::{
    CreateHero, RecallReinforcements, ReleaseReinforcements, SendReinforcement, TrainUnits,
};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::{army::TroopSet, buildings::BuildingName, map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::es::VillageEsService;

use super::fixtures::{
    barracks, deployed_armies, deployed_units, granary, home_units, main_building, rally_point,
    resources, setup_village, stationed_armies, stationed_units, warehouse, with_test_pool,
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
async fn village_es_service_persists_events_and_projects_reinforcement() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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

        let (_, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
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

        assert_eq!(home_units(&pool, source_village_id, 0).await, 1);
        assert_eq!(stationed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(home_units(&pool, target_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        assert_eq!(deployed_units(&pool, target_village_id, 0).await, 0);

        let movement_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id,
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();

        let source_movements = service
            .get_village_troop_movements(source_village_id)
            .await
            .unwrap();
        assert_eq!(source_movements.outgoing.len(), 1);
        assert_eq!(source_movements.incoming.len(), 0);
        assert_eq!(source_movements.outgoing[0].job_id, movement_id);

        let target_movements = service
            .get_village_troop_movements(target_village_id)
            .await
            .unwrap();
        assert_eq!(target_movements.incoming.len(), 1);
        assert_eq!(target_movements.outgoing.len(), 0);
        assert_eq!(target_movements.incoming[0].job_id, movement_id);

        let village = service.get_village(source_village_id).await.unwrap();
        assert_eq!(village.player_id, source_player_id);
        assert_eq!(village.village_name, "Source Village");
        assert_eq!(home_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(village.buildings.len(), 5);

        let processed = service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        assert_eq!(processed, 1);

        assert_eq!(home_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, source_village_id, 0).await, 0);
        let source_after_recall = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            deployed_armies(&pool, source_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(
            source_after_recall.production.upkeep, source_after_recall.population,
            "owner village should not pay crop for troops stationed elsewhere"
        );
        assert_eq!(home_units(&pool, target_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 1);
        assert_eq!(deployed_units(&pool, target_village_id, 0).await, 0);
        let target_after_arrival = service.get_village(target_village_id).await.unwrap();
        assert_eq!(
            target_after_arrival.production.upkeep,
            target_after_arrival.population + 1,
            "stationed village should pay crop for hosted reinforcement"
        );

        let source_movements_after = service
            .get_village_troop_movements(source_village_id)
            .await
            .unwrap();
        let target_movements_after = service
            .get_village_troop_movements(target_village_id)
            .await
            .unwrap();
        assert_eq!(source_movements_after.outgoing.len(), 0);
        assert_eq!(target_movements_after.incoming.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_merges_reinforcement_waves_from_same_source() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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

        let (_, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
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
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(2), 10)
            .await
            .unwrap();
        assert_eq!(home_units(&pool, source_village_id, 0).await, 2);

        for minutes in [5, 6] {
            service
                .send_reinforcement(
                    source_village_id,
                    &SendReinforcement {
                        movement_id: Uuid::new_v4(),
                        army_id: Uuid::new_v4(),
                        player_id: source_player_id,
                        target_village_id,
                        units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                        hero_id: None,
                        arrives_at: chrono::Utc::now() + chrono::Duration::minutes(minutes),
                    },
                )
                .await
                .unwrap();
        }

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();

        let stationed = stationed_armies(&pool, target_village_id).await;
        assert_eq!(stationed.len(), 1);
        assert_eq!(stationed[0].village_id, source_village_id);
        assert_eq!(stationed[0].units().get(0), 2);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 2);
        assert_eq!(
            deployed_armies(&pool, source_village_id).await.len(),
            1,
            "owner-side deployed read model should reference the merged stationed army"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_recall_reinforcements_supports_partial_split() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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
        let (_, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let movement_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id,
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        let deployed_army = deployed_armies(&pool, source_village_id)
            .await
            .first()
            .cloned()
            .unwrap();
        assert_eq!(deployed_army.id, army_id);

        let recall_action_id = Uuid::new_v4();
        let recall_movement_id = Uuid::new_v4();
        service
            .recall_reinforcements(
                source_village_id,
                &RecallReinforcements {
                    action_id: recall_action_id,
                    movement_id: recall_movement_id,
                    player_id: source_player_id,
                    home_village_id: source_village_id,
                    stationed_village_id: target_village_id,
                    reinforcement_army: deployed_army,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            deployed_armies(&pool, source_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(
            stationed_armies(&pool, target_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        let source_movements_after_recall = service
            .get_village_troop_movements(source_village_id)
            .await
            .unwrap();
        assert_eq!(
            source_movements_after_recall.incoming.len(),
            1,
            "partial recall should project one incoming return movement for home village"
        );

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(30), 10)
            .await
            .unwrap();

        assert_eq!(home_units(&pool, source_village_id, 0).await, 1);
        assert_eq!(
            deployed_armies(&pool, source_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(
            stationed_armies(&pool, target_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_release_reinforcements_supports_partial_split() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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
        let (_, target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let movement_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id,
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        let reinforcement_army = stationed_armies(&pool, target_village_id)
            .await
            .first()
            .cloned()
            .unwrap();

        service
            .release_reinforcements(
                target_village_id,
                &ReleaseReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: target_player_id,
                    stationed_village_id: target_village_id,
                    home_village_id: source_village_id,
                    reinforcement_army,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            deployed_armies(&pool, source_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(
            stationed_armies(&pool, target_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        let source_movements_after_release = service
            .get_village_troop_movements(source_village_id)
            .await
            .unwrap();
        assert_eq!(
            source_movements_after_release.incoming.len(),
            1,
            "partial release should project one incoming return movement for home village"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_recall_reinforcements_full_return_clears_stationed_entries() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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
        let (_, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let movement_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id,
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        let deployed_army = deployed_armies(&pool, source_village_id)
            .await
            .first()
            .cloned()
            .unwrap();

        service
            .recall_reinforcements(
                source_village_id,
                &RecallReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: source_player_id,
                    home_village_id: source_village_id,
                    stationed_village_id: target_village_id,
                    reinforcement_army: deployed_army,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        let source_returning = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            source_returning.production.upkeep,
            source_returning.population + 2,
            "owner village should pay crop while recalled reinforcements return"
        );
        let target_after_recall = service.get_village(target_village_id).await.unwrap();
        assert_eq!(
            target_after_recall.production.upkeep, target_after_recall.population,
            "stationed village should stop paying crop after recall"
        );
        let source_movements = service
            .get_village_troop_movements(source_village_id)
            .await
            .unwrap();
        assert_eq!(source_movements.incoming.len(), 1);

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(30), 10)
            .await
            .unwrap();

        assert_eq!(home_units(&pool, source_village_id, 0).await, 2);
        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        let source_home = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            source_home.production.upkeep,
            source_home.population + 2,
            "owner village should pay crop after recalled reinforcements are home"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_release_reinforcements_full_return_clears_stationed_entries() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
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
        let (_, target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 10, y: 10 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let movement_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id,
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();
        let reinforcement_army = stationed_armies(&pool, target_village_id)
            .await
            .first()
            .cloned()
            .unwrap();

        service
            .release_reinforcements(
                target_village_id,
                &ReleaseReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: target_player_id,
                    stationed_village_id: target_village_id,
                    home_village_id: source_village_id,
                    reinforcement_army,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        let source_returning = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            source_returning.production.upkeep,
            source_returning.population + 2,
            "owner village should pay crop while released reinforcements return"
        );
        let target_after_release = service.get_village(target_village_id).await.unwrap();
        assert_eq!(
            target_after_release.production.upkeep, target_after_release.population,
            "stationed village should stop paying crop after release"
        );

        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(30), 10)
            .await
            .unwrap();

        assert_eq!(home_units(&pool, source_village_id, 0).await, 2);
        assert_eq!(deployed_units(&pool, source_village_id, 0).await, 0);
        assert_eq!(stationed_units(&pool, target_village_id, 0).await, 0);
        let source_home = service.get_village(source_village_id).await.unwrap();
        assert_eq!(
            source_home.production.upkeep,
            source_home.population + 2,
            "owner village should pay crop after released reinforcements are home"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_recall_partial_split_with_hero_moves_hero_with_returning_army() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source Village",
            Position { x: 20, y: 20 },
            Tribe::Roman,
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
        let (_, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 21, y: 21 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id: source_player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: Some(hero_id),
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();

        let deployed = deployed_armies(&pool, source_village_id)
            .await
            .into_iter()
            .find(|a| a.id == army_id)
            .unwrap();
        assert_eq!(deployed.hero().map(|h| h.id), Some(hero_id));

        service
            .recall_reinforcements(
                source_village_id,
                &RecallReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: source_player_id,
                    home_village_id: source_village_id,
                    stationed_village_id: target_village_id,
                    reinforcement_army: deployed,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: Some(hero_id),
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        let target_after_recall = stationed_armies(&pool, target_village_id).await;
        assert_eq!(
            deployed_armies(&pool, source_village_id)
                .await
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(
            target_after_recall
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(target_after_recall[0].hero().map(|h| h.id), None);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_release_partial_split_without_hero_keeps_hero_stationed() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, source_player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Source Village",
            Position { x: 22, y: 22 },
            Tribe::Roman,
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
        let (_, target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Target Village",
            Position { x: 23, y: 23 },
            Tribe::Roman,
            vec![main_building(1), warehouse(20), granary(20)],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id: source_player_id,
                    unit_idx: 0,
                    building_name: BuildingName::Barracks,
                    quantity: 2,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::hours(4), 10)
            .await
            .unwrap();

        let hero_id = Uuid::new_v4();
        service
            .create_hero(
                source_village_id,
                &CreateHero {
                    hero_id,
                    player_id: source_player_id,
                    village_id: source_village_id,
                    has_existing_hero: false,
                },
            )
            .await
            .unwrap();

        let army_id = Uuid::new_v4();
        service
            .send_reinforcement(
                source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id,
                    player_id: source_player_id,
                    target_village_id,
                    units: TroopSet::new([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: Some(hero_id),
                    arrives_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::minutes(10), 10)
            .await
            .unwrap();

        let stationed = stationed_armies(&pool, target_village_id)
            .await
            .into_iter()
            .find(|a| a.id == army_id)
            .unwrap();
        assert_eq!(stationed.hero().map(|h| h.id), Some(hero_id));

        service
            .release_reinforcements(
                target_village_id,
                &ReleaseReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: target_player_id,
                    stationed_village_id: target_village_id,
                    home_village_id: source_village_id,
                    reinforcement_army: stationed,
                    units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    hero_id: None,
                    returns_at: chrono::Utc::now() + chrono::Duration::minutes(20),
                },
            )
            .await
            .unwrap();

        let target_after_release = stationed_armies(&pool, target_village_id).await;
        assert_eq!(
            target_after_release
                .iter()
                .map(|a| a.units().get(0))
                .sum::<u32>(),
            1
        );
        assert_eq!(target_after_release[0].hero().map(|h| h.id), Some(hero_id));
    })
    .await;
}
