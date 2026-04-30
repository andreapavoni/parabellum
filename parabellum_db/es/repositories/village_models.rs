use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::repositories::VillageModelRepository;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresVillageModelRepository {
    pool: PgPool,
}

impl PostgresVillageModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct DbVillageModelRow {
    village_id: i32,
    player_id: Uuid,
    village_name: String,
    position: Json<parabellum_types::map::Position>,
    tribe: DbTribe,
    buildings: Json<Vec<parabellum_game::models::village::VillageBuilding>>,
    production: Json<parabellum_game::models::village::VillageProduction>,
    stocks: Json<parabellum_game::models::village::VillageStocks>,
    population: i32,
    loyalty: i16,
    is_capital: bool,
    culture_points: i32,
    culture_points_production: i32,
    parent_village_id: Option<i32>,
    stationed_army: Json<parabellum_types::army::TroopSet>,
    total_merchants: i16,
    busy_merchants: i16,
}

impl TryFrom<DbVillageModelRow> for VillageModel {
    type Error = ApplicationError;

    fn try_from(value: DbVillageModelRow) -> Result<Self, Self::Error> {
        Ok(VillageModel {
            village_id: value.village_id as u32,
            player_id: value.player_id,
            village_name: value.village_name,
            position: value.position.0,
            tribe: value.tribe.into(),
            buildings: value.buildings.0,
            production: value.production.0,
            stocks: value.stocks.0,
            population: value.population as u32,
            loyalty: value.loyalty as u8,
            is_capital: value.is_capital,
            culture_points: value.culture_points as u32,
            culture_points_production: value.culture_points_production as u32,
            total_merchants: value.total_merchants as u8,
            busy_merchants: value.busy_merchants as u8,
            parent_village_id: value.parent_village_id.map(|v| v as u32),
            stationed_army: value.stationed_army.0,
        })
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "tribe", rename_all = "PascalCase")]
enum DbTribe {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

impl From<DbTribe> for parabellum_types::tribe::Tribe {
    fn from(value: DbTribe) -> Self {
        match value {
            DbTribe::Roman => Self::Roman,
            DbTribe::Gaul => Self::Gaul,
            DbTribe::Teuton => Self::Teuton,
            DbTribe::Natar => Self::Natar,
            DbTribe::Nature => Self::Nature,
        }
    }
}

#[async_trait::async_trait]
impl VillageModelRepository for PostgresVillageModelRepository {
    async fn list_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        let rows: Vec<DbVillageModelRow> = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points, culture_points_production, parent_village_id,
                   stationed_army, total_merchants, busy_merchants
            FROM rm_village
            WHERE player_id = $1
            ORDER BY village_id ASC
            "#,
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn upsert_from_village(
        &self,
        village_id: u32,
        player_id: Uuid,
        stationed_army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO rm_village (
                village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                population, loyalty, is_capital, culture_points, culture_points_production, parent_village_id,
                stationed_army, total_merchants, busy_merchants
            )
            SELECT
                v.id,
                $2,
                v.name,
                v.position,
                p.tribe,
                v.buildings,
                v.production,
                v.stocks,
                v.population,
                v.loyalty,
                v.is_capital,
                v.culture_points,
                v.culture_points_production,
                v.parent_village_id,
                $3,
                0,
                0
            FROM villages v
            JOIN players p ON p.id = v.player_id
            WHERE v.id = $1
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
                culture_points = EXCLUDED.culture_points,
                culture_points_production = EXCLUDED.culture_points_production,
                parent_village_id = EXCLUDED.parent_village_id,
                stationed_army = EXCLUDED.stationed_army,
                total_merchants = EXCLUDED.total_merchants,
                busy_merchants = EXCLUDED.busy_merchants,
                updated_at = NOW()
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .bind(Json(stationed_army))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update_player_id(
        &self,
        village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET player_id = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update_stationed_army(
        &self,
        village_id: u32,
        stationed_army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET stationed_army = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(stationed_army))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn refresh_from_source(&self, village_id: u32) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village rm
            SET
                player_id = v.player_id,
                village_name = v.name,
                position = v.position,
                tribe = p.tribe,
                buildings = v.buildings,
                production = v.production,
                stocks = v.stocks,
                population = v.population,
                loyalty = v.loyalty,
                is_capital = v.is_capital,
                culture_points = v.culture_points,
                culture_points_production = v.culture_points_production,
                parent_village_id = v.parent_village_id,
                updated_at = NOW()
            FROM villages v
            JOIN players p ON p.id = v.player_id
            WHERE rm.village_id = v.id AND rm.village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_village_id(&self, village_id: u32) -> Result<VillageModel, ApplicationError> {
        let row: DbVillageModelRow = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points, culture_points_production, parent_village_id,
                   stationed_army, total_merchants, busy_merchants
            FROM rm_village
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        row.try_into()
    }
}
