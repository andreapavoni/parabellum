use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::repositories::VillageModelRepository;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::village::{VillageBuilding, VillageProduction, VillageStocks};
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_types::{
    buildings::BuildingName, common::ResourceGroup, map::Position, tribe::Tribe,
};
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
    army: Json<parabellum_types::army::TroopSet>,
    reinforcements: Json<parabellum_types::army::TroopSet>,
    deployed_armies: Json<parabellum_types::army::TroopSet>,
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
            army: value.army.0,
            reinforcements: value.reinforcements.0,
            deployed_armies: value.deployed_armies.0,
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

impl From<Tribe> for DbTribe {
    fn from(value: Tribe) -> Self {
        match value {
            Tribe::Roman => Self::Roman,
            Tribe::Gaul => Self::Gaul,
            Tribe::Teuton => Self::Teuton,
            Tribe::Natar => Self::Natar,
            Tribe::Nature => Self::Nature,
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
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants
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
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        buildings: &[VillageBuilding],
        army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO rm_village (
                village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                population, loyalty, is_capital, culture_points, culture_points_production, parent_village_id,
                army, reinforcements, deployed_armies, total_merchants, busy_merchants
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19
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
                culture_points = EXCLUDED.culture_points,
                culture_points_production = EXCLUDED.culture_points_production,
                parent_village_id = EXCLUDED.parent_village_id,
                army = EXCLUDED.army,
                reinforcements = EXCLUDED.reinforcements,
                deployed_armies = EXCLUDED.deployed_armies,
                total_merchants = EXCLUDED.total_merchants,
                busy_merchants = EXCLUDED.busy_merchants,
                updated_at = NOW()
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .bind(village_name)
        .bind(Json(position))
        .bind(DbTribe::from(tribe))
        .bind(Json(buildings))
        .bind(Json(VillageProduction::default()))
        .bind(Json(VillageStocks::default()))
        .bind(2_i32)
        .bind(100_i16)
        .bind(false)
        .bind(0_i32)
        .bind(0_i32)
        .bind(None::<i32>)
        .bind(Json(army))
        .bind(Json(parabellum_types::army::TroopSet::default()))
        .bind(Json(parabellum_types::army::TroopSet::default()))
        .bind(0_i16)
        .bind(0_i16)
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

    async fn update_army(
        &self,
        village_id: u32,
        army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET army = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(army))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update_reinforcements(
        &self,
        village_id: u32,
        reinforcements: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET reinforcements = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(reinforcements))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update_deployed_armies(
        &self,
        village_id: u32,
        deployed_armies: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET deployed_armies = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(deployed_armies))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update_building(
        &self,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    ) -> Result<(), ApplicationError> {
        let (buildings, stocks): (Json<Vec<VillageBuilding>>, Json<VillageStocks>) =
            sqlx::query_as("SELECT buildings, stocks FROM rm_village WHERE village_id = $1")
                .bind(village_id as i32)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut next_buildings = buildings.0;
        if let Some(entry) = next_buildings.iter_mut().find(|b| b.slot_id == slot_id) {
            let next_building = Building::new(building_name.clone(), speed)
                .at_level(level, speed)
                .map_err(ApplicationError::Game)?;
            entry.building = next_building;
        } else {
            let next_building = Building::new(building_name.clone(), speed)
                .at_level(level, speed)
                .map_err(ApplicationError::Game)?;
            next_buildings.push(VillageBuilding {
                slot_id,
                building: next_building,
            });
        }

        let warehouse_capacity = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Warehouse)
            .map(|b| b.building.value)
            .max()
            .unwrap_or(800);
        let granary_capacity = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Granary)
            .map(|b| b.building.value)
            .max()
            .unwrap_or(800);

        let mut next_stocks = stocks.0;
        next_stocks.warehouse_capacity = warehouse_capacity;
        next_stocks.granary_capacity = granary_capacity;
        next_stocks.lumber = next_stocks.lumber.min(warehouse_capacity);
        next_stocks.clay = next_stocks.clay.min(warehouse_capacity);
        next_stocks.iron = next_stocks.iron.min(warehouse_capacity);
        next_stocks.crop = next_stocks.crop.min(granary_capacity as i64);

        sqlx::query(
            r#"
            UPDATE rm_village
            SET buildings = $2, stocks = $3, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(next_buildings))
        .bind(Json(next_stocks))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn set_stored_resources(
        &self,
        village_id: u32,
        resources: ResourceGroup,
    ) -> Result<(), ApplicationError> {
        let stocks: Json<VillageStocks> =
            sqlx::query_scalar("SELECT stocks FROM rm_village WHERE village_id = $1")
                .bind(village_id as i32)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut next = stocks.0;
        next.lumber = resources.lumber().min(next.warehouse_capacity);
        next.clay = resources.clay().min(next.warehouse_capacity);
        next.iron = resources.iron().min(next.warehouse_capacity);
        next.crop = (resources.crop() as i64).min(next.granary_capacity as i64);

        sqlx::query(
            r#"
            UPDATE rm_village
            SET stocks = $2, updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(next))
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
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants
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
