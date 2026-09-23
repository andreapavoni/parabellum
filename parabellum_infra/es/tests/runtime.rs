//! Regression tests for transaction recovery and bounded aggregate loading.
use mini_cqrs_es::{AggregateSnapshot, EventMetadata, EventStore, NewEvent, SnapshotStore};
use parabellum_app::villages::models::ScheduledActionStatus;
use parabellum_app::villages::projection_repositories::ScheduledActionRepository;
use parabellum_app::villages::{RenameVillage, TrainUnits, VillageAggregate, VillageEvent};
use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};

use super::fixtures::{
    barracks, granary, home_units, main_building, resources, scheduled_action_status_count,
    setup_village, snapshot_version, warehouse, with_test_pool,
};
use crate::es::advisory_lock::AdvisoryLock;
use crate::es::{
    PostgresEventStore, PostgresScheduledActionRepository, PostgresSnapshotStore, ReplayMode,
    ReplayRequest, ReplayService, ReplayTarget, VillageEsService,
};

async fn village(pool: &sqlx::PgPool, service: &VillageEsService) -> (uuid::Uuid, u32) {
    let (_, player, village) = setup_village(
        pool,
        service,
        "Recovery",
        Position { x: 0, y: 0 },
        Tribe::Teuton,
        vec![main_building(1), barracks(1), warehouse(20), granary(20)],
        resources(20000, 20000, 20000, 20000),
    )
    .await;
    (player, village)
}

async fn event_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM es_events")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn reject_snapshots(pool: &sqlx::PgPool) {
    sqlx::raw_sql("CREATE FUNCTION reject_runtime_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected snapshot failure'; END $$; CREATE TRIGGER reject_runtime_snapshot BEFORE INSERT OR UPDATE ON es_snapshots FOR EACH ROW EXECUTE FUNCTION reject_runtime_snapshot();")
        .execute(pool).await.unwrap();
}

async fn allow_snapshots(pool: &sqlx::PgPool) {
    sqlx::raw_sql("DROP TRIGGER reject_runtime_snapshot ON es_snapshots; DROP FUNCTION reject_runtime_snapshot();")
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn runtime_command_snapshot_failure_rolls_back_events_and_projections() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        let before = event_count(&pool).await;
        let version = snapshot_version(&pool, id).await;
        reject_snapshots(&pool).await;
        let result = svc
            .rename_village(
                id,
                &RenameVillage {
                    player_id: player,
                    village_name: "Rollback".into(),
                },
            )
            .await;
        allow_snapshots(&pool).await;
        assert!(result.is_err());
        assert_eq!(event_count(&pool).await, before);
        assert_eq!(snapshot_version(&pool, id).await, version);
        assert_eq!(svc.get_village(id).await.unwrap().village_name, "Recovery");
        svc.rename_village(
            id,
            &RenameVillage {
                player_id: player,
                village_name: "Recovered".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(svc.get_village(id).await.unwrap().village_name, "Recovered");
    })
    .await;
}

#[tokio::test]
async fn runtime_training_duplicate_delivery_has_one_effect_and_continuation() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        svc.train_units(
            id,
            &TrainUnits {
                player_id: player,
                unit_idx: 0,
                building_name: BuildingName::Barracks,
                quantity: 2,
                speed: 1,
            },
        )
        .await
        .unwrap();
        let repo = PostgresScheduledActionRepository::new(crate::ProjectionDb::new(pool.clone()));
        let actions = repo
            .take_due_pending(chrono::Utc::now() + chrono::Duration::days(1), 1)
            .await
            .unwrap();
        assert_eq!(actions.len(), 1);
        // Concurrent delivery and a subsequent redelivery of the same claimed action.
        let (a, b) = tokio::join!(svc.process_actions(&actions), svc.process_actions(&actions));
        a.unwrap();
        b.unwrap();
        let count = event_count(&pool).await;
        svc.process_actions(&actions).await.unwrap();
        assert_eq!(home_units(&pool, id, 0).await, 1);
        assert_eq!(event_count(&pool).await, count);
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Completed).await,
            1
        );
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Pending).await,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn runtime_workflow_snapshot_failure_rolls_back_unit_and_continuation() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        svc.train_units(
            id,
            &TrainUnits {
                player_id: player,
                unit_idx: 0,
                building_name: BuildingName::Barracks,
                quantity: 2,
                speed: 1,
            },
        )
        .await
        .unwrap();
        let repo = PostgresScheduledActionRepository::new(crate::ProjectionDb::new(pool.clone()));
        let actions = repo
            .take_due_pending(chrono::Utc::now() + chrono::Duration::days(1), 1)
            .await
            .unwrap();
        let before = event_count(&pool).await;
        let version = snapshot_version(&pool, id).await;
        reject_snapshots(&pool).await;
        let result = svc.process_actions(&actions).await;
        allow_snapshots(&pool).await;
        result.unwrap();
        assert_eq!(event_count(&pool).await, before);
        assert_eq!(snapshot_version(&pool, id).await, version);
        assert_eq!(home_units(&pool, id, 0).await, 0);
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Pending).await,
            0
        );
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Completed).await,
            0
        );
        // Operator retry after resolving the underlying storage problem.
        repo.update_status(actions[0].id, ScheduledActionStatus::Processing)
            .await
            .unwrap();
        svc.process_actions(&actions).await.unwrap();
        assert_eq!(home_units(&pool, id, 0).await, 1);
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Pending).await,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn runtime_snapshot_tail_is_paged_and_stale_writes_cannot_regress() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        let snapshots = PostgresSnapshotStore::new(crate::EventStoreDb::new(pool.clone()));
        let old = snapshots
            .load_snapshot::<VillageAggregate>(&id)
            .await
            .unwrap();
        let old_aggregate = old.get_payload::<VillageAggregate>().unwrap();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(pool.clone()));
        let events: Vec<_> = (0..600)
            .map(|n| {
                NewEvent::from_payload(
                    VillageEvent::VillageRenamed {
                        village_id: id,
                        player_id: player,
                        village_name: format!("Name {n}"),
                    },
                    EventMetadata::default(),
                )
                .unwrap()
            })
            .collect();
        store
            .save_events(
                std::any::type_name::<VillageAggregate>(),
                &id.to_string(),
                &events,
                old.version,
            )
            .await
            .unwrap();
        // A no-op command proves the loader sees the final tail event.
        svc.rename_village(
            id,
            &RenameVillage {
                player_id: player,
                village_name: "Name 599".into(),
            },
        )
        .await
        .unwrap();
        let expected = old.version + 600;
        assert_eq!(snapshot_version(&pool, id).await, Some(expected));
        snapshots
            .save_snapshot(AggregateSnapshot::new(&old_aggregate, Some(old.version)).unwrap())
            .await
            .unwrap();
        let current = snapshots
            .load_snapshot::<VillageAggregate>(&id)
            .await
            .unwrap();
        assert_eq!(current.version, expected);
        assert_eq!(
            current
                .get_payload::<VillageAggregate>()
                .unwrap()
                .village()
                .village
                .name,
            "Name 599"
        );
        sqlx::query("DELETE FROM es_snapshots")
            .execute(&pool)
            .await
            .unwrap();
        svc.rename_village(
            id,
            &RenameVillage {
                player_id: player,
                village_name: "Rebuilt".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(svc.get_village(id).await.unwrap().village_name, "Rebuilt");
        assert_eq!(snapshot_version(&pool, id).await, Some(expected + 1));
    })
    .await;
}

#[tokio::test]
async fn runtime_workflow_does_not_decode_history_covered_by_snapshot() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        svc.train_units(
            id,
            &TrainUnits {
                player_id: player,
                unit_idx: 0,
                building_name: BuildingName::Barracks,
                quantity: 1,
                speed: 1,
            },
        )
        .await
        .unwrap();
        // A poisoned old payload is a sentinel: fetching and replaying the full
        // history would fail even though the current snapshot already covers it.
        sqlx::query(
            "UPDATE es_events SET payload = '{}' WHERE aggregate_id = $1 AND stream_version = 1",
        )
        .bind(id.to_string())
        .execute(&pool)
        .await
        .unwrap();
        svc.process_due_actions(chrono::Utc::now() + chrono::Duration::days(1), 1)
            .await
            .unwrap();
        assert_eq!(home_units(&pool, id, 0).await, 1);
        assert_eq!(
            scheduled_action_status_count(&pool, ScheduledActionStatus::Completed).await,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn runtime_cancelled_lock_owner_releases_session_lock() {
    with_test_pool(|pool| async move {
        let key = 829_443_765;
        let (ready, waiting) = tokio::sync::oneshot::channel();
        let worker_pool = pool.clone();
        let task = tokio::spawn(async move {
            let _lock = AdvisoryLock::try_acquire(&worker_pool, key)
                .await
                .unwrap()
                .unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        waiting.await.unwrap();
        assert!(
            AdvisoryLock::try_acquire(&pool, key)
                .await
                .unwrap()
                .is_none()
        );
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(lock) = AdvisoryLock::try_acquire(&pool, key).await.unwrap() {
                    lock.release().await.unwrap();
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        for _ in 0..5 {
            AdvisoryLock::try_acquire(&pool, key)
                .await
                .unwrap()
                .unwrap()
                .release()
                .await
                .unwrap();
        }
    })
    .await;
}

#[tokio::test]
async fn runtime_filtered_rebuild_is_rejected_without_mutation() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (_, id) = village(&pool, &svc).await;
        let replay = ReplayService::new(pool.clone());
        let base = ReplayRequest {
            target: ReplayTarget::All,
            mode: ReplayMode::Full,
            from_global_seq: 1,
            to_global_seq: None,
            aggregate_id: None,
        };
        let mut requests = vec![base.clone(); 3];
        requests[0].from_global_seq = 2;
        requests[1].to_global_seq = Some(2);
        requests[2].aggregate_id = Some(id.to_string());
        for request in requests {
            assert!(replay.replay(request).await.is_err());
            assert_eq!(svc.get_village(id).await.unwrap().village_name, "Recovery");
        }
    })
    .await;
}

#[tokio::test]
async fn runtime_replay_failure_restores_previous_projection() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (_, id) = village(&pool, &svc).await;
        let replay = ReplayService::new(pool.clone());
        // Failure after a valid foundation event has already been projected.
        sqlx::query("UPDATE es_events SET payload = '{}' WHERE aggregate_id = $1 AND stream_version = (SELECT MAX(stream_version) FROM es_events WHERE aggregate_id = $1)")
            .bind(id.to_string()).execute(&pool).await.unwrap();
        let before = svc.get_village(id).await.unwrap();
        let result = replay.replay(ReplayRequest { target: ReplayTarget::All, mode: ReplayMode::Full,
            from_global_seq: 1, to_global_seq: None, aggregate_id: None }).await;
        assert!(result.is_err());
        let after = svc.get_village(id).await.unwrap();
        assert_eq!(after.village_name, before.village_name);
        assert_eq!(after.stocks, before.stocks);
    }).await;
}

#[tokio::test]
async fn runtime_completion_status_failure_rolls_back_training_effects() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        svc.train_units(id, &TrainUnits { player_id: player, unit_idx: 0,
            building_name: BuildingName::Barracks, quantity: 2, speed: 1 }).await.unwrap();
        let before = event_count(&pool).await;
        sqlx::raw_sql("CREATE FUNCTION reject_runtime_completion() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.status = 'completed' THEN RAISE EXCEPTION 'injected completion failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_runtime_completion BEFORE UPDATE ON rm_scheduled_actions FOR EACH ROW EXECUTE FUNCTION reject_runtime_completion();")
            .execute(&pool).await.unwrap();
        let result = svc.process_due_actions(chrono::Utc::now() + chrono::Duration::days(1), 1).await;
        sqlx::raw_sql("DROP TRIGGER reject_runtime_completion ON rm_scheduled_actions; DROP FUNCTION reject_runtime_completion();")
            .execute(&pool).await.unwrap();
        result.unwrap();
        assert_eq!(event_count(&pool).await, before);
        assert_eq!(home_units(&pool, id, 0).await, 0);
        assert_eq!(scheduled_action_status_count(&pool, ScheduledActionStatus::Pending).await, 0);
        assert_eq!(scheduled_action_status_count(&pool, ScheduledActionStatus::Failed).await, 1);
    }).await;
}

#[tokio::test]
async fn runtime_transient_failures_have_persistent_bounded_retries() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        let (player, id) = village(&pool, &svc).await;
        svc.train_units(id, &TrainUnits { player_id: player, unit_idx: 0,
            building_name: BuildingName::Barracks, quantity: 2, speed: 1 }).await.unwrap();
        let before = event_count(&pool).await;
        sqlx::raw_sql("CREATE FUNCTION reject_runtime_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected serialization failure' USING ERRCODE = '40001'; END $$; CREATE TRIGGER reject_runtime_snapshot BEFORE INSERT OR UPDATE ON es_snapshots FOR EACH ROW EXECUTE FUNCTION reject_runtime_snapshot();")
            .execute(&pool).await.unwrap();
        let mut results = Vec::new();
        for _ in 0..5 {
            // New service instances cannot reset the database-backed attempt budget.
            results.push(VillageEsService::new(pool.clone())
                .process_due_actions(chrono::Utc::now() + chrono::Duration::days(1), 1).await);
        }
        allow_snapshots(&pool).await;
        for result in results { assert_eq!(result.unwrap(), 1); }
        assert_eq!(event_count(&pool).await, before);
        assert_eq!(home_units(&pool, id, 0).await, 0);
        assert_eq!(scheduled_action_status_count(&pool, ScheduledActionStatus::Failed).await, 1);
        assert_eq!(scheduled_action_status_count(&pool, ScheduledActionStatus::Pending).await, 0);
        let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM rm_scheduled_actions LIMIT 1")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(attempts, 5);
    }).await;
}

#[tokio::test]
async fn runtime_scheduler_query_error_does_not_leak_execution_lock() {
    with_test_pool(|pool| async move {
        let svc = VillageEsService::new(pool.clone());
        sqlx::query("ALTER TABLE rm_scheduled_actions RENAME TO runtime_hidden_actions")
            .execute(&pool)
            .await
            .unwrap();
        let result = svc.process_due_actions(chrono::Utc::now(), 1).await;
        sqlx::query("ALTER TABLE runtime_hidden_actions RENAME TO rm_scheduled_actions")
            .execute(&pool)
            .await
            .unwrap();
        assert!(result.is_err());
        let key = crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(lock) = AdvisoryLock::try_acquire(&pool, key).await.unwrap() {
                    lock.release().await.unwrap();
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            svc.process_due_actions(chrono::Utc::now(), 1)
                .await
                .unwrap(),
            0
        );
    })
    .await;
}
