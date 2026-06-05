use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::army::TroopSet;
use parabellum_types::battle::AttackType;
use parabellum_types::buildings::BuildingName;
use uuid::Uuid;

use crate::villages::{
    ArmyDispatch, ArmyDispatchRequest, VillageAggregate, VillageEvent, commands::as_domain_error,
};

#[derive(Debug, Clone)]
/// Schedules an attack trip from source village to target village.
pub struct AttackVillage {
    pub movement_id: Uuid,
    pub arrival_action_id: Uuid,
    pub return_action_id: Uuid,
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub attack_type: AttackType,
    pub catapult_targets: [Option<BuildingName>; 2],
    pub arrives_at: DateTime<Utc>,
    pub returns_at: DateTime<Utc>,
}

impl Command for AttackVillage {
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
                hero_id: self.hero_id,
                allow_hero: true,
                scouts_only: false,
            },
        )
        .map_err(as_domain_error)?;
        Ok(vec![
            VillageEvent::VillageArmyDetached {
                army: detached_army.clone(),
            },
            VillageEvent::AttackSent {
                movement_id: self.movement_id,
                army_id: self.movement_id,
                arrival_action_id: self.arrival_action_id,
                return_action_id: self.return_action_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                army: detached_army.clone(),
                attack_type: self.attack_type.clone(),
                catapult_targets: self.catapult_targets.clone(),
                arrives_at: self.arrives_at,
                returns_at: self.returns_at,
            },
            VillageEvent::AttackArrivalScheduled {
                action_id: self.arrival_action_id,
                movement_id: self.movement_id,
                return_action_id: self.return_action_id,
                player_id: self.player_id,
                source_village_id,
                target_village_id: self.target_village_id,
                army_id: self.movement_id,
                army: detached_army,
                attack_type: self.attack_type.clone(),
                catapult_targets: self.catapult_targets.clone(),
                arrives_at: self.arrives_at,
                returns_at: self.returns_at,
            },
        ])
    }
}
