//! Query builders for village movement projections.

use parabellum_app::villages::{
    models::VillageMovement, projection_repositories::VillageMovementFilter,
};
use sqlx::{Postgres, QueryBuilder, types::Json};

use super::rows::{DbMovementDirection, DbMovementType};

pub(super) fn upsert_village_movement_query(
    movement: &VillageMovement,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        INSERT INTO rm_village_movements (
            village_id, movement_id, direction, movement_type, source_village_id,
            target_village_id, eta, payload
        )
        VALUES (
        "#,
    );
    query.push_bind(movement.viewing_village_id as i32);
    query.push(", ");
    query.push_bind(movement.movement_id);
    query.push(", ");
    query.push_bind(DbMovementDirection::from(movement.direction));
    query.push(", ");
    query.push_bind(DbMovementType::from(movement.movement_type));
    query.push(", ");
    query.push_bind(movement.origin_village_id as i32);
    query.push(", ");
    query.push_bind(movement.target_village_id as i32);
    query.push(", ");
    query.push_bind(movement.arrives_at);
    query.push(", ");
    query.push_bind(Json(movement.clone()));
    query.push(
        r#"
        )
        ON CONFLICT (village_id, movement_id, direction)
        DO UPDATE SET
            movement_type = EXCLUDED.movement_type,
            source_village_id = EXCLUDED.source_village_id,
            target_village_id = EXCLUDED.target_village_id,
            eta = EXCLUDED.eta,
            payload = EXCLUDED.payload,
            updated_at = NOW()
        "#,
    );
    query
}

pub(super) fn village_movement_list_query(
    filter: VillageMovementFilter,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        SELECT village_id, payload
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
