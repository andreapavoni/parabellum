//! SQL builders for hero projections.

use parabellum_app::villages::projection_repositories::HeroPlacementState;
use parabellum_game::models::hero::Hero;
use sqlx::{Postgres, QueryBuilder, types::Json};
use uuid::Uuid;

pub(super) fn upsert_hero_query(
    hero: &Hero,
    home_village_id: u32,
    current_village_id: u32,
    state: HeroPlacementState,
) -> QueryBuilder<'static, Postgres> {
    let tribe: crate::persistence::models::Tribe = hero.tribe.clone().into();
    let mut query = QueryBuilder::new(
        r#"
        INSERT INTO rm_heroes (
            hero_id, player_id, home_village_id, current_village_id, state, tribe, level,
            health, experience, resource_focus, strength_points, off_bonus_points,
            def_bonus_points, regeneration_points, resources_points, unassigned_points
        )
        VALUES (
        "#,
    );
    query.push_bind(hero.id);
    query.push(", ");
    query.push_bind(hero.player_id);
    query.push(", ");
    query.push_bind(home_village_id as i32);
    query.push(", ");
    query.push_bind(current_village_id as i32);
    query.push(", ");
    query.push_bind(hero_state_name(state));
    query.push(", ");
    query.push_bind(tribe);
    query.push(", ");
    query.push_bind(hero.level as i16);
    query.push(", ");
    query.push_bind(hero.health as i16);
    query.push(", ");
    query.push_bind(hero.experience as i32);
    query.push(", ");
    query.push_bind(Json(hero.resource_focus.clone()));
    query.push(", ");
    query.push_bind(hero.strength_points as i16);
    query.push(", ");
    query.push_bind(hero.off_bonus_points as i16);
    query.push(", ");
    query.push_bind(hero.def_bonus_points as i16);
    query.push(", ");
    query.push_bind(hero.regeneration_points as i16);
    query.push(", ");
    query.push_bind(hero.resources_points as i16);
    query.push(", ");
    query.push_bind(hero.unassigned_points as i16);
    query.push(
        r#"
        )
        ON CONFLICT (hero_id) DO UPDATE SET
            player_id = EXCLUDED.player_id,
            home_village_id = EXCLUDED.home_village_id,
            current_village_id = EXCLUDED.current_village_id,
            state = EXCLUDED.state,
            tribe = EXCLUDED.tribe,
            level = EXCLUDED.level,
            health = EXCLUDED.health,
            experience = EXCLUDED.experience,
            resource_focus = EXCLUDED.resource_focus,
            strength_points = EXCLUDED.strength_points,
            off_bonus_points = EXCLUDED.off_bonus_points,
            def_bonus_points = EXCLUDED.def_bonus_points,
            regeneration_points = EXCLUDED.regeneration_points,
            resources_points = EXCLUDED.resources_points,
            unassigned_points = EXCLUDED.unassigned_points,
            updated_at = NOW()
        "#,
    );
    query
}

pub(super) fn update_hero_stats_query(hero: &Hero) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        UPDATE rm_heroes
        SET tribe =
        "#,
    );
    query.push_bind(crate::persistence::models::Tribe::from(hero.tribe.clone()));
    query.push(", level = ");
    query.push_bind(hero.level as i16);
    query.push(", health = ");
    query.push_bind(hero.health as i16);
    query.push(", experience = ");
    query.push_bind(hero.experience as i32);
    query.push(", resource_focus = ");
    query.push_bind(Json(hero.resource_focus.clone()));
    query.push(", strength_points = ");
    query.push_bind(hero.strength_points as i16);
    query.push(", off_bonus_points = ");
    query.push_bind(hero.off_bonus_points as i16);
    query.push(", def_bonus_points = ");
    query.push_bind(hero.def_bonus_points as i16);
    query.push(", regeneration_points = ");
    query.push_bind(hero.regeneration_points as i16);
    query.push(", resources_points = ");
    query.push_bind(hero.resources_points as i16);
    query.push(", unassigned_points = ");
    query.push_bind(hero.unassigned_points as i16);
    query.push(", updated_at = NOW() WHERE hero_id = ");
    query.push_bind(hero.id);
    query
}

pub(super) fn hero_by_id_query(hero_id: Uuid) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(hero_select_sql());
    query.push(" WHERE hero_id = ");
    query.push_bind(hero_id);
    query
}

pub(super) fn hero_by_player_query(player_id: Uuid) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(hero_select_sql());
    query.push(" WHERE player_id = ");
    query.push_bind(player_id);
    query
}

pub(super) fn alive_hero_exists_for_player_query(
    player_id: Uuid,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM rm_heroes
            WHERE player_id =
        "#,
    );
    query.push_bind(player_id);
    query.push(" AND health > 0)");
    query
}

fn hero_select_sql() -> &'static str {
    r#"
    SELECT hero_id, player_id, home_village_id, tribe,
           level, health, experience, resource_focus, strength_points, off_bonus_points,
           def_bonus_points, regeneration_points, resources_points, unassigned_points
    FROM rm_heroes
    "#
}

fn hero_state_name(state: HeroPlacementState) -> &'static str {
    match state {
        HeroPlacementState::Home => "home",
        HeroPlacementState::Stationed => "stationed",
        HeroPlacementState::Moving => "moving",
    }
}
