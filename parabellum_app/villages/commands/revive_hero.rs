use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct ReviveHero {
    pub action_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub hero: Hero,
    pub reset: bool,
    pub speed: i8,
    pub revive_at: DateTime<Utc>,
}

impl Command for ReviveHero {
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
        if self.hero.is_alive() {
            return Err(as_domain_error(GameError::HeroNotDead));
        }

        aggregate
            .village()
            .validate_hero_creation_requirements()
            .map_err(as_domain_error)?;

        let cost = self.hero.resurrection_cost(self.speed).resources;
        aggregate
            .village()
            .validate_can_deduct_resources(&cost)
            .map_err(as_domain_error)?;

        Ok(vec![VillageEvent::HeroRevivalScheduled {
            action_id: self.action_id,
            player_id: self.player_id,
            village_id: self.village_id,
            hero: self.hero.clone(),
            reset: self.reset,
            revive_at: self.revive_at,
            cost,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use mini_cqrs_es::Command;
    use parabellum_game::models::{buildings::Building, hero::Hero, village::VillageBuilding};
    use parabellum_types::{buildings::BuildingName, common::ResourceGroup, tribe::Tribe};
    use uuid::Uuid;

    use crate::villages::{ReviveHero, VillageAggregate, VillageEvent};

    fn building(slot_id: u8, name: BuildingName, level: u8) -> VillageBuilding {
        VillageBuilding {
            slot_id,
            building: Building::new(name, 1)
                .at_level(level, 1)
                .expect("building data should be available for hero revival tests"),
        }
    }

    fn dead_hero(player_id: Uuid, village_id: u32) -> Hero {
        let mut hero = Hero::new(None, village_id, player_id, Tribe::Roman, Some(5));
        hero.apply_battle_damage(1.0);
        hero
    }

    fn revive_command(player_id: Uuid, village_id: u32, hero: Hero) -> ReviveHero {
        ReviveHero {
            action_id: Uuid::new_v4(),
            player_id,
            village_id,
            hero,
            reset: false,
            speed: 1,
            revive_at: Utc::now() + Duration::minutes(1),
        }
    }

    fn hero_revival_buildings(include_hero_mansion: bool) -> Vec<VillageBuilding> {
        let mut buildings = vec![
            building(19, BuildingName::MainBuilding, 1),
            building(20, BuildingName::Warehouse, 20),
            building(21, BuildingName::Granary, 20),
        ];
        if include_hero_mansion {
            buildings.push(building(25, BuildingName::HeroMansion, 1));
        }
        buildings
    }

    #[tokio::test]
    async fn rejects_revive_without_hero_mansion() {
        let player_id = Uuid::new_v4();
        let mut aggregate = VillageAggregate::founded(1, player_id, hero_revival_buildings(false));
        aggregate.set_resources_for_test(ResourceGroup::new(80_000, 80_000, 80_000, 80_000));

        let result = revive_command(player_id, 1, dead_hero(player_id, 1))
            .handle(&aggregate)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_revive_when_hero_is_alive() {
        let player_id = Uuid::new_v4();
        let mut aggregate = VillageAggregate::founded(1, player_id, hero_revival_buildings(true));
        aggregate.set_resources_for_test(ResourceGroup::new(80_000, 80_000, 80_000, 80_000));

        let hero = Hero::new(None, 1, player_id, Tribe::Roman, Some(5));
        let result = revive_command(player_id, 1, hero).handle(&aggregate).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_revive_for_dead_hero_with_requirements() {
        let player_id = Uuid::new_v4();
        let mut aggregate = VillageAggregate::founded(1, player_id, hero_revival_buildings(true));
        aggregate.set_resources_for_test(ResourceGroup::new(80_000, 80_000, 80_000, 80_000));
        let hero = dead_hero(player_id, 1);

        let events = revive_command(player_id, 1, hero.clone())
            .handle(&aggregate)
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        let VillageEvent::HeroRevivalScheduled {
            player_id: event_player_id,
            village_id,
            hero: event_hero,
            ..
        } = &events[0]
        else {
            panic!("expected HeroRevivalScheduled");
        };
        assert_eq!(*event_player_id, player_id);
        assert_eq!(*village_id, 1);
        assert_eq!(event_hero.id, hero.id);
    }
}
