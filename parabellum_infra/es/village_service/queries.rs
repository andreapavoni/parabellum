//! Query/read helpers for `VillageEsService`.
//!
//! These methods are side-effect free with respect to aggregate mutations:
//! they compose read models, CQRS query projections, and derived views for API use.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use mini_cqrs_es::{CqrsError, QueryRunner};
use parabellum_app::read_models::VillageReference;
use parabellum_app::villages::VillageService;
use parabellum_app::villages::cqrs_queries::{
    CountUnreadReportsForPlayer, GetMarketplaceOfferById, GetOpenMarketplaceOffers,
    GetReportForPlayer, ListReportsForPlayer, ScheduledActionStatusCounts,
};
use parabellum_app::villages::models::{
    BuildingWorkflow, BuildingWorkflowKind, ReportModel, ScheduledAction, ScheduledActionPayload,
    ScheduledActionStatus, ScheduledActionType, VillageModel,
};
use parabellum_app::villages::projection_repositories::{
    ArmyRepository, HeroRepository, MarketplaceRepository, ReportRepository,
    ScheduledActionRepository, VillageMovementRepository, VillageRepository,
};
use parabellum_app::villages::read_models::{
    MarketplaceData, TroopMovement, TroopMovementDirection, VillageArmyStateView, VillageQueues,
    VillageTroopMovements,
};
use parabellum_game::models::buildings::Building;
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::{DbError, GameError};

use crate::es::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresReportRepository, PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository, village_cqrs_runtime,
};

use super::{
    CancelBuildingConstructionContext, CancelTroopMovementContext, ReinforcementContext,
    TrappedArmyContext, VillageEsService,
};

#[derive(Clone)]
struct CancelableBuildingAction {
    id: uuid::Uuid,
    status: ScheduledActionStatus,
    execute_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    workflow: BuildingWorkflow,
}

fn decode_building_actions(
    actions: Vec<ScheduledAction>,
) -> Result<Vec<CancelableBuildingAction>, CqrsError> {
    actions
        .into_iter()
        .map(|action| {
            let payload: ScheduledActionPayload =
                serde_json::from_value(action.payload).map_err(CqrsError::Serialization)?;
            let ScheduledActionPayload::Building { workflow } = payload else {
                return Err(CqrsError::EventStore(
                    "Scheduled action is not a building workflow".to_string(),
                ));
            };
            Ok(CancelableBuildingAction {
                id: action.id,
                status: action.status,
                execute_at: action.execute_at,
                created_at: action.created_at.unwrap_or_else(chrono::Utc::now),
                workflow,
            })
        })
        .collect()
}

fn prorated_building_refund(
    workflow: &BuildingWorkflow,
    started_at: chrono::DateTime<chrono::Utc>,
    execute_at: chrono::DateTime<chrono::Utc>,
    canceled_at: chrono::DateTime<chrono::Utc>,
) -> Result<ResourceGroup, GameError> {
    let cost = match workflow.kind {
        BuildingWorkflowKind::Add | BuildingWorkflowKind::Upgrade => {
            Building::new(workflow.building_name.clone(), workflow.speed)
                .at_level(workflow.level, workflow.speed)?
                .cost()
                .resources
        }
        BuildingWorkflowKind::Downgrade => ResourceGroup::new(0, 0, 0, 0),
    };
    if cost.total() == 0 {
        return Ok(cost);
    }

    let total_secs = (execute_at - started_at).num_seconds().max(0) as u64;
    if total_secs == 0 {
        return Ok(ResourceGroup::new(0, 0, 0, 0));
    }

    let elapsed_secs = (canceled_at - started_at)
        .num_seconds()
        .clamp(0, total_secs as i64) as u64;
    let remaining_secs = total_secs.saturating_sub(elapsed_secs);

    Ok(ResourceGroup::new(
        prorate_resource(cost.lumber(), remaining_secs, total_secs),
        prorate_resource(cost.clay(), remaining_secs, total_secs),
        prorate_resource(cost.iron(), remaining_secs, total_secs),
        prorate_resource(cost.crop(), remaining_secs, total_secs),
    ))
}

fn prorate_resource(value: u32, remaining_secs: u64, total_secs: u64) -> u32 {
    ((value as u64 * remaining_secs) / total_secs) as u32
}

fn add_resources(left: ResourceGroup, right: ResourceGroup) -> ResourceGroup {
    ResourceGroup::new(
        left.lumber().saturating_add(right.lumber()),
        left.clay().saturating_add(right.clay()),
        left.iron().saturating_add(right.iron()),
        left.crop().saturating_add(right.crop()),
    )
}

impl VillageEsService {
    pub async fn find_cancel_building_construction_context(
        &self,
        village_id: u32,
        action_id: uuid::Uuid,
        canceled_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<CancelBuildingConstructionContext, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let mut actions = Vec::new();
        for action_type in [
            ScheduledActionType::AddBuilding,
            ScheduledActionType::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding,
        ] {
            actions.extend(
                repo.list_active_by_village_and_type(village_id, action_type)
                    .await
                    .map_err(CqrsError::domain_source)?,
            );
        }

        let mut building_actions = decode_building_actions(actions)?;
        let Some(anchor) = building_actions
            .iter()
            .find(|action| action.id == action_id)
            .cloned()
        else {
            return Err(CqrsError::EventStore(
                DbError::JobNotFound(action_id).to_string(),
            ));
        };
        if anchor.status != ScheduledActionStatus::Pending
            || anchor.workflow.village_id != village_id
        {
            return Err(CqrsError::domain_source(
                GameError::BuildingConstructionNotCancelable,
            ));
        }

        building_actions.sort_by_key(|action| action.execute_at);
        let mut previous_execute_at = None;
        let mut action_ids = Vec::new();
        let mut refund = ResourceGroup::new(0, 0, 0, 0);

        for action in building_actions
            .into_iter()
            .filter(|action| action.workflow.slot_id == anchor.workflow.slot_id)
        {
            let started_at = previous_execute_at.unwrap_or(action.created_at);
            previous_execute_at = Some(action.execute_at);

            if action.execute_at < anchor.execute_at {
                continue;
            }
            if action.status != ScheduledActionStatus::Pending {
                return Err(CqrsError::domain_source(
                    GameError::BuildingConstructionNotCancelable,
                ));
            }

            action_ids.push(action.id);
            refund = add_resources(
                refund,
                prorated_building_refund(
                    &action.workflow,
                    started_at,
                    action.execute_at,
                    canceled_at,
                )
                .map_err(CqrsError::domain_source)?,
            );
        }

        if action_ids.is_empty() {
            return Err(CqrsError::domain_source(
                GameError::BuildingConstructionNotCancelable,
            ));
        }

        Ok(CancelBuildingConstructionContext {
            action_ids,
            player_id: anchor.workflow.player_id,
            village_id: anchor.workflow.village_id,
            execute_at: anchor.execute_at,
            refund,
        })
    }

    pub async fn list_cancelable_outgoing_movement_ids(
        &self,
        source_village_id: u32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<HashSet<uuid::Uuid>, CqrsError> {
        let rows = PostgresScheduledActionRepository::new(self.pool.clone())
            .list_pending_troop_arrivals_by_source_village(source_village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let mut movement_ids = HashSet::new();
        for row in rows {
            let cancel_deadline = row.created_at + chrono::Duration::seconds(60);
            if now > cancel_deadline || now >= row.execute_at {
                continue;
            }
            let payload: ScheduledActionPayload =
                serde_json::from_value(row.payload).map_err(CqrsError::Serialization)?;
            let movement_id = match payload {
                ScheduledActionPayload::AttackArrival { workflow } => workflow.movement_id,
                ScheduledActionPayload::ScoutArrival { workflow } => workflow.movement_id,
                ScheduledActionPayload::ReinforcementArrival { workflow } => workflow.movement_id,
                ScheduledActionPayload::SettlersArrival { workflow } => workflow.movement_id,
                _ => continue,
            };
            movement_ids.insert(movement_id);
        }
        Ok(movement_ids)
    }

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, CqrsError> {
        let repo = PostgresVillageMovementRepository::new(self.pool.clone());
        let movements = repo
            .list_by_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let fallback_village_ids = movements
            .iter()
            .flat_map(|m| [m.origin_village_id, m.target_village_id])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let village_repo = PostgresVillageRepository::new(self.pool.clone());
        let fallback_villages = village_repo
            .list_by_village_ids(&fallback_village_ids)
            .await
            .map_err(CqrsError::domain_source)?
            .into_iter()
            .map(|v| (v.village_id, v))
            .collect::<HashMap<_, _>>();

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for movement in movements {
            let origin_model = fallback_villages.get(&movement.origin_village_id);
            let target_model = fallback_villages.get(&movement.target_village_id);

            let mapped = TroopMovement {
                job_id: movement.movement_id,
                movement_type: Self::map_troop_movement_type(movement.movement_type),
                direction: match movement.direction {
                    parabellum_app::villages::models::MovementDirection::Incoming => {
                        TroopMovementDirection::Incoming
                    }
                    parabellum_app::villages::models::MovementDirection::Outgoing => {
                        TroopMovementDirection::Outgoing
                    }
                },
                origin_village_id: movement.origin_village_id,
                origin_village_name: movement
                    .origin_village_name
                    .or_else(|| origin_model.map(|v| v.village_name.clone())),
                origin_player_id: movement.origin_player_id,
                origin_position: movement
                    .origin_position
                    .or_else(|| origin_model.map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                target_village_id: movement.target_village_id,
                target_village_name: movement
                    .target_village_name
                    .or_else(|| target_model.map(|v| v.village_name.clone())),
                target_player_id: movement
                    .target_player_id
                    .or_else(|| target_model.map(|v| v.player_id))
                    .unwrap_or(movement.origin_player_id),
                target_position: movement
                    .target_position
                    .or_else(|| target_model.map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                arrives_at: movement.arrives_at,
                time_seconds: movement.time_seconds.unwrap_or(0),
                units: movement.units,
                has_hero: movement.has_hero,
                tribe: movement
                    .tribe
                    .or_else(|| {
                        if matches!(
                            movement.movement_type,
                            parabellum_app::villages::models::MovementType::Return
                        ) {
                            target_model.map(|v| v.tribe.clone())
                        } else {
                            origin_model.map(|v| v.tribe.clone())
                        }
                    })
                    .unwrap_or(parabellum_types::tribe::Tribe::Nature),
                bounty: movement.bounty,
            };
            match mapped.direction {
                TroopMovementDirection::Outgoing => outgoing.push(mapped),
                TroopMovementDirection::Incoming => incoming.push(mapped),
            };
        }
        outgoing.sort_by_key(|m| m.arrives_at);
        incoming.sort_by_key(|m| m.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    pub async fn get_village(&self, village_id: u32) -> Result<VillageModel, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.get_by_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn get_hero(
        &self,
        hero_id: uuid::Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, CqrsError> {
        let repo = PostgresHeroRepository::new(self.pool.clone());
        repo.get_by_id(hero_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn get_hero_by_player(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, CqrsError> {
        let repo = PostgresHeroRepository::new(self.pool.clone());
        repo.get_by_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn player_has_alive_hero(&self, player_id: uuid::Uuid) -> Result<bool, CqrsError> {
        let repo: Arc<dyn HeroRepository> =
            Arc::new(PostgresHeroRepository::new(self.pool.clone()));
        repo.has_alive_for_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn player_has_pending_hero_revival(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<bool, CqrsError> {
        PostgresScheduledActionRepository::new(self.pool.clone())
            .has_pending_hero_revival_for_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn pending_hero_revival_at(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, CqrsError> {
        PostgresScheduledActionRepository::new(self.pool.clone())
            .pending_hero_revival_at_for_player(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn list_player_village_states(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.list_by_player_id(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn count_child_villages(
        &self,
        player_id: uuid::Uuid,
        parent_village_id: u32,
    ) -> Result<u8, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.count_child_villages(player_id, parent_village_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn find_reinforcement_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<ReinforcementContext, CqrsError> {
        let army_repo: Arc<dyn ArmyRepository> =
            Arc::new(PostgresArmyRepository::new(self.pool.clone()));
        if let Some((stationed_village_id, army)) = army_repo
            .find_stationed_context_by_army_id(army_id)
            .await
            .map_err(CqrsError::domain_source)?
        {
            return Ok(ReinforcementContext {
                stationed_village_id,
                home_village_id: army.village_id,
                army,
            });
        }

        Err(CqrsError::EventStore(
            DbError::ArmyNotFound(army_id).to_string(),
        ))
    }

    pub async fn find_trapped_army_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<TrappedArmyContext, CqrsError> {
        let army_repo: Arc<dyn ArmyRepository> =
            Arc::new(PostgresArmyRepository::new(self.pool.clone()));
        if let Some((trapped_village_id, army)) = army_repo
            .find_trapped_context_by_army_id(army_id)
            .await
            .map_err(CqrsError::domain_source)?
        {
            return Ok(TrappedArmyContext {
                trapped_village_id,
                home_village_id: army.village_id,
                army,
            });
        }

        Err(CqrsError::EventStore(
            DbError::ArmyNotFound(army_id).to_string(),
        ))
    }

    pub async fn find_cancel_troop_movement_context(
        &self,
        movement_id: uuid::Uuid,
    ) -> Result<CancelTroopMovementContext, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let Some(action) = repo
            .find_pending_troop_arrival_by_movement_id(movement_id)
            .await
            .map_err(CqrsError::domain_source)?
        else {
            return Err(CqrsError::EventStore(
                DbError::JobNotFound(movement_id).to_string(),
            ));
        };

        let payload: ScheduledActionPayload =
            serde_json::from_value(action.payload).map_err(CqrsError::Serialization)?;
        match payload {
            ScheduledActionPayload::AttackArrival { workflow } => Ok(CancelTroopMovementContext {
                movement_id,
                arrival_action_id: action.id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army,
                sent_at: action.created_at,
                arrives_at: workflow.arrives_at,
            }),
            ScheduledActionPayload::ScoutArrival { workflow } => Ok(CancelTroopMovementContext {
                movement_id,
                arrival_action_id: action.id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                army: workflow.army,
                sent_at: action.created_at,
                arrives_at: workflow.arrives_at,
            }),
            ScheduledActionPayload::ReinforcementArrival { workflow } => {
                Ok(CancelTroopMovementContext {
                    movement_id,
                    arrival_action_id: action.id,
                    army_id: workflow.army_id,
                    player_id: workflow.player_id,
                    source_village_id: workflow.source_village_id,
                    target_village_id: workflow.target_village_id,
                    army: workflow.army,
                    sent_at: action.created_at,
                    arrives_at: workflow.arrives_at,
                })
            }
            ScheduledActionPayload::SettlersArrival { workflow } => {
                let army_repo: Arc<dyn ArmyRepository> =
                    Arc::new(PostgresArmyRepository::new(self.pool.clone()));
                let army = army_repo
                    .get_moving_army(workflow.army_id)
                    .await
                    .map_err(CqrsError::domain_source)?;
                Ok(CancelTroopMovementContext {
                    movement_id,
                    arrival_action_id: action.id,
                    army_id: workflow.army_id,
                    player_id: workflow.player_id,
                    source_village_id: workflow.source_village_id,
                    target_village_id: workflow.target_village_id,
                    army,
                    sent_at: action.created_at,
                    arrives_at: workflow.arrives_at,
                })
            }
            _ => Err(CqrsError::EventStore(
                "Scheduled action is not a troop arrival workflow".to_string(),
            )),
        }
    }

    pub async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, CqrsError> {
        PostgresScheduledActionRepository::new(self.pool.clone())
            .list_village_queues(village_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn get_village_scheduled_action_status_counts(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.count_by_village_and_type(village_id, action_type, status_filter)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn get_open_marketplace_offers(
        &self,
    ) -> Result<Vec<parabellum_app::villages::models::MarketplaceOfferModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetOpenMarketplaceOffers {
                repository: Arc::new(PostgresMarketplaceRepository::new(self.pool.clone())),
            })
            .await
    }

    pub async fn get_marketplace_offer(
        &self,
        offer_id: uuid::Uuid,
    ) -> Result<parabellum_app::villages::models::MarketplaceOfferModel, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetMarketplaceOfferById {
                repository: Arc::new(PostgresMarketplaceRepository::new(self.pool.clone())),
                offer_id,
            })
            .await
    }

    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, CqrsError> {
        let marketplace_repo = PostgresMarketplaceRepository::new(self.pool.clone());

        let to_offer =
            |m: parabellum_app::villages::models::MarketplaceOfferModel| MarketplaceOffer {
                id: m.offer_id,
                player_id: m.owner_player_id,
                village_id: m.owner_village_id,
                offer_resources: m.offer_resources,
                seek_resources: m.seek_resources,
                merchants_required: m.merchants_reserved,
                created_at: m.created_at,
            };

        let own_open_models = marketplace_repo
            .list_open_by_owner_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let global_open_models = marketplace_repo
            .list_open_excluding_owner_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let outgoing_merchants = marketplace_repo
            .list_active_outgoing(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        let incoming_merchants = marketplace_repo
            .list_active_incoming(village_id)
            .await
            .map_err(CqrsError::domain_source)?;

        let village_ids = own_open_models
            .iter()
            .chain(global_open_models.iter())
            .map(|m| m.owner_village_id)
            .chain(
                outgoing_merchants
                    .iter()
                    .flat_map(|m| [m.origin_village_id, m.destination_village_id]),
            )
            .chain(
                incoming_merchants
                    .iter()
                    .flat_map(|m| [m.origin_village_id, m.destination_village_id]),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let village_references = self.village_references(village_ids).await?;

        Ok(MarketplaceData {
            own_offers: own_open_models.iter().cloned().map(to_offer).collect(),
            global_offers: global_open_models.into_iter().map(to_offer).collect(),
            outgoing_merchants,
            incoming_merchants,
            village_references,
        })
    }

    pub async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, CqrsError> {
        // Read-model ownership contract:
        // - rm_armies is canonical for army state queries
        // - rm_village troop fields are not used as query authority
        let repo = PostgresArmyRepository::new(self.pool.clone());
        let armies = repo
            .army_context_for_village(village_id)
            .await
            .map_err(CqrsError::domain_source)?;
        Ok(VillageArmyStateView {
            home_army: armies.home,
            reinforcements: armies.stationed,
            deployed_armies: armies.deployed,
            trapped_here: armies.trapped_here,
            trapped_away: armies.trapped_away,
        })
    }

    async fn village_references(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageReference>, CqrsError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let repo = PostgresVillageRepository::new(self.pool.clone());
        let villages = repo
            .list_by_village_ids(&village_ids)
            .await
            .map_err(CqrsError::domain_source)?;

        Ok(villages
            .into_iter()
            .map(|v| {
                (
                    v.village_id,
                    VillageReference {
                        id: v.village_id,
                        name: v.village_name,
                        position: v.position,
                    },
                )
            })
            .collect())
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: uuid::Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&ListReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                player_id,
                offset,
                limit,
            })
            .await
    }

    pub async fn get_report_for_player(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<Option<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetReportForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                report_id,
                player_id,
            })
            .await
    }

    pub async fn count_unread_reports_for_player(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<i64, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&CountUnreadReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                player_id,
            })
            .await
    }

    pub async fn mark_report_as_read(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let report = self
            .get_report_for_player(report_id, player_id)
            .await?
            .ok_or_else(|| CqrsError::EventStore("report not found for player".to_string()))?;
        let village_id = report
            .actor_village_id
            .or(report.target_village_id)
            .ok_or_else(|| {
                CqrsError::EventStore("report has no village stream anchor".to_string())
            })?;

        let cqrs = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&cqrs);
        service
            .mark_report_read(
                village_id,
                &parabellum_app::villages::MarkReportRead {
                    report_id,
                    player_id,
                    read_at: chrono::Utc::now(),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn get_village_scheduled_action_status_count(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status: ScheduledActionStatus,
    ) -> Result<usize, CqrsError> {
        let counts = self
            .get_village_scheduled_action_status_counts(village_id, action_type, Some(status))
            .await?;
        Ok(match status {
            ScheduledActionStatus::Pending => counts.pending,
            ScheduledActionStatus::Processing => counts.processing,
            ScheduledActionStatus::Completed => counts.completed,
            ScheduledActionStatus::Failed => counts.failed,
            ScheduledActionStatus::Canceled => counts.canceled,
        })
    }
}
