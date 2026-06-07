//! Village ES orchestration service.
//!
//! This module is intentionally split by concern:
//! - `mod.rs`: command dispatch + scheduler tick orchestration
//! - `queries.rs`: read/query helpers consumed by adapters/web layer
//! - `scheduler.rs`: deterministic fact-driven scheduled workflow progression
//!
//! Public API remains centered on `VillageEsService`.

use mini_cqrs_es::anyhow::Result;
use mini_cqrs_es::{
    Aggregate, AggregateSnapshot, CqrsError, EventMetadata, EventPayload, EventStore, NewEvent,
    SnapshotStore,
};
use parabellum_game::models::army::Army;
use parabellum_game::models::village::Village;
use sqlx::PgPool;
use tracing::{info, warn};

use parabellum_app::ports::map::MapRepository;
use parabellum_app::ports::queries::TroopMovementType;
use parabellum_app::villages::models::{
    MarketplaceOfferSnapshot, MarketplaceOfferStatus, ScheduledAction, ScheduledActionStatus,
};
use parabellum_app::villages::repositories::{MarketplaceRepository, ScheduledActionRepository};
use parabellum_app::villages::{
    AddBuilding, ApplyBattleOutcomeToVillage, AttackVillage, CancelMarketplaceOffer, CreateHero,
    CreateMarketplaceOffer, DowngradeBuilding, FoundVillage, MarketplaceAcceptance,
    RecallReinforcements, ReleaseReinforcements, RenameVillage, ResearchAcademy, ResearchSmithy,
    ResolveAttackBattle, ReviveHero, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
    SendSettlers, SetVillageResources, TrainUnits, UpgradeBuilding, VillageService,
};
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::es::advisory_lock::AdvisoryLock;
use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::workflows;
use crate::es::{
    PostgresEventStore, PostgresMarketplaceRepository, PostgresScheduledActionRepository,
    PostgresSnapshotStore, ReportProjector, VillageProjector, WorkflowStreamAppend,
    village_cqrs_runtime,
};
use crate::map::PostgresMapRepository;

mod queries;
mod scheduler;

const SCHEDULED_ACTION_PROCESSING_STALE_AFTER_SECS: i64 = 120;

#[derive(Debug, Clone)]
/// ES orchestration facade for village command, scheduler, and read helper flows.
pub struct VillageEsService {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ReinforcementContext {
    /// Village where the reinforcement is currently stationed.
    pub stationed_village_id: u32,
    /// Home/origin village of the reinforcement army.
    pub home_village_id: u32,
    /// Full army state for recall/release command construction.
    pub army: Army,
}

pub struct CancelTroopMovementContext {
    pub movement_id: Uuid,
    pub arrival_action_id: Uuid,
    pub army_id: Uuid,
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub army: Army,
    pub sent_at: chrono::DateTime<chrono::Utc>,
    pub arrives_at: chrono::DateTime<chrono::Utc>,
}

impl VillageEsService {
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Converts unordered `(village_id, event)` facts into stream-grouped append
    /// units with expected versions.
    ///
    /// Contract:
    /// - grouping is by aggregate stream id (`village_id`)
    /// - event order is preserved inside each stream group
    /// - expected versions are loaded immediately before append preparation
    ///
    /// This is a local extraction seam for a future generic workflow builder in
    /// `mini_cqrs_es`.
    async fn build_village_workflow_appends(
        &self,
        workflow_events: Vec<(u32, parabellum_app::villages::VillageEvent)>,
    ) -> Result<Vec<WorkflowStreamAppend>, CqrsError> {
        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(self.pool.clone());
        let mut grouped: Vec<(u32, Vec<NewEvent>)> = Vec::new();
        for (aggregate_id, payload) in workflow_events {
            let event = NewEvent {
                event_type: payload.name(),
                payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                metadata: EventMetadata::default(),
                timestamp: chrono::Utc::now(),
            };
            if let Some((_, events)) = grouped.iter_mut().find(|(id, _)| *id == aggregate_id) {
                events.push(event);
            } else {
                grouped.push((aggregate_id, vec![event]));
            }
        }

        let mut streams = Vec::with_capacity(grouped.len());
        for (aggregate_id, events) in grouped {
            let (_, expected_version) = store
                .load_events(aggregate_type, &aggregate_id.to_string())
                .await?;
            streams.push(WorkflowStreamAppend {
                aggregate_id: aggregate_id.to_string(),
                expected_version,
                events,
            });
        }
        Ok(streams)
    }

    fn map_troop_movement_type(
        movement_type: parabellum_app::villages::models::MovementType,
    ) -> TroopMovementType {
        match movement_type {
            parabellum_app::villages::models::MovementType::Attack => TroopMovementType::Attack,
            parabellum_app::villages::models::MovementType::Raid => TroopMovementType::Raid,
            parabellum_app::villages::models::MovementType::Scout => TroopMovementType::Scout,
            parabellum_app::villages::models::MovementType::Reinforcement => {
                TroopMovementType::Reinforcement
            }
            parabellum_app::villages::models::MovementType::Return => TroopMovementType::Return,
            parabellum_app::villages::models::MovementType::FoundVillage => {
                TroopMovementType::FoundVillage
            }
        }
    }

    fn as_offer_snapshot(
        offer: parabellum_app::villages::models::MarketplaceOfferModel,
    ) -> MarketplaceOfferSnapshot {
        MarketplaceOfferSnapshot {
            offer_id: offer.offer_id,
            owner_player_id: offer.owner_player_id,
            owner_village_id: offer.owner_village_id,
            offer_resources: offer.offer_resources,
            seek_resources: offer.seek_resources,
            merchants_reserved: offer.merchants_reserved,
        }
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn materialize_current_resources_for_command(
        &self,
        village_id: u32,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let current = self.get_village(village_id).await?;
        if current.player_id != player_id {
            return Err(CqrsError::domain_source(GameError::VillageNotOwned {
                village_id,
                player_id,
            }));
        }
        let resources = parabellum_types::common::ResourceGroup::new(
            current.stocks.lumber,
            current.stocks.clay,
            current.stocks.iron,
            current.stocks.crop.max(0) as u32,
        );
        self.set_village_resources(
            village_id,
            &SetVillageResources {
                player_id,
                resources,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn found_village(
        &self,
        village_id: u32,
        command: &FoundVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.found_village(village_id, command).await
    }

    pub async fn send_reinforcement(
        &self,
        village_id: u32,
        command: &SendReinforcement,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_reinforcement(village_id, command).await
    }

    pub async fn send_attack(
        &self,
        village_id: u32,
        command: &AttackVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_attack(village_id, command).await
    }

    pub async fn recall_reinforcements(
        &self,
        village_id: u32,
        command: &RecallReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.recall_reinforcements(village_id, command).await
    }

    pub async fn release_reinforcements(
        &self,
        village_id: u32,
        command: &ReleaseReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.release_reinforcements(village_id, command).await
    }

    pub async fn cancel_troop_movement(
        &self,
        village_id: u32,
        command: &parabellum_app::villages::CancelTroopMovement,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.cancel_troop_movement(village_id, command).await
    }

    pub async fn send_scout(
        &self,
        village_id: u32,
        command: &ScoutVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_scout(village_id, command).await
    }

    pub async fn send_settlers(
        &self,
        village_id: u32,
        command: &SendSettlers,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_settlers(village_id, command).await
    }

    pub async fn create_hero(
        &self,
        village_id: u32,
        command: &CreateHero,
    ) -> Result<u32, CqrsError> {
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::domain_source(GameError::HeroAlreadyExists));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_hero(village_id, command).await
    }

    pub async fn revive_hero(
        &self,
        village_id: u32,
        command: &ReviveHero,
    ) -> Result<u32, CqrsError> {
        if self
            .player_has_pending_hero_revival(command.player_id)
            .await?
        {
            return Err(CqrsError::domain_source(
                GameError::HeroRevivalAlreadyPending,
            ));
        }
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::domain_source(GameError::HeroAlreadyExists));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.revive_hero(village_id, command).await
    }

    /// Returns whether a target map field is currently an unoccupied valley.
    pub async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, CqrsError> {
        let map_repo = PostgresMapRepository::new(self.pool.clone());
        map_repo
            .is_unoccupied_valley(field_id as i32)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub async fn add_building(
        &self,
        village_id: u32,
        command: &AddBuilding,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.add_building(village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.upgrade_building(village_id, command).await
    }

    /// Renames a village for its owner.
    pub async fn rename_village(
        &self,
        village_id: u32,
        command: &RenameVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.rename_village(village_id, command).await
    }

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.downgrade_building(village_id, command).await
    }

    pub async fn train_units(
        &self,
        village_id: u32,
        command: &TrainUnits,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.train_units(village_id, command).await
    }

    pub async fn research_academy(
        &self,
        village_id: u32,
        command: &ResearchAcademy,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_academy(village_id, command).await
    }

    pub async fn research_smithy(
        &self,
        village_id: u32,
        command: &ResearchSmithy,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_smithy(village_id, command).await
    }

    pub async fn resolve_attack_battle(
        &self,
        village_id: u32,
        command: &ResolveAttackBattle,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.resolve_attack_battle(village_id, command).await
    }

    pub async fn apply_battle_outcome_to_village(
        &self,
        village_id: u32,
        command: &ApplyBattleOutcomeToVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .apply_battle_outcome_to_village(village_id, command)
            .await
    }

    pub async fn send_resources(
        &self,
        village_id: u32,
        command: &SendMerchantsTransfer,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_resources(village_id, command).await
    }

    pub async fn create_marketplace_offer(
        &self,
        village_id: u32,
        command: &CreateMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(village_id, command.player_id)
            .await?;
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_marketplace_offer(village_id, command).await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        village_id: u32,
        player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
    ) -> Result<u32, CqrsError> {
        let offer = self.get_marketplace_offer(offer_id).await?;
        if offer.status != MarketplaceOfferStatus::Open
            || offer.owner_village_id != village_id
            || offer.owner_player_id != player_id
        {
            return Err(CqrsError::domain_source(GameError::InvalidMarketplaceOffer));
        }

        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .cancel_marketplace_offer(
                village_id,
                &CancelMarketplaceOffer {
                    player_id,
                    offer: Self::as_offer_snapshot(offer),
                },
            )
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        accepting_village_id: u32,
        accepting_player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
        owner_arrives_at: chrono::DateTime<chrono::Utc>,
        accepting_arrives_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<u32, CqrsError> {
        self.materialize_current_resources_for_command(accepting_village_id, accepting_player_id)
            .await?;
        let offers = PostgresMarketplaceRepository::new(self.pool.clone());
        let offer = offers
            .get_by_offer_id(offer_id)
            .await
            .map_err(CqrsError::domain_source)?;
        if offer.status != MarketplaceOfferStatus::Open {
            return Err(CqrsError::domain_source(
                GameError::MarketplaceOfferNoLongerValid,
            ));
        }
        let offer_snapshot = Self::as_offer_snapshot(offer.clone());
        MarketplaceAcceptance {
            accepting_player_id,
            accepting_village_id,
            offer: &offer_snapshot,
        }
        .validate()
        .map_err(CqrsError::domain_source)?;

        let accepting_model = self.get_village(accepting_village_id).await?;
        if accepting_model.player_id != accepting_player_id {
            return Err(CqrsError::domain_source(GameError::VillageNotOwned {
                village_id: accepting_village_id,
                player_id: accepting_player_id,
            }));
        }
        let accepting_village =
            Village::try_from(accepting_model).map_err(CqrsError::domain_source)?;
        let seek_group: parabellum_types::common::ResourceGroup = offer.seek_resources.into();
        let accepting_merchants_used = accepting_village
            .validate_merchant_transfer(
                &seek_group,
                parabellum_app::config::Config::from_env().speed,
            )
            .map_err(CqrsError::domain_source)?;
        let mut accepting_after = accepting_village.clone();
        accepting_after
            .reserve_merchant_transfer(&seek_group, accepting_merchants_used)
            .map_err(CqrsError::domain_source)?;

        let accepted_at = chrono::Utc::now();
        let Some(offer) = offers
            .claim_open_for_accept(
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
            )
            .await
            .map_err(CqrsError::domain_source)?
        else {
            return Err(CqrsError::domain_source(
                GameError::MarketplaceOfferNoLongerValid,
            ));
        };
        self.append_workflow_events(workflows::merchants::offer_acceptance_events(
            workflows::merchants::OfferAcceptanceWorkflowInput {
                offer: &offer,
                accepting_player_id,
                accepting_village_id,
                accepting_stocks: accepting_after.stocks().clone(),
                accepting_busy_merchants: accepting_after.busy_merchants,
                accepting_merchants_used,
                accepted_at,
                owner_arrives_at,
                accepting_arrives_at,
            },
        ))
        .await?;
        Ok(accepting_village_id)
    }

    /// Executes the village resource utility command through the ES runtime.
    pub async fn set_village_resources(
        &self,
        village_id: u32,
        command: &SetVillageResources,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.set_village_resources(village_id, command).await
    }

    /// Executes due scheduled actions by appending canonical workflow facts.
    ///
    /// Status transitions are persisted for each action (`completed` or `failed`).
    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, CqrsError> {
        let Some(lock) =
            AdvisoryLock::try_acquire(&self.pool, SCHEDULED_ACTION_EXECUTION_LOCK_KEY).await?
        else {
            info!(
                action = "scheduler_skip_locked",
                "scheduled action execution lock is held; skipping tick"
            );
            return Ok(0);
        };

        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let stale_before = before_or_equal
            - chrono::Duration::seconds(SCHEDULED_ACTION_PROCESSING_STALE_AFTER_SECS);
        let requeued = repo
            .requeue_stale_processing(stale_before)
            .await
            .map_err(CqrsError::domain_source)?;
        if requeued > 0 {
            warn!(
                action = "scheduler_requeue_stale_processing",
                requeued,
                stale_before = %stale_before,
                "requeued stale processing scheduled actions to pending"
            );
        }
        let actions = repo
            .take_due_pending(before_or_equal, limit)
            .await
            .map_err(CqrsError::domain_source)?;
        let claimed = actions.len();
        if claimed > 0 {
            info!(
                action = "scheduler_claim_due",
                claimed,
                limit,
                before_or_equal = %before_or_equal,
                "claimed due scheduled actions"
            );
        }

        let result = self.process_actions(&actions).await;
        lock.release().await?;
        result
    }

    pub async fn process_actions(
        &self,
        actions: &Vec<ScheduledAction>,
    ) -> Result<usize, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        let mut processed = 0usize;
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());

        for action in actions {
            let result = scheduler::execute_action(self, &service, action).await;
            let next_status = if result.is_ok() {
                ScheduledActionStatus::Completed
            } else if matches!(result, Err(CqrsError::Conflict { .. })) {
                ScheduledActionStatus::Pending
            } else {
                ScheduledActionStatus::Failed
            };
            repo.update_status(action.id, next_status)
                .await
                .map_err(CqrsError::domain_source)?;
            if let Err(err) = &result {
                warn!(
                    action = "scheduler_action_failed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    error = %err,
                    "scheduled action marked failed"
                );
            } else {
                info!(
                    action = "scheduler_action_completed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    "scheduled action completed"
                );
            }
            processed += 1;
        }
        Ok(processed)
    }

    /// Appends a cross-village workflow as one transactional event-store write.
    ///
    /// This is the strict-consistency primitive for multi-stream facts in the
    /// village bounded context:
    /// 1. group events by aggregate stream,
    /// 2. load expected versions,
    /// 3. append all grouped streams in one transaction,
    /// 4. project resulting stored events in global-sequence order.
    ///
    /// If any stream version check conflicts, nothing is appended.
    /// Appends multi-stream village workflow facts atomically, then projects them.
    ///
    /// Contract:
    /// - all stream writes succeed or none are committed
    /// - stream conflicts fail fast with `CqrsError::Conflict`
    /// - projector dispatch runs only after a successful append
    async fn append_village_workflow_events(
        &self,
        workflow_events: Vec<(u32, parabellum_app::villages::VillageEvent)>,
    ) -> Result<(), CqrsError> {
        if workflow_events.is_empty() {
            return Ok(());
        }

        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let store = PostgresEventStore::new(self.pool.clone());
        let streams = self.build_village_workflow_appends(workflow_events).await?;

        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        let mut stored = store
            .append_workflow_events_in_tx(&mut tx, aggregate_type, &streams)
            .await?;
        stored.sort_by_key(|event| event.global_sequence.unwrap_or(i64::MAX));

        let village_projector = VillageProjector::new(self.pool.clone());
        let report_projector = ReportProjector::new(self.pool.clone());
        for event in &stored {
            village_projector.process_in_tx(&mut tx, event).await?;
            report_projector.process_in_tx(&mut tx, event).await?;
        }
        tx.commit().await.map_err(CqrsError::domain_source)?;
        self.refresh_workflow_snapshots(&streams).await?;
        Ok(())
    }

    async fn refresh_workflow_snapshots(
        &self,
        streams: &[WorkflowStreamAppend],
    ) -> Result<(), CqrsError> {
        let aggregate_type = std::any::type_name::<parabellum_app::villages::VillageAggregate>();
        let event_store = PostgresEventStore::new(self.pool.clone());
        let snapshot_store = PostgresSnapshotStore::new(self.pool.clone());

        for stream in streams {
            let (events, version) = event_store
                .load_events(aggregate_type, &stream.aggregate_id)
                .await?;
            let mut aggregate = parabellum_app::villages::VillageAggregate::default();
            aggregate.set_aggregate_id(
                stream
                    .aggregate_id
                    .parse()
                    .map_err(|_| CqrsError::EventStore("invalid village aggregate id".into()))?,
            );
            aggregate.apply_events(&events).await?;
            aggregate.set_version(version);
            snapshot_store
                .save_snapshot(AggregateSnapshot::new(&aggregate, Some(version))?)
                .await?;
        }
        Ok(())
    }

    async fn append_workflow_events(
        &self,
        workflow_events: workflows::WorkflowEvents,
    ) -> Result<(), CqrsError> {
        if workflow_events.is_empty() {
            return Ok(());
        }

        self.append_village_workflow_events(workflow_events.into_inner())
            .await
    }
}
