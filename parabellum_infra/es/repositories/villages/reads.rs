//! Read helpers for village projections.

use parabellum_app::villages::models::VillageModel;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{PostgresVillageRepository, hero_bonus, queries, refresh, rows::DbVillageModelRow};

impl PostgresVillageRepository {
    pub(super) async fn list_refreshed_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        let rows: Vec<DbVillageModelRow> = queries::villages_by_player_query(player_id)
            .build_query_as()
            .fetch_all(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        self.refresh_rows_for_read(rows).await
    }

    pub(super) async fn list_refreshed_by_village_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        if village_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<i32> = village_ids.iter().map(|id| *id as i32).collect();
        let rows: Vec<DbVillageModelRow> = queries::villages_by_ids_query(ids)
            .build_query_as()
            .fetch_all(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        self.refresh_rows_for_read(rows).await
    }

    pub(super) async fn get_refreshed_by_village_id(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let model = self.load_village_model(village_id).await?;
        self.refresh_model_for_read(model).await
    }

    pub(super) async fn load_village_model(
        &self,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let row: Option<DbVillageModelRow> = queries::village_by_id_query(village_id)
            .build_query_as()
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let row = row.ok_or(ApplicationError::Db(DbError::VillageNotFound(village_id)))?;
        row.try_into()
    }

    pub(super) async fn load_village_model_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let row: DbVillageModelRow = queries::village_by_id_query(village_id)
            .build_query_as()
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        row.try_into()
    }

    async fn refresh_rows_for_read(
        &self,
        rows: Vec<DbVillageModelRow>,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        let mut villages = Vec::with_capacity(rows.len());
        for row in rows {
            let model: VillageModel = row.try_into()?;
            villages.push(self.refresh_model_for_read(model).await?);
        }
        Ok(villages)
    }

    async fn refresh_model_for_read(
        &self,
        model: VillageModel,
    ) -> Result<VillageModel, ApplicationError> {
        let army = self.load_army_context(model.village_id).await?;
        let hero_resources = hero_bonus::hero_resource_bonus(self.pool(), model.village_id).await?;
        Ok(refresh::refresh_materialized_village_state(
            model,
            army,
            hero_resources,
        ))
    }

    async fn refresh_model_for_read_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model: VillageModel,
    ) -> Result<VillageModel, ApplicationError> {
        let army = self.load_army_context_in_tx(tx, model.village_id).await?;
        let hero_resources = hero_bonus::hero_resource_bonus_in_tx(tx, model.village_id).await?;
        Ok(refresh::refresh_materialized_village_state(
            model,
            army,
            hero_resources,
        ))
    }

    pub async fn refresh_derived_state_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        let model = self.load_village_model_in_tx(tx, village_id).await?;
        let refreshed = self.refresh_model_for_read_in_tx(tx, model).await?;

        self.store_village_model_in_tx(tx, &refreshed).await
    }

    pub async fn get_by_village_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<VillageModel, ApplicationError> {
        let model = self.load_village_model_in_tx(tx, village_id).await?;
        self.refresh_model_for_read_in_tx(tx, model).await
    }
}
