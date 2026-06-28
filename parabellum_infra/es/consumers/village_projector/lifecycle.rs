//! Village lifecycle and simple village-state projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_game::models::{
    trapper::TrapperState,
    village::{AcademyResearch, Village, VillageSnapshot, VillageStocks},
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::economy::VillageEconomyFacts;
use crate::es::consumers::village_projector::VillageProjector;

/// Read-model fact emitted after conquest ownership changes.
///
/// Battle resolution owns the combat and loyalty calculation. This projector
/// fact only materializes the final owner, parent village, and loyalty values.
struct VillageConquestFact {
    new_player_id: Uuid,
    parent_village_id: u32,
    loyalty_after: u8,
}

impl VillageProjector {
    pub(super) async fn project_lifecycle_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        aggregate_id: &str,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::VillageFounded { .. } => {
                Some(self.project_village_founded(tx, event).await)
            }
            VillageEvent::VillageConquered { .. } => Some(
                self.project_village_conquered(tx, event, aggregate_id)
                    .await,
            ),
            VillageEvent::VillageResourcesSet { .. } => {
                Some(self.project_village_resources_set(tx, event).await)
            }
            VillageEvent::VillageRenamed { .. } => {
                Some(self.project_village_renamed(tx, event).await)
            }
            _ => None,
        }
    }

    async fn project_village_founded(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageFounded {
            village_id,
            village_name,
            position,
            tribe,
            player_id,
            parent_village_id,
            buildings,
            ..
        } = event
        else {
            unreachable!("project_village_founded called with non-VillageFounded event");
        };
        let model = founded_village_model(
            *village_id,
            *player_id,
            village_name,
            position,
            tribe.clone(),
            *parent_village_id,
            buildings.clone(),
        );
        self.village
            .upsert_village_model_in_tx(tx, &model)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.map
            .set_occupancy_in_tx(tx, *village_id, Some(*village_id), Some(*player_id))
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_village_conquered(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
        aggregate_id: &str,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageConquered {
            player_id,
            owner_village_id,
        } = event
        else {
            unreachable!("project_village_conquered called with non-VillageConquered event");
        };
        let village_id = aggregate_id
            .parse::<u32>()
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut conquered = self
            .village
            .get_by_village_id_in_tx(tx, village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        apply_conquest_fact_to_model(
            &mut conquered,
            VillageConquestFact {
                new_player_id: *player_id,
                parent_village_id: *owner_village_id,
                loyalty_after: 0,
            },
        );
        self.village
            .store_village_model_in_tx(tx, &conquered)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.map
            .set_occupancy_in_tx(tx, village_id, Some(village_id), Some(*player_id))
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_village_resources_set(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageResourcesSet {
            village_id,
            resources,
            ..
        } = event
        else {
            unreachable!("project_village_resources_set called with non-VillageResourcesSet event");
        };
        self.apply_village_economy_facts_in_tx(
            tx,
            *village_id,
            VillageEconomyFacts::stored_resources(resources.clone()),
        )
        .await
    }

    async fn project_village_renamed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::VillageRenamed {
            village_id,
            village_name,
            ..
        } = event
        else {
            unreachable!("project_village_renamed called with non-VillageRenamed event");
        };
        let mut village = self
            .village
            .get_by_village_id_in_tx(tx, *village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        village.village_name = village_name.clone();
        self.village
            .store_village_model_in_tx(tx, &village)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

/// Applies a conquest lifecycle fact to an already loaded village model.
fn apply_conquest_fact_to_model(model: &mut VillageModel, fact: VillageConquestFact) {
    model.player_id = fact.new_player_id;
    model.parent_village_id = Some(fact.parent_village_id);
    model.loyalty = fact.loyalty_after;
}

fn founded_village_model(
    village_id: u32,
    player_id: Uuid,
    village_name: &str,
    position: &parabellum_types::map::Position,
    tribe: parabellum_types::tribe::Tribe,
    parent_village_id: Option<u32>,
    buildings: Vec<parabellum_game::models::village::VillageBuilding>,
) -> VillageModel {
    let now = chrono::Utc::now();
    let village = Village::rehydrate(VillageSnapshot {
        id: village_id,
        name: village_name.to_string(),
        player_id,
        position: position.clone(),
        tribe: tribe.clone(),
        buildings: buildings.clone(),
        oases: vec![],
        army: None,
        reinforcements: vec![],
        deployed_armies: vec![],
        loyalty: 100,
        is_capital: parent_village_id.is_none(),
        smithy: [0_u8; 8],
        stocks: VillageStocks::default_for_speed(inferred_server_speed(&buildings)),
        academy_research: AcademyResearch::default(),
        culture_points: 0,
        updated_at: now,
        parent_village_id,
    });

    VillageModel {
        village_id,
        player_id,
        village_name: village.name.clone(),
        position: village.position.clone(),
        tribe: village.tribe.clone(),
        buildings: village.buildings().clone(),
        production: village.production.clone(),
        stocks: village.stocks().clone(),
        population: village.population,
        loyalty: village.loyalty(),
        loyalty_updated_at: now,
        is_capital: village.is_capital,
        culture_points_production: village.culture_points_production,
        smithy_upgrades: *village.smithy(),
        academy_research: village.academy_research().clone(),
        total_merchants: village.total_merchants,
        busy_merchants: village.busy_merchants,
        trapper: TrapperState::default(),
        updated_at: village.updated_at,
        parent_village_id: village.parent_village_id,
    }
}

fn inferred_server_speed(buildings: &[parabellum_game::models::village::VillageBuilding]) -> i8 {
    buildings
        .iter()
        .filter_map(|building| building.building.inferred_server_speed())
        .max()
        .unwrap_or(1)
}
