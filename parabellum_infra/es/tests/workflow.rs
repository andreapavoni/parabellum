use mini_cqrs_es::{EventMetadata, EventStore, NewEvent};
use parabellum_types::{common::ResourceGroup, map::Position, tribe::Tribe};

use crate::es::{PostgresEventStore, VillageEsService, WorkflowStreamAppend};

use super::fixtures::{main_building, resources, setup_village, with_test_pool};

#[tokio::test]
async fn workflow_append_appends_multiple_streams_atomically() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_u1, p1, v1) = setup_village(
            &pool,
            &service,
            "W1",
            Position { x: 0, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;
        let (_u2, p2, v2) = setup_village(
            &pool,
            &service,
            "W2",
            Position { x: 1, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(pool.clone()));
        let (_, v1_version) = store
            .load_events(aggregate_type, &v1.to_string())
            .await
            .unwrap();
        let (_, v2_version) = store
            .load_events(aggregate_type, &v2.to_string())
            .await
            .unwrap();

        let e1 = NewEvent::from_payload(
            parabellum_app::villages::VillageEvent::VillageResourcesSet {
                player_id: p1,
                village_id: v1,
                resources: ResourceGroup::new(111, 222, 333, 444),
            },
            EventMetadata::default(),
        )
        .unwrap();
        let e2 = NewEvent::from_payload(
            parabellum_app::villages::VillageEvent::VillageResourcesSet {
                player_id: p2,
                village_id: v2,
                resources: ResourceGroup::new(555, 666, 777, 888),
            },
            EventMetadata::default(),
        )
        .unwrap();

        let stored = store
            .append_workflow_events(
                aggregate_type,
                &[
                    WorkflowStreamAppend {
                        aggregate_id: v1.to_string(),
                        expected_version: v1_version,
                        events: vec![e1],
                    },
                    WorkflowStreamAppend {
                        aggregate_id: v2.to_string(),
                        expected_version: v2_version,
                        events: vec![e2],
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].version, v1_version + 1);
        assert_eq!(stored[1].version, v2_version + 1);
    })
    .await;
}

#[tokio::test]
async fn workflow_append_conflict_rolls_back_all_streams() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_u1, p1, v1) = setup_village(
            &pool,
            &service,
            "W3",
            Position { x: 2, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;
        let (_u2, p2, v2) = setup_village(
            &pool,
            &service,
            "W4",
            Position { x: 3, y: 0 },
            Tribe::Roman,
            vec![main_building(1)],
            resources(800, 800, 800, 800),
        )
        .await;

        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(crate::EventStoreDb::new(pool.clone()));
        let (_, v1_version) = store
            .load_events(aggregate_type, &v1.to_string())
            .await
            .unwrap();
        let (_, v2_version) = store
            .load_events(aggregate_type, &v2.to_string())
            .await
            .unwrap();
        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM es_events")
            .fetch_one(&pool)
            .await
            .unwrap();

        let e1 = NewEvent::from_payload(
            parabellum_app::villages::VillageEvent::VillageResourcesSet {
                player_id: p1,
                village_id: v1,
                resources: ResourceGroup::new(901, 902, 903, 904),
            },
            EventMetadata::default(),
        )
        .unwrap();
        let e2 = NewEvent::from_payload(
            parabellum_app::villages::VillageEvent::VillageResourcesSet {
                player_id: p2,
                village_id: v2,
                resources: ResourceGroup::new(905, 906, 907, 908),
            },
            EventMetadata::default(),
        )
        .unwrap();

        let err = store
            .append_workflow_events(
                aggregate_type,
                &[
                    WorkflowStreamAppend {
                        aggregate_id: v1.to_string(),
                        expected_version: v1_version,
                        events: vec![e1],
                    },
                    WorkflowStreamAppend {
                        aggregate_id: v2.to_string(),
                        expected_version: v2_version.saturating_sub(1),
                        events: vec![e2],
                    },
                ],
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, mini_cqrs_es::CqrsError::Conflict { .. }),
            "expected conflict, got: {err}"
        );

        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM es_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count_after, count_before,
            "no stream rows must be appended on conflict"
        );

        let (_, v1_version_after) = store
            .load_events(aggregate_type, &v1.to_string())
            .await
            .unwrap();
        let (_, v2_version_after) = store
            .load_events(aggregate_type, &v2.to_string())
            .await
            .unwrap();
        assert_eq!(v1_version_after, v1_version);
        assert_eq!(v2_version_after, v2_version);
    })
    .await;
}
