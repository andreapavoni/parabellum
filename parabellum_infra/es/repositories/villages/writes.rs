//! Full-model write SQL builders for village projections.

use parabellum_app::villages::models::VillageModel;
use sqlx::{Postgres, QueryBuilder, types::Json};

use super::rows::DbTribe;

pub(super) fn upsert_village_model_query(model: &VillageModel) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        INSERT INTO rm_village (
            village_id, player_id, village_name, position, tribe, buildings, production, stocks,
            population, loyalty, is_capital, culture_points_production, smithy_upgrades,
            academy_research, parent_village_id, total_merchants, busy_merchants,
            trapper_active_traps, trapper_broken_traps, trapper_queued_traps, loyalty_updated_at,
            updated_at
        )
        VALUES (
        "#,
    );
    push_village_model_insert_values(&mut query, model);
    query.push(
        r#"
        )
        ON CONFLICT (village_id)
        DO UPDATE SET
            player_id = EXCLUDED.player_id,
            village_name = EXCLUDED.village_name,
            position = EXCLUDED.position,
            tribe = EXCLUDED.tribe,
            buildings = EXCLUDED.buildings,
            production = EXCLUDED.production,
            stocks = EXCLUDED.stocks,
            population = EXCLUDED.population,
            loyalty = EXCLUDED.loyalty,
            is_capital = EXCLUDED.is_capital,
            culture_points_production = EXCLUDED.culture_points_production,
            smithy_upgrades = EXCLUDED.smithy_upgrades,
            academy_research = EXCLUDED.academy_research,
            parent_village_id = EXCLUDED.parent_village_id,
            total_merchants = EXCLUDED.total_merchants,
            busy_merchants = EXCLUDED.busy_merchants,
            trapper_active_traps = EXCLUDED.trapper_active_traps,
            trapper_broken_traps = EXCLUDED.trapper_broken_traps,
            trapper_queued_traps = EXCLUDED.trapper_queued_traps,
            loyalty_updated_at = EXCLUDED.loyalty_updated_at,
            updated_at = EXCLUDED.updated_at
        "#,
    );
    query
}

pub(super) fn store_village_model_query(model: &VillageModel) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("UPDATE rm_village SET ");
    push_village_model_update_assignments(&mut query, model);
    query.push(", loyalty_updated_at = CASE WHEN loyalty <> ");
    query.push_bind(model.loyalty as i16);
    query.push(
        r#"
                THEN NOW()
                ELSE loyalty_updated_at
            END,
            updated_at = NOW()
        WHERE village_id =
        "#,
    );
    query.push_bind(model.village_id as i32);
    query
}

fn push_village_model_insert_values(
    query: &mut QueryBuilder<'static, Postgres>,
    model: &VillageModel,
) {
    query.push_bind(model.village_id as i32);
    query.push(", ");
    push_village_model_common_values(query, model);
    query.push(", ");
    query.push_bind(model.loyalty_updated_at);
    query.push(", ");
    query.push_bind(model.updated_at);
}

fn push_village_model_update_assignments(
    query: &mut QueryBuilder<'static, Postgres>,
    model: &VillageModel,
) {
    query.push("player_id = ");
    query.push_bind(model.player_id);
    query.push(", village_name = ");
    query.push_bind(model.village_name.clone());
    query.push(", position = ");
    query.push_bind(Json(model.position.clone()));
    query.push(", tribe = ");
    query.push_bind(DbTribe::from(model.tribe.clone()));
    query.push(", buildings = ");
    query.push_bind(Json(model.buildings.clone()));
    query.push(", production = ");
    query.push_bind(Json(model.production.clone()));
    query.push(", stocks = ");
    query.push_bind(Json(model.stocks.clone()));
    query.push(", population = ");
    query.push_bind(model.population as i32);
    query.push(", loyalty = ");
    query.push_bind(model.loyalty as i16);
    query.push(", is_capital = ");
    query.push_bind(model.is_capital);
    query.push(", culture_points_production = ");
    query.push_bind(model.culture_points_production as i32);
    query.push(", smithy_upgrades = ");
    query.push_bind(Json(model.smithy_upgrades));
    query.push(", academy_research = ");
    query.push_bind(Json(model.academy_research.clone()));
    query.push(", parent_village_id = ");
    query.push_bind(model.parent_village_id.map(|id| id as i32));
    query.push(", total_merchants = ");
    query.push_bind(model.total_merchants as i16);
    query.push(", busy_merchants = ");
    query.push_bind(model.busy_merchants as i16);
    query.push(", trapper_active_traps = ");
    query.push_bind(model.trapper.active_traps as i32);
    query.push(", trapper_broken_traps = ");
    query.push_bind(model.trapper.broken_traps as i32);
    query.push(", trapper_queued_traps = ");
    query.push_bind(model.trapper.queued_traps as i32);
}

fn push_village_model_common_values(
    query: &mut QueryBuilder<'static, Postgres>,
    model: &VillageModel,
) {
    query.push_bind(model.player_id);
    query.push(", ");
    query.push_bind(model.village_name.clone());
    query.push(", ");
    query.push_bind(Json(model.position.clone()));
    query.push(", ");
    query.push_bind(DbTribe::from(model.tribe.clone()));
    query.push(", ");
    query.push_bind(Json(model.buildings.clone()));
    query.push(", ");
    query.push_bind(Json(model.production.clone()));
    query.push(", ");
    query.push_bind(Json(model.stocks.clone()));
    query.push(", ");
    query.push_bind(model.population as i32);
    query.push(", ");
    query.push_bind(model.loyalty as i16);
    query.push(", ");
    query.push_bind(model.is_capital);
    query.push(", ");
    query.push_bind(model.culture_points_production as i32);
    query.push(", ");
    query.push_bind(Json(model.smithy_upgrades));
    query.push(", ");
    query.push_bind(Json(model.academy_research.clone()));
    query.push(", ");
    query.push_bind(model.parent_village_id.map(|id| id as i32));
    query.push(", ");
    query.push_bind(model.total_merchants as i16);
    query.push(", ");
    query.push_bind(model.busy_merchants as i16);
    query.push(", ");
    query.push_bind(model.trapper.active_traps as i32);
    query.push(", ");
    query.push_bind(model.trapper.broken_traps as i32);
    query.push(", ");
    query.push_bind(model.trapper.queued_traps as i32);
}
