//! Hero read-model and revival action projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::projection_repositories::{ArmyListFilter, ArmyState};
use parabellum_game::models::army::Army;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_hero_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::HeroCreated { .. } | VillageEvent::HeroRevived { .. } => {
                Some(self.project_hero_home(tx, event).await)
            }
            VillageEvent::HeroUpdated { .. } => Some(self.project_hero_updated(tx, event).await),
            VillageEvent::HeroRevivalScheduled { .. } => {
                Some(self.project_hero_revival_scheduled(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_hero_updated(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::HeroUpdated { hero, .. } = event else {
            unreachable!("project_hero_updated called with non-HeroUpdated event");
        };
        self.heroes
            .update_stats_in_tx(tx, hero)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.village
            .refresh_derived_state_in_tx(tx, hero.village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_hero_home(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let (player_id, village_id, hero) = match event {
            VillageEvent::HeroCreated {
                player_id,
                village_id,
                hero,
                ..
            }
            | VillageEvent::HeroRevived {
                player_id,
                village_id,
                hero,
                ..
            } => (player_id, village_id, hero),
            _ => unreachable!("project_hero_home called with non-hero-home event"),
        };
        self.heroes
            .upsert_in_tx(tx, hero, *village_id, *village_id, "home")
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let village = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut home_armies = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .home_village(*village_id)
                    .current_village(*village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut home_army = home_armies
            .pop()
            .unwrap_or_else(|| Army::new_village_army(&Self::village_from_model(&village)));
        home_army.set_hero(Some(hero.clone()));
        self.armies
            .upsert_home_in_tx(tx, &home_army, *player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_hero_revival_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::HeroRevivalScheduled { .. } = event else {
            unreachable!(
                "project_hero_revival_scheduled called with non-HeroRevivalScheduled event"
            );
        };
        let action = workflows::heroes::revival_scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await
    }
}
