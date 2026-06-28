//! Merchant movement projection repository contracts.

use parabellum_types::errors::ApplicationError;

use crate::villages::read_models::MerchantMovement;

/// Persistence boundary for active merchant movement projections.
///
/// Merchant movements are derived from active scheduled merchant actions, not
/// marketplace offer rows.
#[async_trait::async_trait]
pub trait MerchantMovementRepository: Send + Sync {
    /// Lists active merchant movements relative to `village_id`.
    async fn list_active_for_village(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError>;
}
