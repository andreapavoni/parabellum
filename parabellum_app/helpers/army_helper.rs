use std::sync::Arc;
use uuid::Uuid;

use crate::{
    repository::{ArmyRepository, VillageRepository},
    uow::UnitOfWork,
};
use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::{
    army::{Army, TroopSet},
    village::Village,
};

/// Handles the logic of deploying an army from a village.
/// Returns the updated Village and the new deployed Army.
pub async fn deploy_army_from_village<'a>(
    uow: &Box<dyn UnitOfWork<'_> + '_>,
    mut village: Village, // Take ownership to modify
    home_army_id: Uuid,
    units_to_deploy: TroopSet,
) -> Result<(Village, Army), ApplicationError> {
    if units_to_deploy.iter().sum::<u32>() == 0 {
        return Err(ApplicationError::Game(GameError::NotUnitsSelected));
    }

    let army_repo: Arc<dyn ArmyRepository + '_> = uow.armies();
    let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();

    let mut home_army = army_repo.get_by_id(home_army_id).await?;
    let deployed_army = home_army.deploy(units_to_deploy)?;

    if home_army.immensity() == 0 {
        army_repo.remove(home_army_id).await?;
        village.army = None;
    } else {
        army_repo.save(&home_army).await?;
        village.army = Some(home_army);
    }

    village.update_state();
    village_repo.save(&village).await?;

    army_repo.save(&deployed_army).await?;

    Ok((village, deployed_army))
}
