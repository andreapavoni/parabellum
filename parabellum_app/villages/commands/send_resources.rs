use chrono::{DateTime, Utc};
use mini_cqrs_es::{Aggregate, Command, CqrsError};
use parabellum_types::{common::ResourceGroup, errors::GameError};
use uuid::Uuid;

use crate::villages::{VillageAggregate, VillageEvent, commands::as_domain_error};

#[derive(Debug, Clone)]
/// Schedules a merchant-based resource transfer from source village to target village.
pub struct SendMerchantsTransfer {
    pub player_id: Uuid,
    pub target_village_id: u32,
    pub resources: ResourceGroup,
    pub arrives_at: DateTime<Utc>,
}

impl Command for SendMerchantsTransfer {
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
        if self.resources.total() == 0 {
            return Err(as_domain_error(GameError::NotEnoughResources));
        }

        let merchants_used = aggregate
            .village()
            .schedule_send_resources(self.resources.clone())
            .map_err(as_domain_error)?;

        let travel_duration = (self.arrives_at - Utc::now()).max(chrono::Duration::seconds(1));
        let returns_at = self.arrives_at + travel_duration;
        let arrival_action_id = Uuid::new_v4();
        let return_action_id = Uuid::new_v4();

        Ok(vec![VillageEvent::MerchantsTripScheduled {
            arrival_action_id,
            return_action_id,
            player_id: self.player_id,
            source_village_id,
            target_village_id: self.target_village_id,
            resources: self.resources.clone(),
            merchants_used,
            resources_already_reserved: false,
            arrives_at: self.arrives_at,
            returns_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use mini_cqrs_es::{Aggregate, Command};
    use parabellum_game::models::{buildings::Building, village::VillageBuilding};
    use parabellum_types::{
        buildings::{BuildingGroup, BuildingName},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{SendMerchantsTransfer, VillageAggregate, VillageEvent};

    fn village_with_marketplace(level: u8) -> Vec<VillageBuilding> {
        vec![
            VillageBuilding {
                slot_id: 19,
                building: Building {
                    name: BuildingName::MainBuilding,
                    group: BuildingGroup::Infrastructure,
                    value: 0,
                    population: 0,
                    culture_points: 0,
                    level: 1,
                },
            },
            VillageBuilding {
                slot_id: 27,
                building: Building {
                    name: BuildingName::Marketplace,
                    group: BuildingGroup::Infrastructure,
                    value: 0,
                    population: 0,
                    culture_points: 0,
                    level,
                },
            },
        ]
    }

    async fn aggregate_ready(level: u8) -> VillageAggregate {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
                parent_village_id: None,
                buildings: village_with_marketplace(level),
            })
            .await;
        aggregate.set_resources_for_test(parabellum_types::common::ResourceGroup(
            2_000, 2_000, 2_000, 2_000,
        ));
        aggregate
    }

    #[tokio::test]
    async fn rejects_without_marketplace() {
        let mut aggregate = VillageAggregate::default();
        let player_id = Uuid::new_v4();
        aggregate
            .apply(&VillageEvent::VillageFounded {
                village_id: 1,
                village_name: "v1".to_string(),
                position: Position { x: 0, y: 0 },
                tribe: Tribe::Roman,
                player_id,
                parent_village_id: None,
                buildings: village_with_marketplace(0),
            })
            .await;
        aggregate.set_resources_for_test(parabellum_types::common::ResourceGroup(
            2_000, 2_000, 2_000, 2_000,
        ));

        let result = SendMerchantsTransfer {
            player_id,
            target_village_id: 2,
            resources: parabellum_types::common::ResourceGroup(200, 50, 120, 100),
            arrives_at: Utc::now() + Duration::minutes(10),
        }
        .handle(&aggregate)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn schedules_resources_sending() {
        let aggregate = aggregate_ready(1).await;
        let player_id = aggregate.player_id();
        let result = SendMerchantsTransfer {
            player_id,
            target_village_id: 2,
            resources: parabellum_types::common::ResourceGroup(200, 50, 120, 100),
            arrives_at: Utc::now() + Duration::minutes(10),
        }
        .handle(&aggregate)
        .await
        .unwrap();
        assert!(matches!(
            result.first(),
            Some(VillageEvent::MerchantsTripScheduled {
                merchants_used: 1,
                ..
            })
        ));
    }
}
