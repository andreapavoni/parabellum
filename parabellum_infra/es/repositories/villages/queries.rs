//! Query builders and shared SQL fragments for village projections.

use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

pub(super) fn village_by_id_query(village_id: u32) -> QueryBuilder<'static, Postgres> {
    let mut query = village_select_query();
    query.push(" WHERE village_id = ");
    query.push_bind(village_id as i32);
    query
}

pub(super) fn villages_by_player_query(player_id: Uuid) -> QueryBuilder<'static, Postgres> {
    let mut query = village_select_query();
    query.push(" WHERE player_id = ");
    query.push_bind(player_id);
    query.push(" ORDER BY village_id ASC");
    query
}

pub(super) fn villages_by_ids_query(village_ids: Vec<i32>) -> QueryBuilder<'static, Postgres> {
    let mut query = village_select_query();
    query.push(" WHERE village_id = ANY(");
    query.push_bind(village_ids);
    query.push(") ORDER BY village_id ASC");
    query
}

fn village_select_query() -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("");
    query.push(village_select_sql());
    query
}

/// Base projection SELECT used to hydrate `VillageModel` rows.
pub(super) fn village_select_sql() -> &'static str {
    r#"
    SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
           population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
           total_merchants, busy_merchants, trapper_active_traps, trapper_broken_traps, trapper_queued_traps, loyalty_updated_at, updated_at
    FROM rm_village
    "#
}
