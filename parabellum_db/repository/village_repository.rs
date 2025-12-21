use sqlx::{Postgres, Transaction, types::Json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::VillageRepository;
use parabellum_game::models::village::Village;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::{mapping::VillageAggregate, models as db_models};

/// Implements VillageRepository and operates on transactions.
#[derive(Clone)]
pub struct PostgresVillageRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresVillageRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> VillageRepository for PostgresVillageRepository<'a> {
    async fn get_by_id(&self, village_id_u32: u32) -> Result<Village, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let village_id_i32 = village_id_u32 as i32;
        let db_village = sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE id = $1",
            village_id_i32
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id_u32)))?;

        let db_player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE  2=2 AND id = $1"#,
            db_village.player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let all_armies = sqlx::query_as!(
            db_models::Army,
            r#"
            SELECT
                a.id,
                a.village_id,
                a.player_id,
                a.current_map_field_id,
                a.tribe as "tribe: _",
                a.units,
                a.smithy,
                a.hero_id as "hero_id?: Uuid",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.level             END as "hero_level?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.health            END as "hero_health?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.experience        END as "hero_experience?: i32",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.resource_focus    END as "hero_resource_focus?: _",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.strength_points          END as "hero_strength_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.resources_points         END as "hero_resources_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.regeneration_points      END as "hero_regeneration_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.off_bonus_points         END as "hero_off_bonus_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.def_bonus_points         END as "hero_def_bonus_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.unassigned_points END as "hero_unassigned_points?: i16"
            FROM armies a
            LEFT JOIN heroes h ON a.hero_id = h.id
            WHERE a.village_id = $1 OR a.current_map_field_id = $1
            "#,
            village_id_i32
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let db_oases = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE village_id = $1",
            village_id_i32
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        // Count busy merchants from both jobs (deliveries) and marketplace offers (reservations)
        let busy_merchants_from_jobs = sqlx::query!(
                    r#"
                    SELECT COALESCE(SUM((task->'data'->>'merchants_used')::smallint), 0) as total_busy
                    FROM jobs
                    WHERE village_id = $1
                      AND status IN ('Pending', 'Processing')
                      AND task->>'task_type' IN ('MerchantGoing', 'MerchantReturn')
                    "#,
                    village_id_i32
                )
                .fetch_one(&mut *tx_guard.as_mut())
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let busy_merchants_from_offers = sqlx::query!(
            r#"
                    SELECT COALESCE(SUM(merchants_required), 0) as total_reserved
                    FROM marketplace_offers
                    WHERE village_id = $1
                    "#,
            village_id_i32
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let busy_merchants = (busy_merchants_from_jobs.total_busy.unwrap_or(0)
            + busy_merchants_from_offers.total_reserved.unwrap_or(0))
            as u8;

        let aggregate = VillageAggregate {
            village: db_village,
            player: db_player,
            armies: all_armies,
            oases: db_oases,
        };

        let mut game_village = Village::try_from(aggregate)?;
        game_village.busy_merchants = busy_merchants;
        Ok(game_village)
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let db_player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE 3=3 AND id = $1"#,
            player_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;

        let db_villages = sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE player_id = $1",
            player_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        if db_villages.is_empty() {
            return Ok(Vec::new());
        }

        let village_ids: Vec<i32> = db_villages.iter().map(|v| v.id).collect();

        let all_armies = sqlx::query_as!(
            db_models::Army,
            r#"
            SELECT
                a.id,
                a.village_id,
                a.player_id,
                a.current_map_field_id,
                a.tribe as "tribe: _",
                a.units,
                a.smithy,
                a.hero_id as "hero_id?: Uuid",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.level             END as "hero_level?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.health            END as "hero_health?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.experience        END as "hero_experience?: i32",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.resource_focus    END as "hero_resource_focus?: _",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.strength_points          END as "hero_strength_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.resources_points         END as "hero_resources_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.regeneration_points      END as "hero_regeneration_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.off_bonus_points         END as "hero_off_bonus_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.def_bonus_points         END as "hero_def_bonus_points?: i16",
                CASE WHEN h.id IS NULL THEN NULL ELSE h.unassigned_points END as "hero_unassigned_points?: i16"
            FROM armies a
            LEFT JOIN heroes h ON a.hero_id = h.id
            WHERE a.village_id = ANY($1) OR a.current_map_field_id = ANY($1)
            "#,
            &village_ids
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut armies_map: HashMap<i32, Vec<db_models::Army>> = HashMap::new();
        for army in all_armies {
            armies_map.entry(army.village_id).or_default().push(army);
        }

        let all_oases = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE village_id = ANY($1)",
            &village_ids
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let merchant_counts = sqlx::query_as!(
                BusyMerchants,
                r#"
                SELECT village_id, COALESCE(SUM((task->'data'->>'merchants_used')::smallint), 0) as total_busy
                FROM jobs
                WHERE village_id = ANY($1)
                  AND status IN ('Pending', 'Processing')
                  AND task->>'task_type' IN ('MerchantGoing', 'MerchantReturn')
                GROUP BY village_id
                "#,
                &village_ids
            )
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let offer_counts = sqlx::query_as!(
            BusyMerchants,
            r#"
            SELECT village_id, COALESCE(SUM(merchants_required), 0) as total_busy
            FROM marketplace_offers
            WHERE village_id = ANY($1)
            GROUP BY village_id
            "#,
            &village_ids
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        drop(tx_guard);

        let oases_map: HashMap<i32, Vec<db_models::MapField>> =
            all_oases
                .into_iter()
                .fold(HashMap::new(), |mut acc, oasis| {
                    if let Some(vid) = oasis.village_id {
                        acc.entry(vid).or_default().push(oasis);
                    }
                    acc
                });

        let mut merchants_map: HashMap<i32, u8> = merchant_counts
            .into_iter()
            .map(|rec| (rec.village_id, rec.total_busy.unwrap_or(0) as u8))
            .collect();
        for rec in offer_counts {
            let reserved = rec.total_busy.unwrap_or(0) as u8;
            merchants_map
                .entry(rec.village_id)
                .and_modify(|value| *value = value.saturating_add(reserved))
                .or_insert(reserved);
        }

        let mut game_villages: Vec<Village> = Vec::with_capacity(db_villages.len());

        for village in db_villages {
            let village_id_i32 = village.id;

            let related_armies = armies_map.get(&village_id_i32).cloned().unwrap_or_default();
            let related_oases = oases_map.get(&village_id_i32).cloned().unwrap_or_default();

            let aggregate = VillageAggregate {
                village,
                player: db_player.clone(),
                armies: related_armies,
                oases: related_oases.clone().clone(),
            };

            let mut game_village = Village::try_from(aggregate)?;
            let busy_merchants = merchants_map.get(&village_id_i32).cloned().unwrap_or(0);

            game_village.busy_merchants = busy_merchants;

            game_villages.push(game_village);
        }

        Ok(game_villages)
    }

    async fn save(&self, village: &Village) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
                INSERT INTO villages (
                    id, player_id, name, position, buildings, production,
                    stocks, smithy_upgrades, academy_research, population,
                    loyalty, is_capital, culture_points, culture_points_production, parent_village_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (id) DO UPDATE
                SET
                    name = $3,
                    buildings = $5,
                    production = $6,
                    stocks = $7,
                    smithy_upgrades = $8,
                    academy_research = $9,
                    population = $10,
                    loyalty = $11,
                    culture_points = $13,
                    culture_points_production = $14,
                    parent_village_id = $15,
                    updated_at = NOW()
                "#,
            village.id as i32,
            village.player_id,
            village.name,
            Json(&village.position) as _,
            Json(&village.buildings()) as _,
            Json(&village.production) as _,
            Json(&village.stocks()) as _,
            Json(&village.smithy()) as _,
            Json(&village.academy_research()) as _,
            village.population as i32,
            village.loyalty() as i16,
            village.is_capital,
            village.culture_points as i32,
            village.culture_points_production as i32,
            village.parent_village_id.map(|id| id as i32)
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        sqlx::query!(
            r#"
            UPDATE map_fields
            SET village_id = $1, player_id = $2
            WHERE (position->>'x')::int = $3
              AND (position->>'y')::int = $4
            "#,
            village.id as i32,
            village.player_id,
            village.position.x,
            village.position.y,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_info_by_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<
        std::collections::HashMap<u32, parabellum_app::repository::VillageInfo>,
        ApplicationError,
    > {
        use parabellum_app::repository::VillageInfo;
        use std::collections::HashMap;

        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut tx_guard = self.tx.lock().await;
        let village_ids_i32: Vec<i32> = village_ids.iter().map(|&id| id as i32).collect();

        let rows = sqlx::query!(
            r#"
            SELECT id, name, position as "position: Json<parabellum_types::map::Position>"
            FROM villages
            WHERE id = ANY($1)
            "#,
            &village_ids_i32
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(parabellum_types::errors::DbError::Database(e)))?;

        let mut result = HashMap::new();
        for row in rows {
            result.insert(
                row.id as u32,
                VillageInfo {
                    id: row.id as u32,
                    name: row.name,
                    position: row.position.0,
                },
            );
        }

        Ok(result)
    }
}

// Helper struct
struct BusyMerchants {
    village_id: i32,
    total_busy: Option<i64>,
}
