//! Query builders for village movement projections.

use parabellum_app::villages::projection_repositories::VillageMovementFilter;
use sqlx::{Postgres, QueryBuilder};

use super::rows::{DbMovementDirection, DbMovementType};

pub(super) fn village_movement_list_query(
    filter: VillageMovementFilter,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        SELECT payload
        FROM rm_village_movements
        "#,
    );
    push_village_movement_filter(&mut query, filter);
    query.push(" ORDER BY eta ASC, movement_id ASC");
    query
}

fn push_village_movement_filter(
    query: &mut QueryBuilder<'static, Postgres>,
    filter: VillageMovementFilter,
) {
    query.push(" WHERE village_id = ");
    query.push_bind(filter.village_id as i32);

    if !filter.directions.is_empty() {
        query.push(" AND direction IN (");
        let mut separated = query.separated(", ");
        for direction in filter.directions {
            separated.push_bind(DbMovementDirection::from(direction));
        }
        separated.push_unseparated(")");
    }

    if !filter.movement_types.is_empty() {
        query.push(" AND movement_type IN (");
        let mut separated = query.separated(", ");
        for movement_type in filter.movement_types {
            separated.push_bind(DbMovementType::from(movement_type));
        }
        separated.push_unseparated(")");
    }
}
