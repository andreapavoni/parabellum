use sqlx::Row;

use super::fixtures::with_test_pool;

#[tokio::test]
async fn cqrs_es_schema_uses_native_postgres_enums_for_projected_models() {
    with_test_pool(|pool| async move {

    let enum_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_type t
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname = 'public'
          AND t.typname IN ('movement_direction', 'movement_type', 'scheduled_action_status', 'scheduled_action_type')
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(enum_count, 4);

    let movement_direction = sqlx::query(
        r#"
        SELECT data_type, udt_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'rm_village_movements'
          AND column_name = 'direction'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(movement_direction.get::<String, _>("data_type"), "USER-DEFINED");
    assert_eq!(
        movement_direction.get::<String, _>("udt_name"),
        "movement_direction"
    );

    let movement_type = sqlx::query(
        r#"
        SELECT data_type, udt_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'rm_village_movements'
          AND column_name = 'movement_type'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(movement_type.get::<String, _>("data_type"), "USER-DEFINED");
    assert_eq!(movement_type.get::<String, _>("udt_name"), "movement_type");

    let action_status = sqlx::query(
        r#"
        SELECT data_type, udt_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'rm_scheduled_actions'
          AND column_name = 'status'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(action_status.get::<String, _>("data_type"), "USER-DEFINED");
    assert_eq!(
        action_status.get::<String, _>("udt_name"),
        "scheduled_action_status"
    );

    let action_type = sqlx::query(
        r#"
        SELECT data_type, udt_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'rm_scheduled_actions'
          AND column_name = 'action_type'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(action_type.get::<String, _>("data_type"), "USER-DEFINED");
    assert_eq!(
        action_type.get::<String, _>("udt_name"),
        "scheduled_action_type"
    );

    let smithy_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        WHERE t.typname = 'scheduled_action_type'
          AND e.enumlabel = 'ResearchSmithy'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(smithy_variant_exists, 1);
    })
    .await;
}
