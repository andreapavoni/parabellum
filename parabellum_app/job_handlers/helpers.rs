use parabellum_types::errors::ApplicationError;
use crate::uow::UnitOfWork;
use parabellum_game::models::village::Village;

/// Fetches the defender's alliance metallurgy bonus multiplier.
/// Returns 0.0 if the defender has no alliance or if the alliance fetch fails.
pub async fn get_defender_alliance_metallurgy_bonus(
    uow: &Box<dyn UnitOfWork<'_> + '_>,
    defender_village: &Village,
) -> Result<f64, ApplicationError> {
    let def_player = uow.players().get_by_id(defender_village.player_id).await?;

    if let Some(alliance_id) = def_player.alliance_id {
        if let Ok(alliance) = uow.alliances().get_by_id(alliance_id).await {
            return Ok(alliance.get_metallurgy_bonus_multiplier());
        }
    }

    Ok(0.0)
}
