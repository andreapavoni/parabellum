use parabellum_app::villages::CreateHero;
use parabellum_infra::es::VillageEsService;
use parabellum_infra::establish_connection_pool;
use parabellum_server::logs::setup_logging;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
struct MissingHeroPlayer {
    player_id: Uuid,
    village_id: i32,
}

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();

    let execute = std::env::args().skip(1).any(|arg| arg == "--execute");
    let pool = establish_connection_pool().await?;
    let missing = players_missing_heroes(&pool).await?;
    let repair_count = count_home_heroes_missing_army_link(&pool).await?;

    if missing.is_empty() && repair_count == 0 {
        println!("No players missing heroes and no hero army links to repair.");
        return Ok(());
    }

    println!(
        "{} player(s) missing heroes, {} home hero army link(s) to repair. Mode: {}",
        missing.len(),
        repair_count,
        if execute { "execute" } else { "dry-run" }
    );

    if !execute {
        for row in &missing {
            println!(
                "  would create hero for player={} village_id={}",
                row.player_id, row.village_id
            );
        }
        if repair_count > 0 {
            println!("  would repair {repair_count} home hero army link(s)");
        }
        println!("Run with --execute to append HeroCreated events and repair army links.");
        return Ok(());
    }

    let service = VillageEsService::new(pool.clone());
    let mut created = 0usize;
    for row in missing {
        let village_id = row.village_id as u32;
        service
            .create_hero(
                village_id,
                &CreateHero {
                    hero_id: Uuid::new_v4(),
                    player_id: row.player_id,
                    village_id,
                    has_existing_hero: false,
                    bypass_hero_mansion_requirement: true,
                },
            )
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        created += 1;
        println!(
            "  created hero for player={} village_id={}",
            row.player_id, village_id
        );
    }

    let repaired = repair_home_hero_army_links(&pool).await?;

    println!("Backfill complete: {created} hero(s) created, {repaired} army link(s) repaired.");
    Ok(())
}

async fn players_missing_heroes(
    pool: &sqlx::PgPool,
) -> Result<Vec<MissingHeroPlayer>, ApplicationError> {
    sqlx::query_as(
        r#"
        SELECT p.id AS player_id, chosen.village_id
        FROM players p
        JOIN LATERAL (
            SELECT v.village_id
            FROM rm_village v
            WHERE v.player_id = p.id
            ORDER BY v.is_capital DESC, v.village_id ASC
            LIMIT 1
        ) chosen ON TRUE
        WHERE NOT EXISTS (
            SELECT 1
            FROM rm_heroes h
            WHERE h.player_id = p.id
        )
        ORDER BY chosen.village_id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))
}

async fn count_home_heroes_missing_army_link(pool: &sqlx::PgPool) -> Result<i64, ApplicationError> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rm_heroes h
        WHERE h.state = 'home'
          AND h.current_village_id = h.home_village_id
          AND NOT EXISTS (
              SELECT 1
              FROM rm_armies a
              WHERE a.village_id = h.home_village_id
                AND a.current_village_id = h.home_village_id
                AND a.state = 'home'
                AND a.hero_id = h.hero_id
          )
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))
}

async fn repair_home_hero_army_links(pool: &sqlx::PgPool) -> Result<u64, ApplicationError> {
    let updated = sqlx::query(
        r#"
        UPDATE rm_armies a
        SET hero_id = h.hero_id,
            updated_at = NOW()
        FROM rm_heroes h
        WHERE h.state = 'home'
          AND h.current_village_id = h.home_village_id
          AND a.village_id = h.home_village_id
          AND a.current_village_id = h.home_village_id
          AND a.state = 'home'
          AND a.hero_id IS DISTINCT FROM h.hero_id
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    .rows_affected();

    let inserted = sqlx::query(
        r#"
        INSERT INTO rm_armies (
            army_id, village_id, current_village_id, current_map_field_id, player_id, tribe,
            state, units, smithy_upgrades, hero_id, updated_at
        )
        SELECT
            gen_random_uuid(), h.home_village_id, h.home_village_id, h.home_village_id,
            h.player_id, h.tribe, 'home',
            ARRAY[0,0,0,0,0,0,0,0,0,0]::INTEGER[],
            ARRAY[0,0,0,0,0,0,0,0]::SMALLINT[],
            h.hero_id, NOW()
        FROM rm_heroes h
        WHERE h.state = 'home'
          AND h.current_village_id = h.home_village_id
          AND NOT EXISTS (
              SELECT 1
              FROM rm_armies a
              WHERE a.village_id = h.home_village_id
                AND a.state = 'home'
          )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?
    .rows_affected();

    Ok(updated + inserted)
}
