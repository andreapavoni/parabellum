use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_game::battle::BattleReport;
use parabellum_game::models::army::Army;
use parabellum_types::battle::AttackType;
use parabellum_types::errors::AppError;
use uuid::Uuid;

use crate::villages::models::TrappedTroopReturn;
use crate::villages::{VillageAggregate, VillageEvent, commands::as_invariant_error};

#[derive(Debug, Clone)]
pub struct ResolveAttackBattle {
    pub action_id: Uuid,
    pub movement_id: Uuid,
    pub return_action_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub attack_type: AttackType,
    pub report: BattleReport,
    pub returning_army: Option<Army>,
    pub trapped_attacker_army: Option<Army>,
    pub freed_trapped_army_ids: Vec<Uuid>,
    pub freed_trapped_returns: Vec<TrappedTroopReturn>,
    pub stationed_attacker_army: Option<Army>,
    pub returns_at: DateTime<Utc>,
}

impl ResolveAttackBattle {
    pub fn into_outcome_event(self) -> VillageEvent {
        VillageEvent::AttackBattleResolved {
            action_id: self.action_id,
            movement_id: self.movement_id,
            return_action_id: self.return_action_id,
            army_id: self.army_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            attack_type: self.attack_type,
            report: self.report,
            returning_army: self.returning_army,
            trapped_attacker_army: self.trapped_attacker_army,
            freed_trapped_army_ids: self.freed_trapped_army_ids,
            freed_trapped_returns: self.freed_trapped_returns,
            stationed_attacker_army: self.stationed_attacker_army,
            returns_at: self.returns_at,
        }
    }
}

impl Command for ResolveAttackBattle {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(as_invariant_error(AppError::InvalidAggregateTarget {
                expected: self.source_village_id,
                actual: aggregate.aggregate_id(),
            }));
        }

        Ok(vec![self.clone().into_outcome_event()])
    }
}
