// parabellum_app/src/command_handlers/create_hero.rs
use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateHero},
    uow::UnitOfWork,
};
use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::hero::Hero;
use parabellum_types::buildings::BuildingName;
use std::sync::Arc;

pub struct CreateHeroCommandHandler {}

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

        // Fetch the target village and validate ownership
        let village = village_repo.get_by_id(command.village_id).await?;
        if village.player_id != command.player_id {
            return Err(GameError::VillageNotOwned {
                village_id: command.village_id,
                player_id: command.player_id,
            }
            .into());
        }

        // Validate that the village has a HeroMansion of level >= 1
        let level = village
            .get_building_by_name(&BuildingName::HeroMansion)
            .map(|vb| vb.building.level)
            .unwrap_or(0);
        if level < 1 {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::HeroMansion,
                level: 1,
            }
            .into());
        }

        // Create the Hero domain object with default attributes
        let new_hero = Hero {
            id: command.id,
            village_id: village.id,
            player_id: command.player_id,
            health: 100,
            experience: 0,
            attack_points: 0,
            defense_points: 0,
            off_bonus: 0,
            def_bonus: 0,
        };

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

        // Setup a player and a village with HeroMansion level 1
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
        // Manually add a HeroMansion at level 1 to the village
        let hero_building = parabellum_game::models::buildings::Building::new(
            BuildingName::HeroMansion,
            config.speed,
        );
        village.add_building_at_slot(hero_building, 25).unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        // Execute the CreateHero command
        let command = CreateHero::new(None, player.id, village.id);
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        );

        // Verify that the hero was created in the repository
        let created_hero = mock_uow.heroes().get_by_id(command.id).await.unwrap();
        assert_eq!(created_hero.player_id, player.id);
        assert_eq!(created_hero.health, 100);
        assert_eq!(created_hero.experience, 0);
        assert_eq!(created_hero.attack_points, 0);
        assert_eq!(created_hero.defense_points, 0);
    }
}
