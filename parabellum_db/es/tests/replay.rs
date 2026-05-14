use parabellum_types::{map::Position, tribe::Tribe};

use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::{ReplayMode, ReplayRequest, ReplayService, ReplayTarget, VillageEsService};

use super::fixtures::{main_building, resources, setup_village, with_test_pool};

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
