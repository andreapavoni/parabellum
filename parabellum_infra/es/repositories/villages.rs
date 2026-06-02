use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::repositories::VillageRepository;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::village::{VillageBuilding, VillageProduction, VillageStocks};
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_types::{
    buildings::BuildingName, common::ResourceGroup, map::Position, tribe::Tribe,
};
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresVillageRepository {
    pool: PgPool,
}

impl PostgresVillageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_village_state(
        &self,
        model: &VillageModel,
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.replace_village_state_in_tx(&mut tx, model).await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn replace_village_state_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        model: &VillageModel,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET player_id = $2,
                village_name = $3,
                position = $4,
                tribe = $5,
                buildings = $6,
                production = $7,
                stocks = $8,
                population = $9,
                loyalty = $10,
                is_capital = $11,
                culture_points_production = $12,
                smithy_upgrades = $13,
                academy_research = $14,
                parent_village_id = $15,
                army = $16,
                reinforcements = $17,
                deployed_armies = $18,
                total_merchants = $19,
                busy_merchants = $20,
                loyalty_updated_at = CASE
                    WHEN loyalty <> $10 THEN NOW()
                    ELSE loyalty_updated_at
                END,
                updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(model.village_id as i32)
        .bind(model.player_id)
        .bind(&model.village_name)
        .bind(Json(&model.position))
        .bind(DbTribe::from(model.tribe.clone()))
        .bind(Json(&model.buildings))
        .bind(Json(&model.production))
        .bind(Json(&model.stocks))
        .bind(model.population as i32)
        .bind(model.loyalty as i16)
        .bind(model.is_capital)
        .bind(model.culture_points_production as i32)
        .bind(Json(&model.smithy_upgrades))
        .bind(Json(&model.academy_research))
        .bind(model.parent_village_id.map(|id| id as i32))
        .bind(Json(&model.army))
        .bind(Json(&model.reinforcements))
        .bind(Json(&model.deployed_armies))
        .bind(model.total_merchants as i16)
        .bind(model.busy_merchants as i16)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn set_stored_resources_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        resources: ResourceGroup,
    ) -> Result<(), ApplicationError> {
        let stocks: Json<VillageStocks> =
            sqlx::query_scalar("SELECT stocks FROM rm_village WHERE village_id = $1")
                .bind(village_id as i32)
                .fetch_one(&mut **tx)
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
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn set_busy_merchants_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        busy_merchants: u8,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET busy_merchants = $2
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(busy_merchants as i16)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn update_army_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        army: &Option<Army>,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET army = $2
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(army))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn update_reinforcements_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        reinforcements: &[Army],
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET reinforcements = $2
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(reinforcements))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn update_deployed_armies_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        deployed_armies: &[Army],
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET deployed_armies = $2
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(deployed_armies))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn set_map_occupancy_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        field_id: u32,
        village_id: Option<u32>,
        player_id: Option<Uuid>,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_map_fields
            SET village_id = $2,
                player_id = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(field_id as i32)
        .bind(village_id.map(|id| id as i32))
        .bind(player_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    fn refresh_materialized_village_state(model: VillageModel) -> VillageModel {
        let hydrated: parabellum_game::models::village::Village = model.clone().into();
        let mut refreshed = model;
        refreshed.production = hydrated.production.clone();
        refreshed.stocks = hydrated.stocks().clone();
        refreshed.population = hydrated.population;
        refreshed.culture_points_production = hydrated.culture_points_production;
        refreshed.total_merchants = hydrated.total_merchants;
        let residence_or_palace_level = refreshed
            .buildings
            .iter()
            .filter(|b| {
                matches!(
                    b.building.name,
                    BuildingName::Residence | BuildingName::Palace
                )
            })
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);
        if refreshed.loyalty < 100 && residence_or_palace_level > 0 {
            let elapsed_secs = (chrono::Utc::now() - refreshed.loyalty_updated_at).num_seconds();
            if elapsed_secs > 0 {
                let speed = (parabellum_app::config::Config::from_env().speed as f64).max(1.0);
                let rate_per_sec = (2.0 * residence_or_palace_level as f64 * speed) / (3.0 * 3600.0);
                let gained = (elapsed_secs as f64 * rate_per_sec).floor() as u8;
                refreshed.loyalty = refreshed.loyalty.saturating_add(gained).min(100);
            }
        }
        // Busy merchants are operational state managed by movement/marketplace flows,
        // and `Village::from_persistence` resets it to zero internally.
        // Preserve the persisted value from the read model.
        refreshed.updated_at = hydrated.updated_at;
        refreshed
    }

    async fn refresh_for_read(
        &self,
        model: VillageModel,
    ) -> Result<VillageModel, ApplicationError> {
        Ok(Self::refresh_materialized_village_state(model))
    }

    pub async fn update_building_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    ) -> Result<(), ApplicationError> {
        let row: DbVillageModelRow = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let model: VillageModel = row.try_into()?;

        let mut next_buildings = model.buildings.clone();
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
        let total_merchants = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Marketplace)
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);

        let current_stocks = {
            let current = parabellum_game::models::village::Village::from_persistence(
                model.village_id,
                model.village_name.clone(),
                model.player_id,
                model.position.clone(),
                model.tribe.clone(),
                model.buildings.clone(),
                vec![],
                model.population,
                model.army.clone(),
                model.reinforcements.clone(),
                model.deployed_armies.clone(),
                model.loyalty,
                model.production.clone(),
                model.is_capital,
                model.smithy_upgrades,
                model.stocks.clone(),
                model.academy_research.clone(),
                0,
                model.culture_points_production,
                model.updated_at,
                model.parent_village_id,
            );
            current.stocks().clone()
        };

        let mut next_stocks = current_stocks;
        next_stocks.warehouse_capacity = warehouse_capacity;
        next_stocks.granary_capacity = granary_capacity;
        next_stocks.lumber = next_stocks.lumber.min(warehouse_capacity);
        next_stocks.clay = next_stocks.clay.min(warehouse_capacity);
        next_stocks.iron = next_stocks.iron.min(warehouse_capacity);
        next_stocks.crop = next_stocks.crop.min(granary_capacity as i64);

        let hydrated = parabellum_game::models::village::Village::from_persistence(
            model.village_id,
            model.village_name.clone(),
            model.player_id,
            model.position,
            model.tribe,
            next_buildings.clone(),
            vec![],
            model.population,
            model.army.clone(),
            model.reinforcements.clone(),
            model.deployed_armies.clone(),
            model.loyalty,
            model.production.clone(),
            model.is_capital,
            model.smithy_upgrades,
            next_stocks.clone(),
            model.academy_research.clone(),
            0,
            model.culture_points_production,
            chrono::Utc::now(),
            model.parent_village_id,
        );

        sqlx::query(
            r#"
            UPDATE rm_village
            SET buildings = $2,
                stocks = $3,
                total_merchants = $4,
                production = $5,
                population = $6,
                culture_points_production = $7,
                updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(next_buildings))
        .bind(Json(next_stocks))
        .bind(total_merchants as i16)
        .bind(Json(hydrated.production.clone()))
        .bind(hydrated.population as i32)
        .bind(hydrated.culture_points_production as i32)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn refresh_for_read_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        model: VillageModel,
    ) -> Result<VillageModel, ApplicationError> {
        let _ = tx;
        Ok(Self::refresh_materialized_village_state(model))
    }

    pub async fn get_by_village_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let row: DbVillageModelRow = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let model: VillageModel = row.try_into()?;
        self.refresh_for_read_in_tx(tx, model).await
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
    culture_points_production: i32,
    smithy_upgrades: Json<SmithyUpgrades>,
    academy_research: Json<parabellum_game::models::village::AcademyResearch>,
    parent_village_id: Option<i32>,
    army: Json<Option<Army>>,
    reinforcements: Json<Vec<Army>>,
    deployed_armies: Json<Vec<Army>>,
    total_merchants: i16,
    busy_merchants: i16,
    loyalty_updated_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
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
            culture_points_production: value.culture_points_production as u32,
            smithy_upgrades: value.smithy_upgrades.0,
            academy_research: value.academy_research.0,
            total_merchants: value.total_merchants as u8,
            busy_merchants: value.busy_merchants as u8,
            loyalty_updated_at: value.loyalty_updated_at,
            updated_at: value.updated_at,
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
impl VillageRepository for PostgresVillageRepository {
    async fn list_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        let rows: Vec<DbVillageModelRow> = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE player_id = $1
            ORDER BY village_id ASC
            "#,
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut villages = Vec::with_capacity(rows.len());
        for row in rows {
            let model: VillageModel = row.try_into()?;
            villages.push(self.refresh_for_read(model).await?);
        }
        Ok(villages)
    }

    async fn list_by_village_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        if village_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<i32> = village_ids.iter().map(|id| *id as i32).collect();
        let rows: Vec<DbVillageModelRow> = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE village_id = ANY($1)
            ORDER BY village_id ASC
            "#,
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut villages = Vec::with_capacity(rows.len());
        for row in rows {
            let model: VillageModel = row.try_into()?;
            villages.push(self.refresh_for_read(model).await?);
        }
        Ok(villages)
    }

    async fn upsert_from_village(
        &self,
        village_id: u32,
        player_id: Uuid,
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        parent_village_id: Option<u32>,
        buildings: &[VillageBuilding],
        army: &Option<Army>,
    ) -> Result<(), ApplicationError> {
        let warehouse_capacity = buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Warehouse)
            .map(|b| b.building.value)
            .max()
            .unwrap_or(800);
        let granary_capacity = buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Granary)
            .map(|b| b.building.value)
            .max()
            .unwrap_or(800);
        let total_merchants = buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Marketplace)
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);
        let stocks = VillageStocks {
            warehouse_capacity,
            granary_capacity,
            lumber: 800.min(warehouse_capacity),
            clay: 800.min(warehouse_capacity),
            iron: 800.min(warehouse_capacity),
            crop: (800_i64).min(granary_capacity as i64),
        };
        let projected = parabellum_game::models::village::Village::from_persistence(
            village_id,
            village_name.to_string(),
            player_id,
            position.clone(),
            tribe.clone(),
            buildings.to_vec(),
            vec![],
            2,
            army.clone(),
            vec![],
            vec![],
            100,
            VillageProduction::default(),
            parent_village_id.is_none(),
            [0_u8; 8],
            stocks.clone(),
            parabellum_game::models::village::AcademyResearch::default(),
            0,
            0,
            chrono::Utc::now(),
            parent_village_id,
        );

        sqlx::query(
            r#"
            INSERT INTO rm_village (
                village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14, $15,
                $16, $17, $18, $19, $20, NOW()
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
                army = EXCLUDED.army,
                reinforcements = EXCLUDED.reinforcements,
                deployed_armies = EXCLUDED.deployed_armies,
                total_merchants = EXCLUDED.total_merchants,
                busy_merchants = EXCLUDED.busy_merchants,
                loyalty_updated_at = EXCLUDED.loyalty_updated_at,
                updated_at = NOW()
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .bind(village_name)
        .bind(Json(position))
        .bind(DbTribe::from(tribe))
        .bind(Json(buildings))
        .bind(Json(projected.production.clone()))
        .bind(Json(stocks))
        .bind(projected.population as i32)
        .bind(100_i16)
        .bind(parent_village_id.is_none())
        .bind(projected.culture_points_production as i32)
        .bind(Json([0_u8; 8]))
        .bind(Json(parabellum_game::models::village::AcademyResearch::default()))
        .bind(parent_village_id.map(|id| id as i32))
        .bind(Json(army))
        .bind(Json(Vec::<Army>::new()))
        .bind(Json(Vec::<Army>::new()))
        .bind(total_merchants as i16)
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
        army: &Option<Army>,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET army = $2
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
        reinforcements: &[Army],
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET reinforcements = $2
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
        deployed_armies: &[Army],
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET deployed_armies = $2
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
        let row: DbVillageModelRow = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let model: VillageModel = row.try_into()?;

        let mut next_buildings = model.buildings.clone();
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
        let total_merchants = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Marketplace)
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);

        let current_stocks = {
            let current = parabellum_game::models::village::Village::from_persistence(
                model.village_id,
                model.village_name.clone(),
                model.player_id,
                model.position.clone(),
                model.tribe.clone(),
                model.buildings.clone(),
                vec![],
                model.population,
                model.army.clone(),
                model.reinforcements.clone(),
                model.deployed_armies.clone(),
                model.loyalty,
                model.production.clone(),
                model.is_capital,
                model.smithy_upgrades,
                model.stocks.clone(),
                model.academy_research.clone(),
                0,
                model.culture_points_production,
                model.updated_at,
                model.parent_village_id,
            );
            current.stocks().clone()
        };

        let mut next_stocks = current_stocks;
        next_stocks.warehouse_capacity = warehouse_capacity;
        next_stocks.granary_capacity = granary_capacity;
        next_stocks.lumber = next_stocks.lumber.min(warehouse_capacity);
        next_stocks.clay = next_stocks.clay.min(warehouse_capacity);
        next_stocks.iron = next_stocks.iron.min(warehouse_capacity);
        next_stocks.crop = next_stocks.crop.min(granary_capacity as i64);

        let hydrated = parabellum_game::models::village::Village::from_persistence(
            model.village_id,
            model.village_name.clone(),
            model.player_id,
            model.position,
            model.tribe,
            next_buildings.clone(),
            vec![],
            model.population,
            model.army.clone(),
            model.reinforcements.clone(),
            model.deployed_armies.clone(),
            model.loyalty,
            model.production.clone(),
            model.is_capital,
            model.smithy_upgrades,
            next_stocks.clone(),
            model.academy_research.clone(),
            0,
            model.culture_points_production,
            chrono::Utc::now(),
            model.parent_village_id,
        );

        sqlx::query(
            r#"
            UPDATE rm_village
            SET buildings = $2,
                stocks = $3,
                total_merchants = $4,
                production = $5,
                population = $6,
                culture_points_production = $7,
                updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(next_buildings))
        .bind(Json(next_stocks))
        .bind(total_merchants as i16)
        .bind(Json(hydrated.production.clone()))
        .bind(hydrated.population as i32)
        .bind(hydrated.culture_points_production as i32)
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

    async fn set_busy_merchants(
        &self,
        village_id: u32,
        busy_merchants: u8,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_village
            SET busy_merchants = $2
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(busy_merchants as i16)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_village_id(&self, village_id: u32) -> Result<VillageModel, ApplicationError> {
        let row: Option<DbVillageModelRow> = sqlx::query_as(
            r#"
            SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                   population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   army, reinforcements, deployed_armies, total_merchants, busy_merchants, loyalty_updated_at, updated_at
            FROM rm_village
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let row = row.ok_or(DbError::VillageNotFound(village_id))?;

        let model: VillageModel = row.try_into()?;
        self.refresh_for_read(model).await
    }

    async fn set_map_occupancy(
        &self,
        field_id: u32,
        village_id: Option<u32>,
        player_id: Option<Uuid>,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_map_fields
            SET village_id = $2,
                player_id = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(field_id as i32)
        .bind(village_id.map(|id| id as i32))
        .bind(player_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
