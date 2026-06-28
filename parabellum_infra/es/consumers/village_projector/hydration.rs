//! Domain village hydration for projection workflows.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::hydrate_village;
use parabellum_app::villages::models::VillageModel;
use parabellum_game::models::village::Village;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;

impl VillageProjector {
    /// Loads the complete domain village state represented by a village model.
    pub(super) async fn load_village_state_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model: VillageModel,
    ) -> Result<Village, CqrsError> {
        let village_id = model.village_id;
        let armies = self
            .armies
            .army_context_for_village_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(hydrate_village(model, armies))
    }
}
