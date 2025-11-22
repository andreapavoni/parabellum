// parabellum_app/src/command_handlers/create_hero.rs
use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateHero},
    uow::UnitOfWork,
};
use parabellum_types::{errors::{ApplicationError, GameError}, Result};
use parabellum_game::models::hero::Hero;
use parabellum_types::buildings::{BuildingName, BuildingRequirement};
use std::sync::Arc;

pub struct CreateHeroCommandHandler {}

impl Default for CreateHeroCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateHeroCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateHero> for CreateHeroCommandHandler {
    async fn handle(
        &self,
        command: CreateHero,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let hero_repo = uow.heroes();

        let village = village_repo.get_by_id(command.village_id).await?;
        if village.player_id != command.player_id {
            return Err(GameError::VillageNotOwned {
                village_id: command.village_id,
                player_id: command.player_id,
            }
            .into());
        }

        village
            .validate_building_requirements(&[BuildingRequirement(BuildingName::HeroMansion, 1)])?;
        let new_hero = Hero::new(
            Some(command.id),
            village.id,
            command.player_id,
            village.tribe,
            Some(5),
        );

        // Persist the new hero
        hero_repo.save(&new_hero).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
        village_factory,
    };
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use super::*;
    use crate::{config::Config, cqrs::commands::CreateHero, test_utils::tests::MockUnitOfWork};

    #[tokio::test]
    async fn test_create_hero_handler_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateHeroCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        let hero_building = parabellum_game::models::buildings::Building::new(
            BuildingName::HeroMansion,
            config.speed,
        );
        village.add_building_at_slot(hero_building, 25).unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateHero::new(None, player.id, village.id);
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        );

        let created_hero = mock_uow.heroes().get_by_id(command.id).await.unwrap();
        assert_eq!(created_hero.player_id, player.id);
        assert_eq!(created_hero.health, 100);
        assert_eq!(created_hero.experience, 0);
        assert_eq!(created_hero.strength_points, 0);
    }
}
