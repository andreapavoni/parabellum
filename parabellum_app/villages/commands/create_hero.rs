use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct CreateHero {
    pub hero_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub has_existing_hero: bool,
}

impl Command for CreateHero {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }
        if self.has_existing_hero {
            return Err(as_domain_error(GameError::HeroAlreadyExists));
        }

        aggregate
            .village()
            .validate_hero_creation_requirements()
            .map_err(as_domain_error)?;

        let hero = Hero::new(
            Some(self.hero_id),
            self.village_id,
            self.player_id,
            aggregate.village().tribe().clone(),
            Some(5),
        );

        Ok(vec![VillageEvent::HeroCreated {
            player_id: self.player_id,
            village_id: self.village_id,
            hero,
        }])
    }
}
