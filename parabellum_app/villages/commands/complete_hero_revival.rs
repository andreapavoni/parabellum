use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct CompleteHeroRevival {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub hero: Hero,
    pub reset: bool,
    pub revived_at: DateTime<Utc>,
}

impl Command for CompleteHeroRevival {
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

        let mut hero = self.hero.clone();
        hero.resurrect(self.village_id, self.reset);

        Ok(vec![VillageEvent::HeroRevived {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            hero,
            reset: self.reset,
            revived_at: self.revived_at,
        }])
    }
}
