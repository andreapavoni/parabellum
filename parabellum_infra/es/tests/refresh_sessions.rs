//! Session races run against real PostgreSQL transactions and an isolated database.
use super::fixtures::{resources, setup_village, with_test_pool};
use crate::{es::VillageEsService, identity::repositories::PostgresRefreshSessionRepository};
use chrono::{Duration, Utc};
use parabellum_app::identity::refresh_sessions::{
    RefreshSession, RefreshSessionError, RefreshSessionRepository, RefreshSessionUseCases,
    SessionClient,
};
use parabellum_types::{map::Position, tribe::Tribe};
use std::sync::Arc;

async fn setup(
    pool: &sqlx::PgPool,
) -> (
    Arc<PostgresRefreshSessionRepository>,
    RefreshSessionUseCases,
    RefreshSession,
) {
    let service = VillageEsService::new(pool.clone());
    let (user, player, village) = setup_village(
        pool,
        &service,
        "Sessions",
        Position { x: 0, y: 0 },
        Tribe::Roman,
        vec![],
        resources(500, 500, 500, 500),
    )
    .await;
    let repo = Arc::new(PostgresRefreshSessionRepository::new(pool.clone()));
    let sessions = RefreshSessionUseCases::new(
        repo.clone(),
        3600,
        Arc::new(parabellum_app::villages::SystemClock),
        Arc::new(parabellum_app::villages::UuidGenerator),
    );
    let session = sessions
        .create(user, player, village, "old", SessionClient::default())
        .await
        .unwrap();
    (repo, sessions, session)
}

#[tokio::test]
async fn refresh_concurrent_rotation_has_exactly_one_successor() {
    with_test_pool(|pool| async move {
        let (_, sessions, _) = setup(&pool).await;
        let (a,b) = tokio::join!(sessions.rotate("old", "next-a".into(), SessionClient::default()),
            sessions.rotate("old", "next-b".into(), SessionClient::default()));
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let error = if a.is_err() { a.unwrap_err() } else { b.unwrap_err() };
        assert!(matches!(error, RefreshSessionError::Revoked));
        let counts: (i64,i64) = sqlx::query_as("SELECT COUNT(*), COUNT(*) FILTER (WHERE revoked_at IS NULL) FROM auth_refresh_sessions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(counts, (2,1));
    }).await;
}

#[tokio::test]
async fn refresh_failed_successor_insert_keeps_previous_token_usable() {
    with_test_pool(|pool| async move {
        let (_, sessions, _) = setup(&pool).await;
        // The unique hash constraint fails after revocation within the transaction.
        assert!(matches!(
            sessions
                .rotate("old", "old".into(), SessionClient::default())
                .await,
            Err(RefreshSessionError::Storage(_))
        ));
        sessions.validate_hash("old").await.unwrap();
        sessions
            .rotate("old", "valid".into(), SessionClient::default())
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test]
async fn refresh_revocation_and_expiration_prevent_rotation() {
    with_test_pool(|pool| async move {
        let (repo, sessions, old) = setup(&pool).await;
        sessions.revoke("old").await.unwrap();
        assert!(matches!(
            sessions
                .rotate("old", "next".into(), SessionClient::default())
                .await,
            Err(RefreshSessionError::Revoked)
        ));
        let expired = RefreshSession {
            id: uuid::Uuid::new_v4(),
            expires_at: Utc::now() - Duration::seconds(1),
            ..old
        };
        repo.create(&expired, "expired", SessionClient::default())
            .await
            .unwrap();
        assert!(matches!(
            sessions
                .rotate("expired", "next".into(), SessionClient::default())
                .await,
            Err(RefreshSessionError::Expired)
        ));
    })
    .await;
}

#[tokio::test]
async fn refresh_logout_all_race_leaves_no_active_successor() {
    with_test_pool(|pool| async move {
        let (_, sessions, old) = setup(&pool).await;
        let (rotation, logout) = tokio::join!(
            sessions.rotate("old", "next".into(), SessionClient::default()),
            sessions.revoke_all(old.user_id)
        );
        logout.unwrap();
        assert!(rotation.is_ok() || matches!(rotation, Err(RefreshSessionError::Revoked)));
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM auth_refresh_sessions WHERE revoked_at IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
    })
    .await;
}

#[tokio::test]
async fn refresh_single_logout_race_is_serialized_at_the_old_session() {
    with_test_pool(|pool| async move {
        let (_, sessions, _) = setup(&pool).await;
        let (rotation, logout) = tokio::join!(
            sessions.rotate("old", "next".into(), SessionClient::default()),
            sessions.revoke("old")
        );
        logout.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM auth_refresh_sessions WHERE revoked_at IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        match rotation {
            Ok(_) => assert_eq!(count, 1), // Rotation won; logout addresses the consumed token.
            Err(RefreshSessionError::Revoked) => assert_eq!(count, 0),
            other => panic!("unexpected rotation outcome: {other:?}"),
        }
    })
    .await;
}
