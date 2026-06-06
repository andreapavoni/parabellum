//! Synchronous village event projector for ES read models.
//!
//! This consumer runs in the command transaction scope and must keep read-model
//! updates consistent with event appends.
use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::models::{ScheduledAction, VillageModel};
use parabellum_app::villages::{VillageArmyContext, VillageEvent, hydrate_village};
use parabellum_game::models::village::Village;
use parabellum_types::common::ResourceGroup;
use sqlx::{PgPool, Postgres, Transaction};

use crate::es::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository,
};

mod armies;
mod battle;
mod buildings;
mod foundation;
mod heroes;
mod lifecycle;
mod merchants;
mod training;

#[derive(Clone)]
pub struct VillageProjector {
    pool: PgPool,
    village: PostgresVillageRepository,
    armies: PostgresArmyRepository,
    heroes: PostgresHeroRepository,
    movements: PostgresVillageMovementRepository,
    actions: PostgresScheduledActionRepository,
    offers: PostgresMarketplaceRepository,
    project_operational_actions: bool,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_options(pool, true)
    }

    pub fn new_with_options(pool: PgPool, project_operational_actions: bool) -> Self {
        Self {
            pool: pool.clone(),
            village: PostgresVillageRepository::new(pool.clone()),
            armies: PostgresArmyRepository::new(pool.clone()),
            heroes: PostgresHeroRepository::new(pool.clone()),
            movements: PostgresVillageMovementRepository::new(pool.clone()),
            actions: PostgresScheduledActionRepository::new(pool.clone()),
            offers: PostgresMarketplaceRepository::new(pool),
            project_operational_actions,
        }
    }

    pub(super) fn village_from_model(model: &VillageModel) -> Village {
        Village::from(model.clone())
    }

    pub(super) async fn village_from_model_with_armies_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model: VillageModel,
    ) -> Result<Village, CqrsError> {
        let village_id = model.village_id;
        let armies = VillageArmyContext {
            home: self
                .armies
                .get_home_army_in_tx(tx, village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?,
            stationed: self
                .armies
                .list_stationed_armies_in_tx(tx, village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?,
            deployed: self
                .armies
                .list_deployed_armies_in_tx(tx, village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?,
            moving: self
                .armies
                .list_moving_armies_by_owner_in_tx(tx, village_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?,
        };
        Ok(hydrate_village(model, armies))
    }

    pub(super) async fn deduct_village_resources_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        cost: &ResourceGroup,
    ) -> Result<(), CqrsError> {
        if cost.total() == 0 {
            return Ok(());
        }
        let source = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut source = Self::village_from_model(&source);
        source
            .deduct_resources(cost)
            .map_err(CqrsError::domain_source)?;
        self.village
            .set_stored_resources_in_tx(tx, village_id, source.stored_resources())
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }

    pub(super) async fn add_scheduled_action_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        action: &ScheduledAction,
    ) -> Result<(), CqrsError> {
        if !self.project_operational_actions {
            return Ok(());
        }
        self.actions
            .add_in_tx(tx, action)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub(super) async fn set_stored_resources_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        resources: ResourceGroup,
    ) -> Result<(), CqrsError> {
        self.village
            .set_stored_resources_in_tx(tx, village_id, resources)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub(super) async fn set_busy_merchants_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        busy_merchants: u8,
    ) -> Result<(), CqrsError> {
        self.village
            .set_busy_merchants_in_tx(tx, village_id, busy_merchants)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub(super) async fn refresh_village_derived_state_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<(), CqrsError> {
        self.village
            .refresh_derived_state_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn process_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &StoredEvent,
    ) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        if let Some(result) = self.project_merchant_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self
            .project_army_event_in_tx(tx, &domain_event, &event.aggregate_id)
            .await
        {
            return result;
        }
        if let Some(result) = self.project_battle_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self.project_foundation_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self.project_building_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self.project_training_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self.project_hero_event_in_tx(tx, &domain_event).await {
            return result;
        }
        if let Some(result) = self
            .project_lifecycle_event_in_tx(tx, &domain_event, &event.aggregate_id)
            .await
        {
            return result;
        }

        Ok(())
    }
}

impl EventConsumer for VillageProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.process_in_tx(&mut tx, event).await?;
        tx.commit()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}
