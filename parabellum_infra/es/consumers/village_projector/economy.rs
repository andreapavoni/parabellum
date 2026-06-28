//! Village economy fact projection helpers.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::VillageModel;
use parabellum_types::common::ResourceGroup;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;

/// Fact-carried economy values that should be materialized together.
///
/// These are read-model facts, not domain commands. Use them when an event
/// already carries absolute stored resources or busy merchant counts.
pub(super) struct VillageEconomyFacts {
    pub stored_resources: Option<ResourceGroup>,
    pub busy_merchants: Option<u8>,
}

impl VillageEconomyFacts {
    pub fn stored_resources(resources: ResourceGroup) -> Self {
        Self {
            stored_resources: Some(resources),
            busy_merchants: None,
        }
    }

    pub fn busy_merchants(busy_merchants: u8) -> Self {
        Self {
            stored_resources: None,
            busy_merchants: Some(busy_merchants),
        }
    }

    pub fn stored_resources_and_busy_merchants(
        resources: ResourceGroup,
        busy_merchants: u8,
    ) -> Self {
        Self {
            stored_resources: Some(resources),
            busy_merchants: Some(busy_merchants),
        }
    }
}

impl VillageProjector {
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
        let mut source = self.load_village_state_in_tx(tx, source).await?;
        source
            .deduct_resources(cost)
            .map_err(CqrsError::domain_source)?;
        self.apply_village_economy_facts_in_tx(
            tx,
            village_id,
            VillageEconomyFacts::stored_resources(source.stored_resources()),
        )
        .await
    }

    pub(super) async fn apply_village_economy_facts_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        facts: VillageEconomyFacts,
    ) -> Result<(), CqrsError> {
        let mut model = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        apply_village_economy_facts_to_model(&mut model, facts);
        self.village
            .store_village_model_in_tx(tx, &model)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

/// Applies fact-carried economy values to an already loaded village model.
pub(super) fn apply_village_economy_facts_to_model(
    model: &mut VillageModel,
    facts: VillageEconomyFacts,
) {
    if let Some(resources) = facts.stored_resources {
        apply_stored_resources_fact_to_model(model, resources);
    }
    if let Some(busy_merchants) = facts.busy_merchants {
        apply_busy_merchants_fact_to_model(model, busy_merchants);
    }
}

fn apply_stored_resources_fact_to_model(model: &mut VillageModel, resources: ResourceGroup) {
    model.stocks.lumber = resources.lumber().min(model.stocks.warehouse_capacity);
    model.stocks.clay = resources.clay().min(model.stocks.warehouse_capacity);
    model.stocks.iron = resources.iron().min(model.stocks.warehouse_capacity);
    model.stocks.crop = (resources.crop() as i64).min(model.stocks.granary_capacity as i64);
}

fn apply_busy_merchants_fact_to_model(model: &mut VillageModel, busy_merchants: u8) {
    model.busy_merchants = busy_merchants.min(model.total_merchants);
}
