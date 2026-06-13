use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::repositories::{
    ArmyRepository, ExpansionCultureSnapshot, ExpansionOwnershipSnapshot, VillageRepository,
};
use parabellum_app::villages::{VillageArmyContext, hydrate_village};
use parabellum_game::models::buildings::Building;
use parabellum_game::models::smithy::SmithyUpgrades;
use parabellum_game::models::trapper::TrapperState;
use parabellum_game::models::village::{
    AcademyResearch, VillageBuilding, VillageSnapshot, VillageStocks,
};
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_types::{
    buildings::BuildingName, common::ResourceGroup, map::Position, tribe::Tribe,
};
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

use super::PostgresArmyRepository;

#[derive(Debug, Clone)]
pub struct PostgresVillageRepository {
    pool: PgPool,
}

impl PostgresVillageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn inferred_server_speed(buildings: &[VillageBuilding]) -> i8 {
        buildings
            .iter()
            .filter_map(|building| building.building.inferred_server_speed())
            .max()
            .unwrap_or(1)
    }

    fn default_capacity_for_buildings(buildings: &[VillageBuilding]) -> u32 {
        800 * Self::inferred_server_speed(buildings).max(1) as u32
    }

    fn warehouse_capacity_for_buildings(buildings: &[VillageBuilding]) -> u32 {
        buildings
            .iter()
            .filter(|b| {
                matches!(
                    b.building.name,
                    BuildingName::Warehouse | BuildingName::GreatWarehouse
                )
            })
            .map(|b| b.building.value)
            .max()
            .unwrap_or_else(|| Self::default_capacity_for_buildings(buildings))
    }

    fn granary_capacity_for_buildings(buildings: &[VillageBuilding]) -> u32 {
        buildings
            .iter()
            .filter(|b| {
                matches!(
                    b.building.name,
                    BuildingName::Granary | BuildingName::GreatGranary
                )
            })
            .map(|b| b.building.value)
            .max()
            .unwrap_or_else(|| Self::default_capacity_for_buildings(buildings))
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
                total_merchants = $16,
                busy_merchants = $17,
                trapper_active_traps = $18,
                trapper_broken_traps = $19,
                trapper_queued_traps = $20,
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
        .bind(model.total_merchants as i16)
        .bind(model.busy_merchants as i16)
        .bind(model.trapper.active_traps as i32)
        .bind(model.trapper.broken_traps as i32)
        .bind(model.trapper.queued_traps as i32)
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

    pub async fn upsert_from_village_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
        player_id: Uuid,
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        parent_village_id: Option<u32>,
        buildings: &[VillageBuilding],
    ) -> Result<(), ApplicationError> {
        self.upsert_from_village_inner(
            Some(tx),
            village_id,
            player_id,
            village_name,
            position,
            tribe,
            parent_village_id,
            buildings,
        )
        .await
    }

    async fn upsert_from_village_inner(
        &self,
        tx: Option<&mut sqlx::Transaction<'_, sqlx::Postgres>>,
        village_id: u32,
        player_id: Uuid,
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        parent_village_id: Option<u32>,
        buildings: &[VillageBuilding],
    ) -> Result<(), ApplicationError> {
        let warehouse_capacity = Self::warehouse_capacity_for_buildings(buildings);
        let granary_capacity = Self::granary_capacity_for_buildings(buildings);
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
        let projected = parabellum_game::models::village::Village::rehydrate(VillageSnapshot {
            id: village_id,
            name: village_name.to_string(),
            player_id,
            position: position.clone(),
            tribe: tribe.clone(),
            buildings: buildings.to_vec(),
            oases: vec![],
            army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
            loyalty: 100,
            is_capital: parent_village_id.is_none(),
            smithy: [0_u8; 8],
            stocks: stocks.clone(),
            academy_research: AcademyResearch::default(),
            culture_points: 0,
            updated_at: chrono::Utc::now(),
            parent_village_id,
        });

        let q = sqlx::query(
            r#"
            INSERT INTO rm_village (
                village_id, player_id, village_name, position, tribe, buildings, production, stocks,
                population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
                   total_merchants, busy_merchants, trapper_active_traps, trapper_broken_traps, trapper_queued_traps, loyalty_updated_at
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
                total_merchants = EXCLUDED.total_merchants,
                busy_merchants = EXCLUDED.busy_merchants,
                trapper_active_traps = EXCLUDED.trapper_active_traps,
                trapper_broken_traps = EXCLUDED.trapper_broken_traps,
                trapper_queued_traps = EXCLUDED.trapper_queued_traps,
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
        .bind(total_merchants as i16)
        .bind(0_i16)
        .bind(0_i32)
        .bind(0_i32)
        .bind(0_i32);
        if let Some(tx) = tx {
            q.execute(&mut **tx)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        } else {
            q.execute(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        }
        Ok(())
    }

    fn refresh_materialized_village_state(
        model: VillageModel,
        army_context: VillageArmyContext,
        hero_resources: ResourceGroup,
    ) -> VillageModel {
        let moving_armies = army_context.moving.clone();
        let hydrated = hydrate_village(model.clone(), army_context);
        let busy_merchants = model.busy_merchants;
        let previous_updated_at = model.updated_at;
        let mut refreshed = model;
        refreshed.production = hydrated.production.clone();
        refreshed.production.upkeep = refreshed
            .production
            .upkeep
            .saturating_add(Self::moving_armies_upkeep(&hydrated, &moving_armies));
        refreshed.production.calculate_effective_production();
        refreshed.stocks = hydrated.stocks().clone();
        Self::apply_hero_resource_bonus(&mut refreshed, previous_updated_at, hero_resources);
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
                let rate_per_sec =
                    (2.0 * residence_or_palace_level as f64 * speed) / (3.0 * 3600.0);
                let gained = (elapsed_secs as f64 * rate_per_sec).floor() as u8;
                refreshed.loyalty = refreshed.loyalty.saturating_add(gained).min(100);
            }
        }
        // Busy merchants are operational state managed by movement/marketplace flows,
        // and `Village::from_persistence` resets it to zero internally.
        // Preserve the persisted value from the read model.
        refreshed.busy_merchants = busy_merchants.min(refreshed.total_merchants);
        refreshed.updated_at = hydrated.updated_at;
        refreshed
    }

    fn apply_hero_resource_bonus(
        refreshed: &mut VillageModel,
        previous_updated_at: chrono::DateTime<chrono::Utc>,
        hero_resources: ResourceGroup,
    ) {
        if hero_resources == ResourceGroup::default() {
            return;
        }

        refreshed.production.effective.lumber = refreshed
            .production
            .effective
            .lumber
            .saturating_add(hero_resources.lumber());
        refreshed.production.effective.clay = refreshed
            .production
            .effective
            .clay
            .saturating_add(hero_resources.clay());
        refreshed.production.effective.iron = refreshed
            .production
            .effective
            .iron
            .saturating_add(hero_resources.iron());
        refreshed.production.effective.crop = refreshed
            .production
            .effective
            .crop
            .saturating_add(hero_resources.crop() as i64);

        let elapsed = (chrono::Utc::now() - previous_updated_at).num_seconds() as f64;
        if elapsed <= 0.0 {
            return;
        }

        let add = |current: u32, per_hour: u32, capacity: u32| -> u32 {
            (current as f64 + elapsed * (per_hour as f64 / 3600.0))
                .min(capacity as f64)
                .max(0.0)
                .floor() as u32
        };
        refreshed.stocks.lumber = add(
            refreshed.stocks.lumber,
            hero_resources.lumber(),
            refreshed.stocks.warehouse_capacity,
        );
        refreshed.stocks.clay = add(
            refreshed.stocks.clay,
            hero_resources.clay(),
            refreshed.stocks.warehouse_capacity,
        );
        refreshed.stocks.iron = add(
            refreshed.stocks.iron,
            hero_resources.iron(),
            refreshed.stocks.warehouse_capacity,
        );
        refreshed.stocks.crop = add(
            refreshed.stocks.crop.max(0) as u32,
            hero_resources.crop(),
            refreshed.stocks.granary_capacity,
        ) as i64;
    }

    async fn hero_resource_bonus(
        &self,
        village_id: u32,
    ) -> Result<ResourceGroup, ApplicationError> {
        let row: Option<(i16, Json<parabellum_game::models::hero::HeroResourceFocus>)> =
            sqlx::query_as(
                r#"
                SELECT resources_points, resource_focus
                FROM rm_heroes
                WHERE home_village_id = $1 AND health > 0
                LIMIT 1
                "#,
            )
            .bind(village_id as i32)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let Some((points, focus)) = row else {
            return Ok(ResourceGroup::default());
        };
        let mut hero = parabellum_game::models::hero::Hero::new(
            None,
            village_id,
            Uuid::nil(),
            Tribe::Roman,
            None,
        );
        hero.resources_points = points.max(0) as u16;
        hero.resource_focus = focus.0;
        Ok(hero.resources())
    }

    async fn hero_resource_bonus_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<ResourceGroup, ApplicationError> {
        let row: Option<(i16, Json<parabellum_game::models::hero::HeroResourceFocus>)> =
            sqlx::query_as(
                r#"
                SELECT resources_points, resource_focus
                FROM rm_heroes
                WHERE home_village_id = $1 AND health > 0
                LIMIT 1
                "#,
            )
            .bind(village_id as i32)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let Some((points, focus)) = row else {
            return Ok(ResourceGroup::default());
        };
        let mut hero = parabellum_game::models::hero::Hero::new(
            None,
            village_id,
            Uuid::nil(),
            Tribe::Roman,
            None,
        );
        hero.resources_points = points.max(0) as u16;
        hero.resource_focus = focus.0;
        Ok(hero.resources())
    }

    fn moving_armies_upkeep(
        village: &parabellum_game::models::village::Village,
        armies: &[parabellum_game::models::army::Army],
    ) -> u32 {
        armies
            .iter()
            .map(|army| {
                army.tribe
                    .units()
                    .iter()
                    .enumerate()
                    .map(|(idx, unit)| {
                        village
                            .effective_unit_upkeep(unit)
                            .saturating_mul(army.units().get(idx))
                    })
                    .sum::<u32>()
            })
            .sum()
    }

    async fn load_army_context(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let armies = PostgresArmyRepository::new(self.pool.clone());
        armies.army_context_for_village(village_id).await
    }

    async fn load_army_context_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let armies = PostgresArmyRepository::new(self.pool.clone());
        armies.army_context_for_village_in_tx(tx, village_id).await
    }

    async fn refresh_for_read(
        &self,
        model: VillageModel,
    ) -> Result<VillageModel, ApplicationError> {
        let army = self.load_army_context(model.village_id).await?;
        let hero_resources = self.hero_resource_bonus(model.village_id).await?;
        Ok(Self::refresh_materialized_village_state(
            model,
            army,
            hero_resources,
        ))
    }

    async fn fetch_village_model(&self, village_id: u32) -> Result<VillageModel, ApplicationError> {
        let row: Option<DbVillageModelRow> =
            sqlx::query_as(&format!("{} WHERE village_id = $1", village_select_sql()))
                .bind(village_id as i32)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let row = row.ok_or(ApplicationError::Db(DbError::VillageNotFound(village_id)))?;
        row.try_into()
    }

    async fn fetch_village_model_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let row: DbVillageModelRow =
            sqlx::query_as(&format!("{} WHERE village_id = $1", village_select_sql()))
                .bind(village_id as i32)
                .fetch_one(&mut **tx)
                .await
                .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        row.try_into()
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
        let model = self.fetch_village_model_in_tx(tx, village_id).await?;

        let mut next_buildings = model.buildings.clone();
        if level == 0 && !(1..=18).contains(&slot_id) {
            next_buildings.retain(|building| building.slot_id != slot_id);
        } else if let Some(entry) = next_buildings.iter_mut().find(|b| b.slot_id == slot_id) {
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

        let warehouse_capacity = Self::warehouse_capacity_for_buildings(&next_buildings);
        let granary_capacity = Self::granary_capacity_for_buildings(&next_buildings);
        let total_merchants = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Marketplace)
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);

        let current_stocks = {
            let army_context = self.load_army_context_in_tx(tx, model.village_id).await?;
            let current = hydrate_village(model.clone(), army_context);
            current.stocks().clone()
        };

        let mut next_stocks = current_stocks;
        next_stocks.warehouse_capacity = warehouse_capacity;
        next_stocks.granary_capacity = granary_capacity;
        next_stocks.lumber = next_stocks.lumber.min(warehouse_capacity);
        next_stocks.clay = next_stocks.clay.min(warehouse_capacity);
        next_stocks.iron = next_stocks.iron.min(warehouse_capacity);
        next_stocks.crop = next_stocks.crop.min(granary_capacity as i64);

        let mut next_model = model;
        next_model.buildings = next_buildings.clone();
        next_model.stocks = next_stocks.clone();
        next_model.updated_at = chrono::Utc::now();
        let army_context = self
            .load_army_context_in_tx(tx, next_model.village_id)
            .await?;
        let hydrated = hydrate_village(next_model, army_context);

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
        let army = self.load_army_context_in_tx(tx, model.village_id).await?;
        let hero_resources = self.hero_resource_bonus_in_tx(tx, model.village_id).await?;
        Ok(Self::refresh_materialized_village_state(
            model,
            army,
            hero_resources,
        ))
    }

    pub async fn refresh_derived_state_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        let model = self.fetch_village_model_in_tx(tx, village_id).await?;
        let refreshed = self.refresh_for_read_in_tx(tx, model).await?;

        sqlx::query(
            r#"
            UPDATE rm_village
            SET production = $2,
                population = $3,
                culture_points_production = $4,
                total_merchants = $5,
                busy_merchants = $6,
                updated_at = NOW()
            WHERE village_id = $1
            "#,
        )
        .bind(village_id as i32)
        .bind(Json(refreshed.production))
        .bind(refreshed.population as i32)
        .bind(refreshed.culture_points_production as i32)
        .bind(refreshed.total_merchants as i16)
        .bind(refreshed.busy_merchants as i16)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn get_by_village_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let model = self.fetch_village_model_in_tx(tx, village_id).await?;
        self.refresh_for_read_in_tx(tx, model).await
    }
}

fn village_select_sql() -> &'static str {
    r#"
    SELECT village_id, player_id, village_name, position, tribe, buildings, production, stocks,
           population, loyalty, is_capital, culture_points_production, smithy_upgrades, academy_research, parent_village_id,
           total_merchants, busy_merchants, trapper_active_traps, trapper_broken_traps, trapper_queued_traps, loyalty_updated_at, updated_at
    FROM rm_village
    "#
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
    total_merchants: i16,
    busy_merchants: i16,
    trapper_active_traps: i32,
    trapper_broken_traps: i32,
    trapper_queued_traps: i32,
    loyalty_updated_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<DbVillageModelRow> for VillageModel {
    type Error = ApplicationError;

    fn try_from(value: DbVillageModelRow) -> Result<Self, Self::Error> {
        let buildings = value
            .buildings
            .0
            .into_iter()
            .filter(|building| (1..=18).contains(&building.slot_id) || building.building.level > 0)
            .collect();

        Ok(VillageModel {
            village_id: value.village_id as u32,
            player_id: value.player_id,
            village_name: value.village_name,
            position: value.position.0,
            tribe: value.tribe.into(),
            buildings,
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
            trapper: TrapperState {
                active_traps: value.trapper_active_traps.max(0) as u32,
                broken_traps: value.trapper_broken_traps.max(0) as u32,
                queued_traps: value.trapper_queued_traps.max(0) as u32,
            },
            loyalty_updated_at: value.loyalty_updated_at,
            updated_at: value.updated_at,
            parent_village_id: value.parent_village_id.map(|v| v as u32),
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
        let rows: Vec<DbVillageModelRow> = sqlx::query_as(&format!(
            r#"
            {}
            WHERE player_id = $1
            ORDER BY village_id ASC
            "#,
            village_select_sql()
        ))
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
        let rows: Vec<DbVillageModelRow> = sqlx::query_as(&format!(
            r#"
            {}
            WHERE village_id = ANY($1)
            ORDER BY village_id ASC
            "#,
            village_select_sql()
        ))
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

    async fn get_expansion_culture_snapshot(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<ExpansionCultureSnapshot, ApplicationError> {
        let row: Option<(i32, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
              v.culture_points_production AS village_cpp,
              SUM(rv.culture_points_production)::bigint AS player_cpp,
              COUNT(rv.village_id)::bigint AS village_count
            FROM rm_village v
            JOIN rm_village rv ON rv.player_id = v.player_id
            WHERE v.village_id = $1
              AND v.player_id = $2
            GROUP BY v.village_id, v.culture_points_production
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let Some((village_cpp, player_cpp, village_count)) = row else {
            return Err(ApplicationError::Db(DbError::VillageNotFound(village_id)));
        };

        Ok(ExpansionCultureSnapshot {
            village_culture_points_production: village_cpp as u32,
            player_culture_points_production: player_cpp as u32,
            player_village_count: village_count.max(0) as usize,
        })
    }

    async fn count_child_villages(
        &self,
        player_id: Uuid,
        parent_village_id: u32,
    ) -> Result<u8, ApplicationError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM rm_village
            WHERE player_id = $1
              AND parent_village_id = $2
            "#,
        )
        .bind(player_id)
        .bind(parent_village_id as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(count.min(u8::MAX as i64) as u8)
    }

    async fn get_expansion_ownership_snapshot(
        &self,
        player_id: Uuid,
        source_village_id: u32,
    ) -> Result<ExpansionOwnershipSnapshot, ApplicationError> {
        let (source_child_villages, player_village_count): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
              COUNT(*) FILTER (WHERE parent_village_id = $1)::bigint AS source_child_villages,
              COUNT(*)::bigint AS player_village_count
            FROM rm_village
            WHERE player_id = $2
            "#,
        )
        .bind(source_village_id as i32)
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(ExpansionOwnershipSnapshot {
            source_child_villages: source_child_villages.min(u8::MAX as i64) as u8,
            player_village_count: player_village_count.max(0) as usize,
        })
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
    ) -> Result<(), ApplicationError> {
        self.upsert_from_village_inner(
            None,
            village_id,
            player_id,
            village_name,
            position,
            tribe,
            parent_village_id,
            buildings,
        )
        .await
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

    async fn update_building(
        &self,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    ) -> Result<(), ApplicationError> {
        let model = self.fetch_village_model(village_id).await?;

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

        let warehouse_capacity = Self::warehouse_capacity_for_buildings(&next_buildings);
        let granary_capacity = Self::granary_capacity_for_buildings(&next_buildings);
        let total_merchants = next_buildings
            .iter()
            .filter(|b| b.building.name == BuildingName::Marketplace)
            .map(|b| b.building.level)
            .max()
            .unwrap_or(0);

        let current_stocks = {
            let army_context = self.load_army_context(model.village_id).await?;
            let current = hydrate_village(model.clone(), army_context);
            current.stocks().clone()
        };

        let mut next_stocks = current_stocks;
        next_stocks.warehouse_capacity = warehouse_capacity;
        next_stocks.granary_capacity = granary_capacity;
        next_stocks.lumber = next_stocks.lumber.min(warehouse_capacity);
        next_stocks.clay = next_stocks.clay.min(warehouse_capacity);
        next_stocks.iron = next_stocks.iron.min(warehouse_capacity);
        next_stocks.crop = next_stocks.crop.min(granary_capacity as i64);

        let mut next_model = model;
        next_model.buildings = next_buildings.clone();
        next_model.stocks = next_stocks.clone();
        next_model.updated_at = chrono::Utc::now();
        let army_context = self.load_army_context(next_model.village_id).await?;
        let hydrated = hydrate_village(next_model, army_context);

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
        let model = self.fetch_village_model(village_id).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_game::models::buildings::Building;

    #[test]
    fn default_storage_capacity_for_founded_village_uses_inferred_server_speed() {
        let buildings = vec![
            VillageBuilding {
                slot_id: 1,
                building: Building::new(BuildingName::Woodcutter, 3)
                    .at_level(0, 3)
                    .unwrap(),
            },
            VillageBuilding {
                slot_id: 2,
                building: Building::new(BuildingName::Cropland, 3)
                    .at_level(0, 3)
                    .unwrap(),
            },
        ];

        assert_eq!(
            PostgresVillageRepository::warehouse_capacity_for_buildings(&buildings),
            2400
        );
        assert_eq!(
            PostgresVillageRepository::granary_capacity_for_buildings(&buildings),
            2400
        );
    }
}
