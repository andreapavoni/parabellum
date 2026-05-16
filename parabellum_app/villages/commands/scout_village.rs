use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::models::army::Army;
use parabellum_types::army::{TroopSet, UnitRole};
use parabellum_types::battle::{AttackType, ScoutingTarget};
use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
pub struct ScoutVillage {
    pub movement_id: Uuid,
    pub arrival_action_id: Uuid,
    pub return_action_id: Uuid,
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub target: ScoutingTarget,
    pub attack_type: AttackType,
    pub arrives_at: DateTime<Utc>,
    pub returns_at: DateTime<Utc>,
}

impl Command for ScoutVillage {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        let source_village_id = aggregate.aggregate_id();
        let owner_id = aggregate.village().player_id();

        if owner_id != self.player_id {
            return Err(as_domain_error(GameError::VillageNotOwned {
                village_id: source_village_id,
                player_id: self.player_id,
            }));
        }
        if source_village_id == self.target_village_id {
            return Err(as_domain_error(GameError::VillageCannotTargetItself {
                village_id: source_village_id,
            }));
        }
        if aggregate.village().building_level(BuildingName::RallyPoint) == 0 {
            return Err(as_domain_error(GameError::BuildingRequirementsNotMet {
                building: BuildingName::RallyPoint,
                level: 1,
            }));
        }
        if self.units.immensity() == 0 {
            return Err(as_domain_error(GameError::NoUnitsSelected));
        }
        if !aggregate.village().has_units(&self.units) {
            return Err(as_domain_error(GameError::NotEnoughUnits));
        }

        let tribe_units = aggregate.village().village.tribe.units();
        for (idx, &quantity) in self.units.units().iter().enumerate() {
            if quantity == 0 {
                continue;
            }
            let unit = tribe_units
                .get(idx)
                .ok_or_else(|| as_domain_error(GameError::InvalidUnitIndex(idx as u8)))?;
            if !matches!(unit.role, UnitRole::Scout) {
                return Err(as_domain_error(GameError::OnlyScoutUnitsAllowed));
            }
        }

        let detached_army = Army::new(
            Some(self.movement_id),
            source_village_id,
            Some(self.target_village_id),
            self.player_id,
            aggregate.village().village.tribe.clone(),
            &self.units,
            aggregate.village().village.smithy(),
            None,
        );
        Ok(vec![
            VillageEvent::VillageArmyDetached {
                army: detached_army.clone(),
            },
            VillageEvent::ScoutSent {
                movement_id: self.movement_id,
                army_id: self.movement_id,
                arrival_action_id: self.arrival_action_id,
                return_action_id: self.return_action_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                army: detached_army,
                target: self.target.clone(),
                attack_type: self.attack_type.clone(),
                arrives_at: self.arrives_at,
                returns_at: self.returns_at,
            },
        ])
    }
}
