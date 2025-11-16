use std::sync::Arc;
use uuid::Uuid;

use crate::{
    repository::{ArmyRepository, HeroRepository, VillageRepository},
    uow::UnitOfWork,
};
use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::{
    army::{Army, TroopSet},
    village::Village,
};

/// Handles the logic of deploying an army from a village.
/// Returns the updated Village and the new deployed Army.
pub async fn deploy_army_from_village(
    uow: &Box<dyn UnitOfWork<'_> + '_>,
    mut village: Village, // Take ownership to modify
    home_army_id: Uuid,
    units_to_deploy: TroopSet,
    hero_id: Option<Uuid>,
) -> Result<(Village, Army), ApplicationError> {
    if units_to_deploy.iter().sum::<u32>() == 0 && hero_id.is_none() {
        return Err(ApplicationError::Game(GameError::NoUnitsSelected));
    }
    let army_repo: Arc<dyn ArmyRepository + '_> = uow.armies();
    let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
    let hero_repo: Arc<dyn HeroRepository + '_> = uow.heroes();

    let mut home_army = army_repo.get_by_id(home_army_id).await?;
    let attacker_village = village_repo.get_by_id(village.id).await?;

    let hero = if let (Some(id), Some(home_hero)) = (hero_id, home_army.hero()) {
        let hero = hero_repo.get_by_id(id).await?;
        home_army.set_hero(None);

        if !(hero.village_id == attacker_village.id
            && hero.player_id == attacker_village.player_id
            && home_hero.id == hero.id)
        {
            return Err(ApplicationError::Game(GameError::HeroNotAtHome {
                hero_id: hero.id,
                village_id: attacker_village.id,
            }));
        }
        Some(hero)
    } else {
        None
    };

    let deployed_army = home_army.deploy(units_to_deploy, hero)?;
    if home_army.immensity() == 0 {
        army_repo.remove(home_army_id).await?;
        village.set_army(None)?;
    } else {
        army_repo.save(&home_army).await?;
        village.set_army(Some(&home_army))?;
    }

    village_repo.save(&village).await?;
    army_repo.save(&deployed_army).await?;

    Ok((village, deployed_army))
}
