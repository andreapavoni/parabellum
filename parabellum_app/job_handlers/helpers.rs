use std::sync::Arc;
use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

use crate::{repository::PlayerRepository, uow::UnitOfWork};

/// Updates player's total culture points by aggregating from all their villages.
/// Should be called after any village state change that affects culture points.
pub async fn update_player_culture_points(
    uow: &Box<dyn UnitOfWork<'_> + '_>,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    let player_repo: Arc<dyn PlayerRepository + '_> = uow.players();
    player_repo.update_culture_points(player_id).await?;
    Ok(())
}
