//! SQL builders for army projections.

use parabellum_app::villages::projection_repositories::{ArmyListFilter, ArmyState};
use parabellum_game::models::army::Army;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

pub(super) fn upsert_army_query(
    army: &Army,
    current_village_id: u32,
    player_id: Uuid,
    state: ArmyState,
) -> QueryBuilder<'static, Postgres> {
    let units: Vec<i32> = army
        .units()
        .units()
        .iter()
        .map(|value| *value as i32)
        .collect();
    let smithy_upgrades: Vec<i16> = army.smithy().iter().map(|value| *value as i16).collect();
    let hero_id = army.hero().map(|hero| hero.id);
    let tribe: crate::persistence::models::Tribe = army.tribe.clone().into();

    let mut query = QueryBuilder::new(
        r#"
        INSERT INTO rm_armies (
            army_id, village_id, current_village_id, current_map_field_id, player_id, tribe,
            state, units, smithy_upgrades, hero_id, updated_at
        )
        VALUES (
        "#,
    );
    query.push_bind(army.id);
    query.push(", ");
    query.push_bind(army.village_id as i32);
    query.push(", ");
    query.push_bind(current_village_id as i32);
    query.push(", ");
    query.push_bind(army.current_map_field_id.map(|id| id as i32));
    query.push(", ");
    query.push_bind(player_id);
    query.push(", ");
    query.push_bind(tribe);
    query.push(", ");
    query.push_bind(state_name(state));
    query.push(", ");
    query.push_bind(units);
    query.push(", ");
    query.push_bind(smithy_upgrades);
    query.push(", ");
    query.push_bind(hero_id);
    query.push(
        r#",
            NOW()
        )
        ON CONFLICT (army_id) DO UPDATE SET
            village_id = EXCLUDED.village_id,
            current_village_id = EXCLUDED.current_village_id,
            current_map_field_id = EXCLUDED.current_map_field_id,
            player_id = EXCLUDED.player_id,
            tribe = EXCLUDED.tribe,
            state = EXCLUDED.state,
            units = EXCLUDED.units,
            smithy_upgrades = EXCLUDED.smithy_upgrades,
            hero_id = EXCLUDED.hero_id,
            updated_at = NOW()
        "#,
    );
    query
}

pub(super) fn delete_other_home_armies_query(
    village_id: u32,
    keep_army_id: Uuid,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        DELETE FROM rm_armies
        WHERE village_id =
        "#,
    );
    query.push_bind(village_id as i32);
    query.push(" AND state = ");
    query.push_bind(state_name(ArmyState::Home));
    query.push(" AND army_id <> ");
    query.push_bind(keep_army_id);
    query
}

pub(super) fn delete_army_query(army_id: Uuid) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("DELETE FROM rm_armies WHERE army_id = ");
    query.push_bind(army_id);
    query
}

pub(super) fn delete_armies_by_home_village_query(
    village_id: u32,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("DELETE FROM rm_armies WHERE village_id = ");
    query.push_bind(village_id as i32);
    query
}

pub(super) fn army_query(
    filter: ArmyListFilter,
) -> Result<QueryBuilder<'static, Postgres>, ApplicationError> {
    let mut query = QueryBuilder::<Postgres>::new(army_select_sql());
    let mut has_where = false;

    if let Some(army_id) = filter.army_id {
        push_filter(&mut query, &mut has_where);
        query.push("a.army_id = ");
        query.push_bind(army_id);
    }

    if let Some(home_village_id) = filter.home_village_id {
        push_filter(&mut query, &mut has_where);
        query.push("a.village_id = ");
        query.push_bind(home_village_id as i32);
    }

    if let Some(current_village_id) = filter.current_village_id {
        push_filter(&mut query, &mut has_where);
        query.push("a.current_village_id = ");
        query.push_bind(current_village_id as i32);
    }

    if let Some(state) = filter.state {
        push_filter(&mut query, &mut has_where);
        query.push("a.state = ");
        query.push_bind(state_name(state));
    }

    if let Some(deployed) = filter.deployed {
        let Some(home_village_id) = filter.home_village_id else {
            return Err(ApplicationError::Db(DbError::Database(
                sqlx::Error::Protocol("army deployed filter requires home_village_id".into()),
            )));
        };
        push_filter(&mut query, &mut has_where);
        if deployed {
            query.push("a.current_village_id <> ");
        } else {
            query.push("a.current_village_id = ");
        }
        query.push_bind(home_village_id as i32);
    }

    query.push(" ORDER BY a.updated_at DESC");

    if let Some(limit) = filter.limit {
        query.push(" LIMIT ");
        query.push_bind(limit);
    }

    Ok(query)
}

pub(super) fn village_army_context_query(village_id: u32) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::<Postgres>::new(army_select_sql());
    query.push(" WHERE a.village_id = ");
    query.push_bind(village_id as i32);
    query.push(" OR a.current_village_id = ");
    query.push_bind(village_id as i32);
    query.push(" ORDER BY a.updated_at DESC");
    query
}

fn army_select_sql() -> &'static str {
    r#"
    SELECT
      a.army_id,
      a.village_id,
      a.current_village_id,
      a.current_map_field_id,
      a.player_id,
      a.tribe,
      a.state,
      a.units,
      a.smithy_upgrades,
      a.hero_id,
      h.player_id AS hero_player_id,
      h.home_village_id AS hero_home_village_id,
      h.tribe AS hero_tribe,
      h.level AS hero_level,
      h.health AS hero_health,
      h.experience AS hero_experience,
      h.resource_focus AS hero_resource_focus,
      h.strength_points AS hero_strength_points,
      h.off_bonus_points AS hero_off_bonus_points,
      h.def_bonus_points AS hero_def_bonus_points,
      h.regeneration_points AS hero_regeneration_points,
      h.resources_points AS hero_resources_points,
      h.unassigned_points AS hero_unassigned_points
    FROM rm_armies a
    LEFT JOIN rm_heroes h ON h.hero_id = a.hero_id
    "#
}

fn push_filter(query: &mut QueryBuilder<'static, Postgres>, has_where: &mut bool) {
    if *has_where {
        query.push(" AND ");
    } else {
        query.push(" WHERE ");
        *has_where = true;
    }
}

fn state_name(state: ArmyState) -> &'static str {
    match state {
        ArmyState::Home => "home",
        ArmyState::Stationed => "stationed",
        ArmyState::Moving => "moving",
        ArmyState::Trapped => "trapped",
    }
}
