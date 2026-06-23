use std::collections::HashMap;

use async_trait::async_trait;

use parabellum_types::errors::ApplicationError;

use crate::read_models::VillageReference;

/// Read port for compact village references.
#[async_trait]
pub trait VillageReferenceReadPort: Send + Sync {
    /// Resolves compact village references by village id.
    async fn get_village_references(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageReference>, ApplicationError>;
}
