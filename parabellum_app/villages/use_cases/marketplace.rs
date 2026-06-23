//! Marketplace and merchant transfer use cases.
//!
//! This service owns app-level marketplace orchestration: it loads current
//! village/offer context, applies app policies, plans merchant travel times
//! through domain map/tribe helpers, and delegates command/workflow execution
//! through app ports.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parabellum_types::errors::{ApplicationError, DbError, GameError};
use uuid::Uuid;

use crate::{
    villages::read_models::MarketplaceData,
    villages::{
        CreateMarketplaceOffer, MarketplaceAcceptance, MarketplaceOfferCreation,
        SendMerchantsTransfer,
        models::{
            MarketplaceOfferModel, MarketplaceOfferSnapshot, MarketplaceOfferStatus, VillageModel,
        },
        ports::{Clock, MarketplaceCommandExecutor, MarketplaceCommandIntent, MarketplaceReadPort},
        requests::marketplace::{
            AcceptMarketplaceOfferRequest, CancelMarketplaceOfferRequest,
            CreateMarketplaceOfferRequest, GetMarketplaceDataRequest, GetMarketplaceOfferRequest,
            SendResourcesRequest,
        },
    },
};

/// Runtime settings needed to plan marketplace merchant travel.
#[derive(Debug, Clone, Copy)]
pub struct MarketplaceSettings {
    /// Square world size used by map distance calculations.
    pub world_size: i32,
    /// Server speed multiplier used by merchant movement and commands.
    pub server_speed: i8,
}

/// Application service for direct merchant transfers and marketplace offers.
#[derive(Clone)]
pub struct MarketplaceUseCases {
    reads: Arc<dyn MarketplaceReadPort>,
    executor: Arc<dyn MarketplaceCommandExecutor>,
    clock: Arc<dyn Clock>,
    settings: MarketplaceSettings,
}

impl MarketplaceUseCases {
    pub fn new(
        reads: Arc<dyn MarketplaceReadPort>,
        executor: Arc<dyn MarketplaceCommandExecutor>,
        clock: Arc<dyn Clock>,
        settings: MarketplaceSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            settings,
        }
    }

    pub async fn get_marketplace_offer(
        &self,
        request: GetMarketplaceOfferRequest,
    ) -> Result<MarketplaceOfferModel, ApplicationError> {
        self.reads.get_marketplace_offer(request.offer_id).await
    }

    pub async fn get_marketplace_data(
        &self,
        request: GetMarketplaceDataRequest,
    ) -> Result<MarketplaceData, ApplicationError> {
        self.reads.get_marketplace_data(request.village_id).await
    }

    pub async fn send_resources(
        &self,
        request: SendResourcesRequest,
    ) -> Result<(), ApplicationError> {
        let source = self
            .reads
            .get_marketplace_village(request.source_village_id)
            .await?;
        let target = self
            .reads
            .get_marketplace_village(request.target_village_id)
            .await
            .map_err(|err| match err {
                ApplicationError::Db(DbError::VillageNotFound(_)) => {
                    ApplicationError::Game(GameError::InvalidValley(request.target_village_id))
                }
                other => other,
            })?;
        self.ensure_village_owner(&source, request.player_id)?;

        let arrives_at = self.merchant_arrival_at(&source, &target);
        self.executor
            .execute_marketplace_command(MarketplaceCommandIntent::SendResources {
                source_village_id: request.source_village_id,
                command: SendMerchantsTransfer {
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    resources: request.resources,
                    arrives_at,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    pub async fn create_marketplace_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        MarketplaceOfferCreation {
            offer_resources: request.offer_resources,
            seek_resources: request.seek_resources,
        }
        .validate()
        .map_err(ApplicationError::Game)?;

        self.executor
            .execute_marketplace_command(MarketplaceCommandIntent::CreateOffer {
                village_id: request.village_id,
                command: CreateMarketplaceOffer {
                    player_id: request.player_id,
                    offer_resources: request.offer_resources,
                    seek_resources: request.seek_resources,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        let offer = self.reads.get_marketplace_offer(request.offer_id).await?;
        if offer.status != MarketplaceOfferStatus::Open {
            return Err(ApplicationError::Game(
                GameError::MarketplaceOfferNoLongerValid,
            ));
        }
        let offer_snapshot = offer_snapshot(&offer);
        MarketplaceAcceptance {
            accepting_player_id: request.player_id,
            accepting_village_id: request.village_id,
            offer: &offer_snapshot,
        }
        .validate()
        .map_err(ApplicationError::Game)?;

        let owner = self
            .reads
            .get_marketplace_village(offer.owner_village_id)
            .await?;
        let accepting = self
            .reads
            .get_marketplace_village(request.village_id)
            .await?;
        self.ensure_village_owner(&accepting, request.player_id)?;

        let owner_arrives_at = self.merchant_arrival_at(&owner, &accepting);
        let accepting_arrives_at = self.merchant_arrival_at(&accepting, &owner);
        self.executor
            .execute_marketplace_command(MarketplaceCommandIntent::AcceptOffer {
                accepting_village_id: request.village_id,
                accepting_player_id: request.player_id,
                offer_id: request.offer_id,
                owner_arrives_at,
                accepting_arrives_at,
            })
            .await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        let offer = self.reads.get_marketplace_offer(request.offer_id).await?;
        if offer.status != MarketplaceOfferStatus::Open
            || offer.owner_village_id != request.village_id
            || offer.owner_player_id != request.player_id
        {
            return Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer));
        }

        self.executor
            .execute_marketplace_command(MarketplaceCommandIntent::CancelOffer {
                village_id: request.village_id,
                player_id: request.player_id,
                offer_id: request.offer_id,
            })
            .await
    }

    fn ensure_village_owner(
        &self,
        village: &VillageModel,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        if village.player_id != player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: village.village_id,
                player_id,
            }));
        }
        Ok(())
    }

    fn merchant_arrival_at(&self, source: &VillageModel, target: &VillageModel) -> DateTime<Utc> {
        self.clock.now() + self.merchant_travel_duration(source, target)
    }

    fn merchant_travel_duration(&self, source: &VillageModel, target: &VillageModel) -> Duration {
        let secs = source.position.calculate_travel_time_secs(
            target.position.clone(),
            source.tribe.merchant_stats().speed,
            self.settings.world_size,
            self.settings.server_speed as u8,
        );
        Duration::seconds(std::cmp::max(1, secs) as i64)
    }
}

fn offer_snapshot(offer: &MarketplaceOfferModel) -> MarketplaceOfferSnapshot {
    MarketplaceOfferSnapshot {
        offer_id: offer.offer_id,
        owner_player_id: offer.owner_player_id,
        owner_village_id: offer.owner_village_id,
        offer_resources: offer.offer_resources,
        seek_resources: offer.seek_resources,
        merchants_reserved: offer.merchants_reserved,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::{Duration, TimeZone, Utc};
    use parabellum_game::models::{
        trapper::TrapperState,
        village::{
            AcademyResearch, ProductionBonus, VillageEffectiveProduction, VillageProduction,
            VillageStocks,
        },
    };
    use parabellum_types::{
        common::{ResourceGroup, ResourceKind, ResourceQuantity},
        errors::{ApplicationError, DbError, GameError},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::read_models::MarketplaceData;
    use crate::villages::{
        models::{MarketplaceOfferModel, MarketplaceOfferStatus, VillageModel},
        ports::{Clock, MarketplaceCommandExecutor, MarketplaceCommandIntent, MarketplaceReadPort},
        requests::marketplace::{
            AcceptMarketplaceOfferRequest, CancelMarketplaceOfferRequest,
            CreateMarketplaceOfferRequest, GetMarketplaceDataRequest, GetMarketplaceOfferRequest,
            SendResourcesRequest,
        },
    };

    use super::{MarketplaceSettings, MarketplaceUseCases};

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    #[derive(Default)]
    struct FakeMarketplaceReads {
        villages: Mutex<HashMap<u32, VillageModel>>,
        offers: Mutex<HashMap<Uuid, MarketplaceOfferModel>>,
        data: Mutex<HashMap<u32, MarketplaceData>>,
    }

    #[async_trait]
    impl MarketplaceReadPort for FakeMarketplaceReads {
        async fn get_marketplace_village(
            &self,
            village_id: u32,
        ) -> Result<VillageModel, ApplicationError> {
            self.villages
                .lock()
                .expect("village lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::VillageNotFound(village_id)))
        }

        async fn get_marketplace_offer(
            &self,
            offer_id: Uuid,
        ) -> Result<MarketplaceOfferModel, ApplicationError> {
            self.offers
                .lock()
                .expect("offer lock should not be poisoned")
                .get(&offer_id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::MarketplaceOfferNotFound(
                    offer_id,
                )))
        }

        async fn get_marketplace_data(
            &self,
            village_id: u32,
        ) -> Result<MarketplaceData, ApplicationError> {
            self.data
                .lock()
                .expect("data lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::VillageNotFound(village_id)))
        }
    }

    #[derive(Default)]
    struct FakeMarketplaceExecutor {
        commands: Mutex<Vec<MarketplaceCommandIntent>>,
    }

    #[async_trait]
    impl MarketplaceCommandExecutor for FakeMarketplaceExecutor {
        async fn execute_marketplace_command(
            &self,
            command: MarketplaceCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push(command);
            Ok(())
        }
    }

    fn village(village_id: u32, player_id: Uuid, position: Position, tribe: Tribe) -> VillageModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        VillageModel {
            village_id,
            player_id,
            village_name: format!("village-{village_id}"),
            position,
            tribe,
            buildings: vec![],
            production: VillageProduction {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
                upkeep: 0,
                bonus: ProductionBonus {
                    lumber: 0,
                    clay: 0,
                    iron: 0,
                    crop: 0,
                },
                effective: VillageEffectiveProduction {
                    lumber: 0,
                    clay: 0,
                    iron: 0,
                    crop: 0,
                },
            },
            stocks: VillageStocks {
                warehouse_capacity: 800,
                granary_capacity: 800,
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
            },
            population: 0,
            loyalty: 100,
            loyalty_updated_at: now,
            is_capital: false,
            culture_points_production: 0,
            smithy_upgrades: [0; 8],
            academy_research: AcademyResearch::default(),
            total_merchants: 0,
            busy_merchants: 0,
            trapper: TrapperState {
                active_traps: 0,
                broken_traps: 0,
                queued_traps: 0,
            },
            updated_at: now,
            parent_village_id: None,
        }
    }

    fn offer(
        offer_id: Uuid,
        owner_player_id: Uuid,
        owner_village_id: u32,
        status: MarketplaceOfferStatus,
    ) -> MarketplaceOfferModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        MarketplaceOfferModel {
            offer_id,
            owner_player_id,
            owner_village_id,
            offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
            seek_resources: ResourceQuantity::new(ResourceKind::Clay, 100),
            merchants_reserved: 1,
            status,
            accepted_by_player_id: None,
            accepted_by_village_id: None,
            created_at: now,
            accepted_at: None,
            canceled_at: None,
        }
    }

    fn use_cases(
        reads: Arc<FakeMarketplaceReads>,
        executor: Arc<FakeMarketplaceExecutor>,
    ) -> MarketplaceUseCases {
        MarketplaceUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            MarketplaceSettings {
                world_size: 100,
                server_speed: 1,
            },
        )
    }

    #[tokio::test]
    async fn marketplace_queries_delegate_to_read_port() {
        let player_id = Uuid::new_v4();
        let offer_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        reads.offers.lock().unwrap().insert(
            offer_id,
            offer(offer_id, player_id, 1, MarketplaceOfferStatus::Open),
        );
        reads.data.lock().unwrap().insert(
            1,
            MarketplaceData {
                own_offers: vec![],
                global_offers: vec![],
                outgoing_merchants: vec![],
                incoming_merchants: vec![],
                village_references: HashMap::new(),
            },
        );
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let loaded_offer = use_cases
            .get_marketplace_offer(GetMarketplaceOfferRequest { offer_id })
            .await
            .unwrap();
        let loaded_data = use_cases
            .get_marketplace_data(GetMarketplaceDataRequest { village_id: 1 })
            .await
            .unwrap();

        assert_eq!(loaded_offer.offer_id, offer_id);
        assert!(loaded_data.own_offers.is_empty());
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn send_resources_maps_missing_target_to_invalid_valley_without_executing() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, player_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .send_resources(SendResourcesRequest {
                player_id,
                source_village_id: 1,
                target_village_id: 99,
                resources: ResourceGroup(100, 0, 0, 0),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::InvalidValley(99)))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn send_resources_builds_transfer_command_with_planned_arrival() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        let source = village(1, player_id, Position { x: 0, y: 0 }, Tribe::Roman);
        let target = village(2, Uuid::new_v4(), Position { x: 10, y: 0 }, Tribe::Gaul);
        reads.villages.lock().unwrap().insert(1, source.clone());
        reads.villages.lock().unwrap().insert(2, target.clone());
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .send_resources(SendResourcesRequest {
                player_id,
                source_village_id: 1,
                target_village_id: 2,
                resources: ResourceGroup(100, 0, 0, 0),
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let MarketplaceCommandIntent::SendResources {
            source_village_id,
            command,
        } = commands.first().expect("command should be executed")
        else {
            panic!("expected send resources command");
        };
        let expected_secs = source.position.calculate_travel_time_secs(
            target.position,
            source.tribe.merchant_stats().speed,
            100,
            1,
        );
        assert_eq!(*source_village_id, 1);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.target_village_id, 2);
        assert_eq!(
            command.arrives_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
                + Duration::seconds(std::cmp::max(1, expected_secs) as i64)
        );
    }

    #[tokio::test]
    async fn create_offer_rejects_invalid_terms_without_executing() {
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(Arc::new(FakeMarketplaceReads::default()), executor.clone());

        let result = use_cases
            .create_marketplace_offer(CreateMarketplaceOfferRequest {
                player_id: Uuid::new_v4(),
                village_id: 1,
                offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
                seek_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn accept_offer_rejects_closed_offer_without_executing() {
        let owner_player_id = Uuid::new_v4();
        let offer_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        reads.offers.lock().unwrap().insert(
            offer_id,
            offer(
                offer_id,
                owner_player_id,
                1,
                MarketplaceOfferStatus::Accepted,
            ),
        );
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .accept_marketplace_offer(AcceptMarketplaceOfferRequest {
                player_id: Uuid::new_v4(),
                village_id: 2,
                offer_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(
                GameError::MarketplaceOfferNoLongerValid
            ))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn accept_offer_builds_claim_command_with_both_arrivals() {
        let owner_player_id = Uuid::new_v4();
        let accepting_player_id = Uuid::new_v4();
        let offer_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        let owner = village(1, owner_player_id, Position { x: 0, y: 0 }, Tribe::Teuton);
        let accepting = village(
            2,
            accepting_player_id,
            Position { x: 12, y: 0 },
            Tribe::Roman,
        );
        reads.villages.lock().unwrap().insert(1, owner.clone());
        reads.villages.lock().unwrap().insert(2, accepting.clone());
        reads.offers.lock().unwrap().insert(
            offer_id,
            offer(offer_id, owner_player_id, 1, MarketplaceOfferStatus::Open),
        );
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .accept_marketplace_offer(AcceptMarketplaceOfferRequest {
                player_id: accepting_player_id,
                village_id: 2,
                offer_id,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let MarketplaceCommandIntent::AcceptOffer {
            accepting_village_id,
            accepting_player_id: command_accepting_player_id,
            offer_id: command_offer_id,
            owner_arrives_at,
            accepting_arrives_at,
        } = commands.first().expect("command should be executed")
        else {
            panic!("expected accept offer command");
        };
        assert_eq!(*accepting_village_id, 2);
        assert_eq!(*command_accepting_player_id, accepting_player_id);
        assert_eq!(*command_offer_id, offer_id);
        assert!(*owner_arrives_at > Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap());
        assert!(*accepting_arrives_at > Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap());
    }

    #[tokio::test]
    async fn cancel_offer_rejects_non_owner_without_executing() {
        let owner_player_id = Uuid::new_v4();
        let offer_id = Uuid::new_v4();
        let reads = Arc::new(FakeMarketplaceReads::default());
        reads.offers.lock().unwrap().insert(
            offer_id,
            offer(offer_id, owner_player_id, 1, MarketplaceOfferStatus::Open),
        );
        let executor = Arc::new(FakeMarketplaceExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .cancel_marketplace_offer(CancelMarketplaceOfferRequest {
                player_id: Uuid::new_v4(),
                village_id: 1,
                offer_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::InvalidMarketplaceOffer))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }
}
