//! Expansion snapshot queries for village projections.

use parabellum_app::villages::projection_repositories::{
    ExpansionCultureSnapshot, ExpansionOwnershipSnapshot,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct DbExpansionCultureSnapshotRow {
    village_cpp: i32,
    player_cpp: i64,
    village_count: i64,
}

#[derive(sqlx::FromRow)]
struct DbExpansionOwnershipSnapshotRow {
    source_child_villages: i64,
    player_village_count: i64,
}

/// Returns culture-point production and village count data for expansion reads.
pub(super) async fn get_expansion_culture_snapshot(
    pool: &PgPool,
    player_id: Uuid,
    village_id: u32,
) -> Result<ExpansionCultureSnapshot, ApplicationError> {
    let row: Option<DbExpansionCultureSnapshotRow> = sqlx::query_as(
        r#"
        SELECT
          v.culture_points_production AS village_cpp,
          SUM(rv.culture_points_production)::bigint AS player_cpp,
          COUNT(rv.village_id)::bigint AS village_count
        FROM rm_village v
        JOIN rm_village rv ON rv.player_id = v.player_id
        WHERE v.village_id = $1
          AND v.player_id = $2
        GROUP BY v.village_id, v.culture_points_production
        "#,
    )
    .bind(village_id as i32)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    let Some(row) = row else {
        return Err(ApplicationError::Db(DbError::VillageNotFound(village_id)));
    };

    Ok(ExpansionCultureSnapshot {
        village_culture_points_production: row.village_cpp as u32,
        player_culture_points_production: row.player_cpp.max(0) as u32,
        player_village_count: row.village_count.max(0) as usize,
    })
}

/// Counts villages founded from the selected parent village.
pub(super) async fn count_child_villages(
    pool: &PgPool,
    player_id: Uuid,
    parent_village_id: u32,
) -> Result<u8, ApplicationError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM rm_village
        WHERE player_id = $1
          AND parent_village_id = $2
        "#,
    )
    .bind(player_id)
    .bind(parent_village_id as i32)
    .fetch_one(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    Ok(count.clamp(0, u8::MAX as i64) as u8)
}

/// Returns ownership counters needed to validate expansion from a source village.
pub(super) async fn get_expansion_ownership_snapshot(
    pool: &PgPool,
    player_id: Uuid,
    source_village_id: u32,
) -> Result<ExpansionOwnershipSnapshot, ApplicationError> {
    let row: DbExpansionOwnershipSnapshotRow = sqlx::query_as(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE parent_village_id = $1)::bigint AS source_child_villages,
          COUNT(*)::bigint AS player_village_count
        FROM rm_village
        WHERE player_id = $2
        "#,
    )
    .bind(source_village_id as i32)
    .bind(player_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    Ok(ExpansionOwnershipSnapshot {
        source_child_villages: row.source_child_villages.clamp(0, u8::MAX as i64) as u8,
        player_village_count: row.player_village_count.max(0) as usize,
    })
}
