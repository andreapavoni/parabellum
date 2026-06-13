use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct ResetHeroPoints {
    pub player_id: Uuid,
    pub village_id: u32,
    pub hero: Hero,
}

impl Command for ResetHeroPoints {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.village().player_id() != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: aggregate.aggregate_id(),
                player_id: self.player_id,
            }));
        }
        if self.hero.player_id != self.player_id {
            return Err(as_domain_error(GameError::HeroNotOwned {
                hero_id: self.hero.id,
                player_id: self.player_id,
            }));
        }
        if self.hero.village_id != self.village_id {
            return Err(as_domain_error(GameError::HeroNotOwned {
                hero_id: self.hero.id,
                player_id: self.player_id,
            }));
        }

        let mut hero = self.hero.clone();
        hero.reset_level_zero_points().map_err(as_domain_error)?;

        Ok(vec![VillageEvent::HeroUpdated {
            player_id: self.player_id,
            village_id: self.village_id,
            hero,
        }])
    }
}
