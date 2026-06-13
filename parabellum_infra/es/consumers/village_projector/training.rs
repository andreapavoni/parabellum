//! Unit training and research projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_training_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::UnitTrainingScheduled { .. } => {
                Some(self.project_unit_training_scheduled(tx, event).await)
            }
            VillageEvent::UnitTrained { .. } => Some(self.project_unit_trained(tx, event).await),
            VillageEvent::TrapBuildScheduled { .. } => {
                Some(self.project_trap_build_scheduled(tx, event).await)
            }
            VillageEvent::TrapBuilt { .. } => Some(self.project_trapper_state(tx, event).await),
            VillageEvent::AcademyResearchScheduled { .. }
            | VillageEvent::SmithyResearchScheduled { .. } => {
                Some(self.project_research_scheduled(tx, event).await)
            }
            VillageEvent::AcademyResearchCompleted { .. } => {
                Some(self.project_academy_research_completed(tx, event).await)
            }
            VillageEvent::SmithyResearchCompleted { .. } => {
                Some(self.project_smithy_research_completed(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_trap_build_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let action = workflows::traps::scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &action).await?;
        let VillageEvent::TrapBuildScheduled {
            village_id,
            cost,
            trapper,
            ..
        } = event
        else {
            unreachable!("project_trap_build_scheduled called with non-TrapBuildScheduled event");
        };
        self.update_trapper_state_in_tx(tx, *village_id, *trapper)
            .await?;
        self.deduct_village_resources_in_tx(tx, *village_id, cost)
            .await
    }

    async fn project_trapper_state(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        match event {
            VillageEvent::TrapBuilt {
                village_id,
                trapper,
                ..
            }
            | VillageEvent::TrappedTroopsReleased {
                trapped_village_id: village_id,
                trapper,
                ..
            }
            | VillageEvent::TrappedTroopsDisbanded {
                trapped_village_id: village_id,
                trapper,
                ..
            } => {
                self.update_trapper_state_in_tx(tx, *village_id, *trapper)
                    .await
            }
            _ => unreachable!("project_trapper_state called with non-trapper state event"),
        }
    }

    async fn update_trapper_state_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        trapper: parabellum_game::models::trapper::TrapperState,
    ) -> Result<(), CqrsError> {
        let mut current = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        current.trapper = trapper;
        self.village
            .replace_village_state_in_tx(tx, &current)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_unit_training_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let scheduled = workflows::training::scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &scheduled.action)
            .await?;
        self.deduct_village_resources_in_tx(tx, scheduled.village_id, &scheduled.cost)
            .await
    }

    async fn project_unit_trained(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::UnitTrained {
            village_id,
            unit,
            quantity_trained,
            ..
        } = event
        else {
            unreachable!("project_unit_trained called with non-UnitTrained event");
        };
        let current = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let player_id = current.player_id;
        let mut village = self
            .village_from_model_with_armies_in_tx(tx, current)
            .await?;
        village
            .add_trained_units_home(unit.clone(), *quantity_trained)
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let next_army = village.army().cloned();
        if let Some(army) = &next_army {
            self.armies
                .upsert_home_in_tx(tx, army, player_id)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        }
        Ok(())
    }

    async fn project_research_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let scheduled = workflows::research::scheduled_action_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &scheduled.action)
            .await?;
        self.deduct_village_resources_in_tx(tx, scheduled.village_id, &scheduled.cost)
            .await
    }

    async fn project_academy_research_completed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::AcademyResearchCompleted {
            village_id, unit, ..
        } = event
        else {
            unreachable!(
                "project_academy_research_completed called with non-AcademyResearchCompleted event"
            );
        };
        let current = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut village = Self::village_from_model(&current);
        village
            .research_academy(unit.clone())
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut next = current.clone();
        next.academy_research = village.academy_research().clone();
        self.village
            .replace_village_state_in_tx(tx, &next)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_smithy_research_completed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::SmithyResearchCompleted {
            village_id, unit, ..
        } = event
        else {
            unreachable!(
                "project_smithy_research_completed called with non-SmithyResearchCompleted event"
            );
        };
        let current = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut village = Self::village_from_model(&current);
        village
            .upgrade_smithy(unit.clone())
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut next = current.clone();
        next.smithy_upgrades = *village.smithy();
        self.village
            .replace_village_state_in_tx(tx, &next)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}
