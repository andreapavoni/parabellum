use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::army::TroopSet;
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent};

#[derive(Debug, Clone)]
pub struct ReinforcementArrived {
    pub movement_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub arrives_at: DateTime<Utc>,
}

impl Command for ReinforcementArrived {
    type Aggregate = VillageAggregate;

    async fn handle(&self, aggregate: &Self::Aggregate) -> Result<Vec<VillageEvent>, CqrsError> {
        if aggregate.aggregate_id() != self.source_village_id {
            return Err(CqrsError::Domain(
                "reinforcement arrival must be executed on source village stream".to_string(),
            ));
        }

        Ok(vec![VillageEvent::ReinforcementArrived {
            movement_id: self.movement_id,
            army_id: self.army_id,
            player_id: self.player_id,
            source_village_id: self.source_village_id,
            target_village_id: self.target_village_id,
            units: self.units.clone(),
            hero_id: self.hero_id,
            arrives_at: self.arrives_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mini_cqrs_es::Command;
    use parabellum_types::army::TroopSet;
    use uuid::Uuid;

    use crate::villages::{ReinforcementArrived, VillageAggregate, VillageEvent};

    #[tokio::test]
    async fn emits_reinforcement_arrived_event() {
        let aggregate = VillageAggregate::default();
        let command = ReinforcementArrived {
            movement_id: Uuid::new_v4(),
            army_id: Uuid::new_v4(),
            player_id: Uuid::new_v4(),
            source_village_id: 0,
            target_village_id: 2,
            units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            hero_id: None,
            arrives_at: Utc::now(),
        };

        let events = command.handle(&aggregate).await.unwrap();
        assert!(matches!(
            events.first(),
            Some(VillageEvent::ReinforcementArrived { .. })
        ));
    }
}
