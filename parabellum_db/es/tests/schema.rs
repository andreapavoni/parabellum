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

    let scout_movement_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'movement_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'Scout'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scout_movement_variant_exists, 1);

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
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'scheduled_action_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'ResearchSmithy'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(smithy_variant_exists, 1);

    let attack_arrival_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'scheduled_action_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'AttackArrival'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attack_arrival_variant_exists, 1);

    let settlers_arrival_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'scheduled_action_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'SettlersArrival'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(settlers_arrival_variant_exists, 1);

    let army_return_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'scheduled_action_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'ArmyReturn'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(army_return_variant_exists, 1);

    let scout_arrival_variant_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'scheduled_action_type'
          AND n.nspname = 'public'
          AND e.enumlabel = 'ScoutArrival'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scout_arrival_variant_exists, 1);
    let rm_reports_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM information_schema.tables
        WHERE table_schema = 'public'
          AND table_name = 'rm_reports'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rm_reports_exists, 1);

    let rm_report_reads_exists: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM information_schema.tables
        WHERE table_schema = 'public'
          AND table_name = 'rm_report_reads'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rm_report_reads_exists, 1);
    })
    .await;
}
