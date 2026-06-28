//! Hero resource bonus lookup for village read refreshes.

use parabellum_game::models::hero::{Hero, HeroResourceFocus};
use parabellum_types::{
    common::ResourceGroup,
    errors::{ApplicationError, DbError},
    tribe::Tribe,
};
use sqlx::{FromRow, PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
struct DbHeroResourceBonusRow {
    resources_points: i16,
    resource_focus: Json<HeroResourceFocus>,
}

/// Loads active hero resource bonus for one village using the projection pool.
pub(super) async fn hero_resource_bonus(
    pool: &PgPool,
    village_id: u32,
) -> Result<ResourceGroup, ApplicationError> {
    let row: Option<DbHeroResourceBonusRow> = sqlx::query_as(hero_resource_bonus_sql())
        .bind(village_id as i32)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    Ok(row_to_resource_group(village_id, row))
}

/// Loads active hero resource bonus for one village inside a transaction.
pub(super) async fn hero_resource_bonus_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    village_id: u32,
) -> Result<ResourceGroup, ApplicationError> {
    let row: Option<DbHeroResourceBonusRow> = sqlx::query_as(hero_resource_bonus_sql())
        .bind(village_id as i32)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

    Ok(row_to_resource_group(village_id, row))
}

fn hero_resource_bonus_sql() -> &'static str {
    r#"
        SELECT resources_points, resource_focus
        FROM rm_heroes
        WHERE home_village_id = $1 AND health > 0
        LIMIT 1
        "#
}

fn row_to_resource_group(village_id: u32, row: Option<DbHeroResourceBonusRow>) -> ResourceGroup {
    let Some(row) = row else {
        return ResourceGroup::default();
    };

    let mut hero = Hero::new(None, village_id, Uuid::nil(), Tribe::Roman, None);
    hero.resources_points = row.resources_points.max(0) as u16;
    hero.resource_focus = row.resource_focus.0;
    hero.resources()
}
