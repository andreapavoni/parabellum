use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_game::models::village::{VillageBuilding, VillageProduction, VillageStocks};
use parabellum_types::errors::AppError;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct ApplyBattleOutcomeToVillage {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub target_player_id: Uuid,
    pub target_tribe: Tribe,
    pub target_parent_village_id: Option<u32>,
    pub target_loyalty: u8,
    pub target_buildings: Vec<VillageBuilding>,
    pub target_production: VillageProduction,
    pub target_population: u32,
    pub target_stocks: VillageStocks,
    pub target_army: Option<Army>,
    pub target_reinforcements: Vec<Army>,
    pub stationed_attacker_army: Option<Army>,
}

impl ApplyBattleOutcomeToVillage {
    pub fn into_outcome_event(self) -> VillageEvent {
        VillageEvent::BattleOutcomeAppliedToVillage {
            action_id: self.action_id,
            movement_id: self.movement_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            target_player_id: self.target_player_id,
            target_tribe: self.target_tribe,
            target_parent_village_id: self.target_parent_village_id,
            target_loyalty: self.target_loyalty,
            target_buildings: self.target_buildings,
            target_production: self.target_production,
            target_population: self.target_population,
            target_stocks: self.target_stocks,
            target_army: self.target_army,
            target_reinforcements: self.target_reinforcements,
            stationed_attacker_army: self.stationed_attacker_army,
        }
    }
}

impl Command for ApplyBattleOutcomeToVillage {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.target_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.target_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![self.clone().into_outcome_event()])
    }
}
