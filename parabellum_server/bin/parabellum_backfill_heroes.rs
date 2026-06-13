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

    if missing.is_empty() {
        println!("No players missing heroes.");
        return Ok(());
    }

    println!(
        "{} player(s) missing heroes. Mode: {}",
        missing.len(),
        if execute { "execute" } else { "dry-run" }
    );

    if !execute {
        for row in &missing {
            println!(
                "  would create hero for player={} village_id={}",
                row.player_id, row.village_id
            );
        }
        println!("Run with --execute to append HeroCreated events.");
        return Ok(());
    }

    let service = VillageEsService::new(pool);
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

    println!("Backfill complete: {created} hero(s) created.");
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
