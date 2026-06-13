//! Hero read-model and revival action projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
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
        let (village_id, hero) = match event {
            VillageEvent::HeroCreated {
                village_id, hero, ..
            }
            | VillageEvent::HeroRevived {
                village_id, hero, ..
            } => (village_id, hero),
            _ => unreachable!("project_hero_home called with non-hero-home event"),
        };
        self.heroes
            .upsert_in_tx(tx, hero, *village_id, *village_id, "home")
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
