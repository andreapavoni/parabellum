//! Hero read-model and revival action projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::projection_repositories::{
    ArmyListFilter, ArmyState, HeroPlacementState,
};
use parabellum_game::battle::{BattlePartyReport, BattleReport};
use parabellum_game::models::{army::Army, hero::Hero};
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    /// Projects hero placement in the hero read model.
    ///
    /// The caller must pass the same home/current placement that the army
    /// projection represents. Hero availability for dispatch still depends on
    /// the matching army projection carrying the same hero.
    pub(super) async fn project_hero_placement_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hero: &Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: HeroPlacementState,
    ) -> Result<(), CqrsError> {
        self.heroes
            .upsert_in_tx(tx, hero, home_village_id, current_village_id, state)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    /// Updates persisted hero stats from a resolved battle report.
    ///
    /// Battle reports are the canonical source for hero health and experience
    /// changes. The army projection may no longer contain a dead hero after the
    /// domain applies losses, so stats must be written from the report itself.
    pub(super) async fn update_battle_hero_stats_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        report: &BattleReport,
    ) -> Result<(), CqrsError> {
        self.update_battle_party_hero_stats_in_tx(tx, &report.attacker)
            .await?;
        if let Some(defender) = &report.defender {
            self.update_battle_party_hero_stats_in_tx(tx, defender)
                .await?;
        }
        for reinforcement in &report.reinforcements {
            self.update_battle_party_hero_stats_in_tx(tx, reinforcement)
                .await?;
        }
        Ok(())
    }

    async fn update_battle_party_hero_stats_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        report: &BattlePartyReport,
    ) -> Result<(), CqrsError> {
        let Some(hero) = report.hero_after_battle() else {
            return Ok(());
        };
        self.heroes
            .update_stats_in_tx(tx, &hero)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

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
        self.project_hero_placement_in_tx(
            tx,
            hero,
            *village_id,
            *village_id,
            HeroPlacementState::Home,
        )
        .await?;

        let village = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let default_home_army =
            Army::new_village_army(&self.load_village_state_in_tx(tx, village).await?);
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
        let mut home_army = home_armies.pop().unwrap_or(default_home_army);
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
