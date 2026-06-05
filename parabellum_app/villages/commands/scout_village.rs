use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::army::TroopSet;
use parabellum_types::battle::{AttackType, ScoutingTarget};
use uuid::Uuid;

use crate::villages::{
    ArmyDispatch, ArmyDispatchRequest, VillageAggregate, VillageEvent, commands::as_domain_error,
};

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
        let detached_army = ArmyDispatch::detach_from_home(
            aggregate.village(),
            ArmyDispatchRequest {
                army_id: self.movement_id,
                source_village_id,
                target_village_id: self.target_village_id,
                player_id: self.player_id,
                units: self.units.clone(),
                hero_id: None,
                allow_hero: false,
                scouts_only: true,
            },
        )
        .map_err(as_domain_error)?;
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
