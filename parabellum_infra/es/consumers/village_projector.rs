//! Synchronous village event projector for ES read models.
//!
//! This consumer runs in the command transaction scope and must keep read-model
//! updates consistent with event appends.
use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::ScheduledAction;
use sqlx::{PgPool, Postgres, Transaction};

use crate::es::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository,
};
use crate::map::PostgresMapRepository;

mod armies;
mod battle;
mod buildings;
mod economy;
mod foundation;
mod heroes;
mod hydration;
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
    map: PostgresMapRepository,
    project_operational_actions: bool,
}

impl VillageProjector {
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_options(pool, true)
    }

    pub fn new_with_options(pool: PgPool, project_operational_actions: bool) -> Self {
        Self {
            pool: pool.clone(),
            village: PostgresVillageRepository::new(crate::ProjectionDb::new(pool.clone())),
            armies: PostgresArmyRepository::new(crate::ProjectionDb::new(pool.clone())),
            heroes: PostgresHeroRepository::new(crate::ProjectionDb::new(pool.clone())),
            movements: PostgresVillageMovementRepository::new(crate::ProjectionDb::new(
                pool.clone(),
            )),
            actions: PostgresScheduledActionRepository::new(crate::ProjectionDb::new(pool.clone())),
            offers: PostgresMarketplaceRepository::new(crate::ProjectionDb::new(pool.clone())),
            map: PostgresMapRepository::new(crate::ProjectionDb::new(pool)),
            project_operational_actions,
        }
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
