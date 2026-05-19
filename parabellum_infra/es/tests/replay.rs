use chrono::{Duration, Utc};
use parabellum_app::villages::CreateMarketplaceOffer;
use parabellum_types::{map::Position, tribe::Tribe};
use parabellum_app::villages::{AttackVillage, ResearchAcademy, TrainUnits};
use parabellum_game::models::{buildings::Building, village::VillageBuilding};
use parabellum_types::army::{TroopSet, UnitName};
use parabellum_types::battle::AttackType;
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::{ResourceKind, ResourceQuantity};
use uuid::Uuid;

use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::{ReplayMode, ReplayRequest, ReplayService, ReplayTarget, VillageEsService};

use super::fixtures::{
    academy, granary, main_building, marketplace, resources, setup_village, warehouse,
    with_test_pool,
};

#[tokio::test]
async fn replay_dry_run_applies_village_events_without_writes() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            "Replay Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let replay = ReplayService::new(pool);
        let summary = replay
            .replay(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::DryRun,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        assert_eq!(summary.scanned, 2);
        assert_eq!(summary.applied, 2);
        assert_eq!(summary.skipped, 0);
        let first_global_seq = summary.first_global_seq.unwrap();
        let last_global_seq = summary.last_global_seq.unwrap();
        assert_eq!(last_global_seq, first_global_seq + 1);
    })
    .await;
}

#[tokio::test]
async fn replay_dry_run_reports_skip_non_report_events() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            "Replay Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let replay = ReplayService::new(pool);
        let summary = replay
            .dry_run(ReplayRequest {
                target: ReplayTarget::Reports,
                mode: ReplayMode::DryRun,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        assert_eq!(summary.scanned, 2);
        assert_eq!(summary.applied, 0);
        assert_eq!(summary.skipped, 2);
    })
    .await;
}

#[tokio::test]
async fn replay_full_mode_rebuilds_village_projection() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, _, village_id) = setup_village(
            &pool,
            &service,
            "Replay Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        sqlx::query("DELETE FROM rm_village WHERE village_id = $1")
            .bind(village_id as i32)
            .execute(&pool)
            .await
            .unwrap();

        let replay = ReplayService::new(pool);
        let summary = replay
            .replay(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::Full,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        assert_eq!(summary.scanned, 2);
        assert_eq!(summary.applied, 2);
        assert_eq!(summary.skipped, 0);

        let rebuilt = service.get_village(village_id).await.unwrap();
        assert_eq!(rebuilt.village_id, village_id);
    })
    .await;
}

#[tokio::test]
async fn replay_dry_run_filters_by_aggregate_id() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_, _, village_id_a) = setup_village(
            &pool,
            &service,
            "Replay Village A",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;
        setup_village(
            &pool,
            &service,
            "Replay Village B",
            Position { x: 1, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let replay = ReplayService::new(pool);
        let summary = replay
            .dry_run(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::DryRun,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: Some(village_id_a.to_string()),
            })
            .await
            .unwrap();

        assert_eq!(summary.scanned, 2);
        assert_eq!(summary.applied, 2);
        assert_eq!(summary.skipped, 0);
    })
    .await;
}

#[tokio::test]
async fn process_due_actions_returns_zero_when_execution_lock_is_held() {
    with_test_pool(|pool| async move {
        let mut conn = pool.acquire().await.unwrap();
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert!(acquired);

        let service = VillageEsService::new(pool.clone());
        let processed = service
            .process_due_actions(chrono::Utc::now(), 100)
            .await
            .unwrap();
        assert_eq!(processed, 0);

        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .execute(&mut *conn)
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test]
async fn replay_full_mode_preserves_operational_scheduled_actions() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        setup_village(
            &pool,
            &service,
            "Replay Village",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let action_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO rm_scheduled_actions (id, action_type, execute_at, payload, status)
            VALUES ($1, 'TrainUnit', NOW() + interval '10 minutes', '{}'::jsonb, 'pending')
            "#,
        )
        .bind(action_id)
        .execute(&pool)
        .await
        .unwrap();

        let replay = ReplayService::new(pool.clone());
        replay
            .replay(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::Full,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM rm_scheduled_actions WHERE id = $1 AND status = 'pending'",
        )
        .bind(action_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);
    })
    .await;
}

#[tokio::test]
async fn replay_full_mode_rebuilds_marketplace_window_deterministically() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_owner_user_id, owner_player_id, owner_village_id) = setup_village(
            &pool,
            &service,
            "Replay Market Owner",
            Position { x: 0, y: 0 },
            Tribe::Gaul,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(800_000, 800_000, 800_000, 800_000),
        )
        .await;
        let (_acceptor_user_id, acceptor_player_id, acceptor_village_id) = setup_village(
            &pool,
            &service,
            "Replay Market Acceptor",
            Position { x: 5, y: 5 },
            Tribe::Roman,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .create_marketplace_offer(
                owner_village_id,
                &CreateMarketplaceOffer {
                    player_id: owner_player_id,
                    offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 1_000),
                    seek_resources: ResourceQuantity::new(ResourceKind::Iron, 900),
                },
            )
            .await
            .unwrap();

        let offer = service.get_open_marketplace_offers().await.unwrap()[0].clone();
        service
            .accept_marketplace_offer(
                acceptor_village_id,
                acceptor_player_id,
                offer.offer_id,
                Utc::now() + Duration::seconds(2),
                Utc::now() + Duration::seconds(2),
            )
            .await
            .unwrap();

        service
            .process_due_actions(Utc::now() + Duration::seconds(3), 100)
            .await
            .unwrap();

        let owner_before = service.get_village(owner_village_id).await.unwrap();
        let acceptor_before = service.get_village(acceptor_village_id).await.unwrap();
        let offer_before = service.get_marketplace_offer(offer.offer_id).await.unwrap();
        let queue_counts_before: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
              COUNT(*)::bigint,
              COUNT(*) FILTER (WHERE status = 'pending')::bigint,
              COUNT(*) FILTER (WHERE status = 'processing')::bigint,
              COUNT(*) FILTER (WHERE status = 'completed')::bigint
            FROM rm_scheduled_actions
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let replay = ReplayService::new(pool.clone());
        replay
            .replay(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::Full,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        let owner_after = service.get_village(owner_village_id).await.unwrap();
        let acceptor_after = service.get_village(acceptor_village_id).await.unwrap();
        let offer_after = service.get_marketplace_offer(offer.offer_id).await.unwrap();
        let queue_counts_after: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
              COUNT(*)::bigint,
              COUNT(*) FILTER (WHERE status = 'pending')::bigint,
              COUNT(*) FILTER (WHERE status = 'processing')::bigint,
              COUNT(*) FILTER (WHERE status = 'completed')::bigint
            FROM rm_scheduled_actions
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(owner_after.stocks, owner_before.stocks);
        assert_eq!(owner_after.busy_merchants, owner_before.busy_merchants);
        assert_eq!(acceptor_after.stocks, acceptor_before.stocks);
        assert_eq!(acceptor_after.busy_merchants, acceptor_before.busy_merchants);
        assert_eq!(offer_after.status, offer_before.status);
        assert_eq!(queue_counts_after, queue_counts_before);
    })
    .await;
}

#[tokio::test]
async fn replay_full_mode_rebuilds_attack_outcome_window_deterministically() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_user_id, player_id, source_village_id) = setup_village(
            &pool,
            &service,
            "Replay Conquer Source",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![
                main_building(20),
                super::fixtures::rally_point(1),
                VillageBuilding {
                    slot_id: 26,
                    building: Building::new(BuildingName::Palace, 1)
                        .at_level(20, 1)
                        .unwrap(),
                },
                VillageBuilding {
                    slot_id: 28,
                    building: Building::new(BuildingName::GreatWarehouse, 1)
                        .at_level(20, 1)
                        .unwrap(),
                },
                VillageBuilding {
                    slot_id: 29,
                    building: Building::new(BuildingName::GreatGranary, 1)
                        .at_level(20, 1)
                        .unwrap(),
                },
                academy(20),
                super::fixtures::warehouse(20),
                super::fixtures::granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_target_user_id, _target_player_id, target_village_id) = setup_village(
            &pool,
            &service,
            "Replay Conquer Target",
            Position { x: 2, y: 2 },
            Tribe::Teuton,
            vec![
                main_building(1),
                super::fixtures::warehouse(20),
                super::fixtures::granary(20),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        sqlx::query("UPDATE players SET culture_points = 5000 WHERE id = $1")
            .bind(player_id)
            .execute(&pool)
            .await
            .unwrap();

        service
            .research_academy(
                source_village_id,
                &ResearchAcademy {
                    player_id,
                    unit: UnitName::Senator,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::days(7), 400)
            .await
            .unwrap();
        service
            .train_units(
                source_village_id,
                &TrainUnits {
                    player_id,
                    unit_idx: 8,
                    building_name: BuildingName::Palace,
                    quantity: 1,
                    speed: 1,
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(chrono::Utc::now() + chrono::Duration::days(7), 400)
            .await
            .unwrap();

        let now = chrono::Utc::now();
        service
            .send_attack(
                source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id,
                    target_village_id,
                    units: TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 1, 0]),
                    hero_id: None,
                    attack_type: AttackType::Normal,
                    catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
                    arrives_at: now + chrono::Duration::seconds(2),
                    returns_at: now + chrono::Duration::seconds(5),
                },
            )
            .await
            .unwrap();
        service
            .process_due_actions(now + chrono::Duration::seconds(3), 20)
            .await
            .unwrap();

        let rows = sqlx::query_as::<_, (String, String, i64)>(
            r#"
            SELECT aggregate_id, event_type, global_seq
            FROM es_events
            WHERE event_type IN ('AttackBattleResolved', 'BattleOutcomeAppliedToVillage')
            ORDER BY global_seq ASC
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, source_village_id.to_string());
        assert_eq!(rows[0].1, "AttackBattleResolved");
        assert_eq!(rows[1].0, target_village_id.to_string());
        assert_eq!(rows[1].1, "BattleOutcomeAppliedToVillage");
        assert!(rows[0].2 < rows[1].2);

        let before_target = service.get_village(target_village_id).await.unwrap();
        let before_source = service.get_village(source_village_id).await.unwrap();

        let replay = ReplayService::new(pool.clone());
        replay
            .replay(ReplayRequest {
                target: ReplayTarget::Village,
                mode: ReplayMode::Full,
                from_global_seq: 1,
                to_global_seq: None,
                aggregate_id: None,
            })
            .await
            .unwrap();

        let after_target = service.get_village(target_village_id).await.unwrap();
        let after_source = service.get_village(source_village_id).await.unwrap();

        assert_eq!(before_target.player_id, player_id);
        assert_eq!(after_target.player_id, before_target.player_id);
        assert_eq!(after_target.parent_village_id, before_target.parent_village_id);
        assert_eq!(after_target.loyalty, before_target.loyalty);
        assert_eq!(after_target.reinforcements.len(), before_target.reinforcements.len());
        assert_eq!(
            after_source.deployed_armies.len(),
            before_source.deployed_armies.len()
        );
    })
    .await;
}
