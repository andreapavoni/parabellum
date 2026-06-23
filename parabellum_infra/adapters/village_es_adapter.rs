use std::collections::HashMap;
use uuid::Uuid;

use async_trait::async_trait;

use parabellum_app::{
    identity::{InitialVillageCommandExecutor, PlayerRepository},
    scheduler::SchedulerPort,
    villages::ports::{
        BuildingCommandExecutor, BuildingCommandIntent, BuildingReadPort,
        DevelopmentCommandExecutor, DevelopmentCommandIntent, DevelopmentReadPort,
        ExpansionReadPort, HeroCommandExecutor, HeroCommandIntent, HeroReadPort,
        MarketplaceCommandExecutor, MarketplaceCommandIntent, MarketplaceReadPort,
        MovementControlCommandExecutor, MovementControlCommandIntent, MovementControlReadPort,
        MovementReadPort, ReinforcementArmyContext, ReinforcementCommandExecutor,
        ReinforcementCommandIntent, ReinforcementReadPort, ReportCommandExecutor,
        ReportCommandIntent, ReportReadPort, TrapCommandExecutor, TrapCommandIntent, TrapReadPort,
        TrappedArmyContext, VillageActivityReadPort, VillageArmyReadPort, VillageCommandExecutor,
        VillageCommandIntent, VillageProfileCommandExecutor, VillageProfileCommandIntent,
        VillageReferenceReadPort, VillageStateReadPort,
    },
    villages::projection_repositories::VillageRepository,
    villages::read_models::{
        MarketplaceData, VillageArmyStateView, VillageQueues, VillageTroopMovements,
    },
    villages::{CreateHero, FoundVillage, SetVillageResources},
};
use parabellum_types::errors::{AppError, ApplicationError, DbError, GameError};

use crate::es::{PostgresVillageRepository, VillageEsService};
use crate::identity::repositories::PostgresPlayerRepository;

#[derive(Clone)]
/// Transport adapter implementing app ports by delegating to ES service/query flows.
pub struct VillageEsAdapter {
    service: VillageEsService,
}

impl VillageEsAdapter {
    pub fn new(service: VillageEsService) -> Self {
        Self { service }
    }

    /// Maps CQRS/runtime failures to stable app-layer error categories.
    ///
    /// Why this exists:
    /// - HTTP contract mapping is done from `ApplicationError` variants.
    /// - Leaving CQRS failures as opaque `Unknown` turns client-visible `4xx`
    ///   conditions into `500`.
    ///
    /// Policy:
    /// - stream/version conflicts -> app conflict bucket
    /// - domain/invariant source errors -> downcast into typed app/domain errors
    /// - string domain/invariant payloads (legacy paths) -> minimal compatibility mapping
    /// - everything else remains `Unknown` and is treated as internal
    fn map_cqrs_error(err: mini_cqrs_es::CqrsError) -> ApplicationError {
        match err {
            mini_cqrs_es::CqrsError::Conflict { .. } => {
                ApplicationError::App(AppError::QueueItemAlreadyQueued {
                    queue: "cqrs",
                    item: "aggregate_version".to_string(),
                })
            }
            mini_cqrs_es::CqrsError::DomainSource(source)
            | mini_cqrs_es::CqrsError::CommandInvariantSource(source) => {
                let source = match source.downcast::<GameError>() {
                    Ok(game_error) => return ApplicationError::Game(*game_error),
                    Err(source) => source,
                };
                let source = match source.downcast::<AppError>() {
                    Ok(app_error) => return ApplicationError::App(*app_error),
                    Err(source) => source,
                };
                let source = match source.downcast::<ApplicationError>() {
                    Ok(app_error) => return *app_error,
                    Err(source) => source,
                };
                ApplicationError::Unknown(source.to_string())
            }
            mini_cqrs_es::CqrsError::Domain(msg)
            | mini_cqrs_es::CqrsError::CommandInvariant(msg) => ApplicationError::Unknown(format!(
                "unexpected stringly cqrs domain/invariant error: {msg}"
            )),
            mini_cqrs_es::CqrsError::Other(other) => {
                if let Some(game_error) = other
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<GameError>())
                {
                    return ApplicationError::Game(game_error.clone());
                }
                if let Some(app_error) = other
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<AppError>())
                {
                    return ApplicationError::App(app_error.clone());
                }
                ApplicationError::Unknown(other.to_string())
            }
            other => ApplicationError::Unknown(other.to_string()),
        }
    }

    /// Maps read/query failures that currently cross the ES boundary as
    /// typed source errors.
    ///
    /// Keep this intentionally narrow: only map well-known not-found cases to
    /// typed DB errors so web layer can return stable `404` contracts.
    fn map_query_cqrs_error(err: mini_cqrs_es::CqrsError) -> ApplicationError {
        match err {
            mini_cqrs_es::CqrsError::DomainSource(source)
            | mini_cqrs_es::CqrsError::CommandInvariantSource(source) => {
                let source = match source.downcast::<ApplicationError>() {
                    Ok(app_error) => return *app_error,
                    Err(source) => source,
                };
                let source = match source.downcast::<DbError>() {
                    Ok(db_error) => return ApplicationError::Db(*db_error),
                    Err(source) => source,
                };
                let source = match source.downcast::<GameError>() {
                    Ok(game_error) => return ApplicationError::Game(*game_error),
                    Err(source) => source,
                };
                let source = match source.downcast::<AppError>() {
                    Ok(app_error) => return ApplicationError::App(*app_error),
                    Err(source) => source,
                };
                ApplicationError::Unknown(source.to_string())
            }
            other => ApplicationError::Unknown(other.to_string()),
        }
    }
}

#[async_trait]
impl MovementReadPort for VillageEsAdapter {
    async fn get_movement_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_movement_hero(
        &self,
        hero_id: Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, ApplicationError> {
        self.service
            .get_hero(hero_id)
            .await
            .map_err(Self::map_cqrs_error)
    }

    async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, ApplicationError> {
        self.service
            .is_unoccupied_valley(field_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl MarketplaceReadPort for VillageEsAdapter {
    async fn get_marketplace_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<parabellum_app::villages::models::MarketplaceOfferModel, ApplicationError> {
        self.service
            .get_marketplace_offer(offer_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::MarketplaceOfferNotFound(offer_id)))
    }

    async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, ApplicationError> {
        self.service
            .get_marketplace_data(village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id)))
    }
}

#[async_trait]
impl MovementControlReadPort for VillageEsAdapter {
    async fn get_cancel_troop_movement_context(
        &self,
        movement_id: Uuid,
    ) -> Result<parabellum_app::villages::CancelTroopMovementContext, ApplicationError> {
        let context = self
            .service
            .find_cancel_troop_movement_context(movement_id)
            .await
            .map_err(Self::map_query_cqrs_error)?;
        Ok(parabellum_app::villages::CancelTroopMovementContext {
            movement_id: context.movement_id,
            arrival_action_id: context.arrival_action_id,
            army_id: context.army_id,
            player_id: context.player_id,
            source_village_id: context.source_village_id,
            target_village_id: context.target_village_id,
            army: context.army,
            sent_at: context.sent_at,
            arrives_at: context.arrives_at,
        })
    }

    async fn get_movement_control_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl ReinforcementReadPort for VillageEsAdapter {
    async fn get_reinforcement_context(
        &self,
        army_id: Uuid,
    ) -> Result<ReinforcementArmyContext, ApplicationError> {
        let context = self
            .service
            .find_reinforcement_context(army_id)
            .await
            .map_err(Self::map_query_cqrs_error)?;
        Ok(ReinforcementArmyContext {
            stationed_village_id: context.stationed_village_id,
            home_village_id: context.home_village_id,
            army: context.army,
        })
    }

    async fn get_trapped_army_context(
        &self,
        army_id: Uuid,
    ) -> Result<TrappedArmyContext, ApplicationError> {
        let context = self
            .service
            .find_trapped_army_context(army_id)
            .await
            .map_err(Self::map_query_cqrs_error)?;
        Ok(TrappedArmyContext {
            trapped_village_id: context.trapped_village_id,
            home_village_id: context.home_village_id,
            army: context.army,
        })
    }

    async fn get_reinforcement_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_reinforcement_army_state(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError> {
        self.service
            .get_village_army_state_view(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl TrapReadPort for VillageEsAdapter {
    async fn get_trap_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_trap_army_state(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError> {
        self.service
            .get_village_army_state_view(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl BuildingReadPort for VillageEsAdapter {
    async fn get_cancel_building_construction_context(
        &self,
        village_id: u32,
        action_id: Uuid,
        canceled_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<parabellum_app::villages::CancelBuildingConstructionContext, ApplicationError> {
        let context = self
            .service
            .find_cancel_building_construction_context(village_id, action_id, canceled_at)
            .await
            .map_err(Self::map_query_cqrs_error)?;

        Ok(
            parabellum_app::villages::CancelBuildingConstructionContext {
                action_ids: context.action_ids,
                player_id: context.player_id,
                village_id: context.village_id,
                execute_at: context.execute_at,
                refund: context.refund,
            },
        )
    }
}

#[async_trait]
impl DevelopmentReadPort for VillageEsAdapter {
    async fn get_development_village(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn count_development_child_villages(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<u8, ApplicationError> {
        self.service
            .count_child_villages(player_id, village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_development_village_queues(
        &self,
        village_id: u32,
    ) -> Result<VillageQueues, ApplicationError> {
        self.service
            .get_village_queues(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_development_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        self.service
            .get_village_troop_movements(village_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl HeroReadPort for VillageEsAdapter {
    async fn get_hero(
        &self,
        hero_id: Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, ApplicationError> {
        self.service
            .get_hero(hero_id)
            .await
            .map_err(Self::map_cqrs_error)
    }

    async fn get_hero_by_player(
        &self,
        player_id: Uuid,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, ApplicationError> {
        self.service
            .get_hero_by_player(player_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn player_has_alive_hero(&self, player_id: Uuid) -> Result<bool, ApplicationError> {
        self.service
            .player_has_alive_hero(player_id)
            .await
            .map_err(Self::map_cqrs_error)
    }

    async fn player_has_pending_hero_revival(
        &self,
        player_id: Uuid,
    ) -> Result<bool, ApplicationError> {
        self.service
            .player_has_pending_hero_revival(player_id)
            .await
            .map_err(Self::map_cqrs_error)
    }

    async fn get_pending_hero_revival_at(
        &self,
        player_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApplicationError> {
        self.service
            .pending_hero_revival_at(player_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl VillageCommandExecutor for VillageEsAdapter {
    async fn execute_village_command(
        &self,
        village_id: u32,
        command: VillageCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            VillageCommandIntent::SendReinforcement(command) => self
                .service
                .send_reinforcement(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            VillageCommandIntent::AttackVillage(command) => self
                .service
                .send_attack(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            VillageCommandIntent::ScoutVillage(command) => self
                .service
                .send_scout(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            VillageCommandIntent::SendSettlers(command) => self
                .service
                .send_settlers(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl BuildingCommandExecutor for VillageEsAdapter {
    async fn execute_building_command(
        &self,
        command: BuildingCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            BuildingCommandIntent::AddBuilding {
                village_id,
                command,
            } => self
                .service
                .add_building(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            BuildingCommandIntent::UpgradeBuilding {
                village_id,
                command,
            } => self
                .service
                .upgrade_building(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            BuildingCommandIntent::DowngradeBuilding {
                village_id,
                command,
            } => self
                .service
                .downgrade_building(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            BuildingCommandIntent::CancelBuildingConstruction {
                village_id,
                command,
            } => self
                .service
                .cancel_building_construction(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl VillageProfileCommandExecutor for VillageEsAdapter {
    async fn execute_village_profile_command(
        &self,
        command: VillageProfileCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            VillageProfileCommandIntent::RenameVillage {
                village_id,
                command,
            } => self
                .service
                .rename_village(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl DevelopmentCommandExecutor for VillageEsAdapter {
    async fn execute_development_command(
        &self,
        command: DevelopmentCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            DevelopmentCommandIntent::TrainUnits {
                village_id,
                command,
            } => self
                .service
                .train_units(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            DevelopmentCommandIntent::ResearchAcademy {
                village_id,
                command,
            } => self
                .service
                .research_academy(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            DevelopmentCommandIntent::ResearchSmithy {
                village_id,
                command,
            } => self
                .service
                .research_smithy(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl HeroCommandExecutor for VillageEsAdapter {
    async fn execute_hero_command(
        &self,
        command: HeroCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            HeroCommandIntent::CreateHero {
                village_id,
                command,
            } => self
                .service
                .create_hero(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            HeroCommandIntent::ReviveHero {
                village_id,
                command,
            } => self
                .service
                .revive_hero(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            HeroCommandIntent::AssignHeroPoints {
                village_id,
                command,
            } => self
                .service
                .assign_hero_points(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            HeroCommandIntent::ResetHeroPoints {
                village_id,
                command,
            } => self
                .service
                .reset_hero_points(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            HeroCommandIntent::SetHeroResourceFocus {
                village_id,
                command,
            } => self
                .service
                .set_hero_resource_focus(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl MarketplaceCommandExecutor for VillageEsAdapter {
    async fn execute_marketplace_command(
        &self,
        command: MarketplaceCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            MarketplaceCommandIntent::SendResources {
                source_village_id,
                command,
            } => self
                .service
                .send_resources(source_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            MarketplaceCommandIntent::CreateOffer {
                village_id,
                command,
            } => self
                .service
                .create_marketplace_offer(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            MarketplaceCommandIntent::AcceptOffer {
                accepting_village_id,
                accepting_player_id,
                offer_id,
                owner_arrives_at,
                accepting_arrives_at,
            } => self
                .service
                .accept_marketplace_offer(
                    accepting_village_id,
                    accepting_player_id,
                    offer_id,
                    owner_arrives_at,
                    accepting_arrives_at,
                )
                .await
                .map_err(Self::map_cqrs_error)?,
            MarketplaceCommandIntent::CancelOffer {
                village_id,
                player_id,
                offer_id,
            } => self
                .service
                .cancel_marketplace_offer(village_id, player_id, offer_id)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl MovementControlCommandExecutor for VillageEsAdapter {
    async fn execute_movement_control_command(
        &self,
        command: MovementControlCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            MovementControlCommandIntent::CancelTroopMovement {
                source_village_id,
                command,
            } => self
                .service
                .cancel_troop_movement(source_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl ReinforcementCommandExecutor for VillageEsAdapter {
    async fn execute_reinforcement_command(
        &self,
        command: ReinforcementCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            ReinforcementCommandIntent::RecallReinforcements {
                home_village_id,
                command,
            } => self
                .service
                .recall_reinforcements(home_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            ReinforcementCommandIntent::ReleaseReinforcements {
                stationed_village_id,
                command,
            } => self
                .service
                .release_reinforcements(stationed_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            ReinforcementCommandIntent::ReleaseTrappedTroops {
                trapped_village_id,
                command,
            } => self
                .service
                .release_trapped_troops(trapped_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
            ReinforcementCommandIntent::DisbandTrappedTroops {
                trapped_village_id,
                command,
            } => self
                .service
                .disband_trapped_troops(trapped_village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl TrapCommandExecutor for VillageEsAdapter {
    async fn execute_trap_command(
        &self,
        command: TrapCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            TrapCommandIntent::BuildTraps {
                village_id,
                command,
            } => self
                .service
                .build_traps(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl VillageActivityReadPort for VillageEsAdapter {
    async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, ApplicationError> {
        self.service
            .get_village_queues(village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id)))
    }

    async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        self.service
            .get_village_troop_movements(village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id)))
    }

    async fn list_cancelable_outgoing_movement_ids(
        &self,
        village_id: u32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<std::collections::HashSet<Uuid>, ApplicationError> {
        self.service
            .list_cancelable_outgoing_movement_ids(village_id, now)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl VillageArmyReadPort for VillageEsAdapter {
    async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError> {
        self.service
            .get_village_army_state_view(village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id)))
    }
}

#[async_trait]
impl VillageStateReadPort for VillageEsAdapter {
    async fn list_player_village_states(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<parabellum_app::villages::models::VillageModel>, ApplicationError> {
        self.service
            .list_player_village_states(player_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_village_state(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id)))
    }
}

#[async_trait]
impl VillageReferenceReadPort for VillageEsAdapter {
    async fn get_village_references(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, parabellum_app::read_models::VillageReference>, ApplicationError> {
        let repo = PostgresVillageRepository::new(self.service.pool().clone());
        let villages = repo.list_by_village_ids(&village_ids).await?;

        Ok(villages
            .into_iter()
            .map(|v| {
                (
                    v.village_id,
                    parabellum_app::read_models::VillageReference {
                        id: v.village_id,
                        name: v.village_name,
                        position: v.position,
                    },
                )
            })
            .collect())
    }
}

#[async_trait]
impl ExpansionReadPort for VillageEsAdapter {
    async fn get_expansion_culture_snapshot(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<
        parabellum_app::villages::projection_repositories::ExpansionCultureSnapshot,
        ApplicationError,
    > {
        PostgresVillageRepository::new(self.service.pool().clone())
            .get_expansion_culture_snapshot(player_id, village_id)
            .await
    }

    async fn refresh_player_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError> {
        PostgresPlayerRepository::new(self.service.pool().clone())
            .update_culture_points(player_id)
            .await
    }

    async fn get_player(
        &self,
        player_id: Uuid,
    ) -> Result<parabellum_types::common::Player, ApplicationError> {
        PostgresPlayerRepository::new(self.service.pool().clone())
            .get_by_id(player_id)
            .await
    }
}

#[async_trait]
impl ReportReadPort for VillageEsAdapter {
    async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<parabellum_app::villages::models::ReportModel>, ApplicationError> {
        self.service
            .list_reports_for_player(player_id, offset, limit)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<parabellum_app::villages::models::ReportModel>, ApplicationError> {
        self.service
            .get_report_for_player(report_id, player_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }

    async fn count_unread_reports_for_player(
        &self,
        player_id: Uuid,
    ) -> Result<i64, ApplicationError> {
        self.service
            .count_unread_reports_for_player(player_id)
            .await
            .map_err(Self::map_query_cqrs_error)
    }
}

#[async_trait]
impl ReportCommandExecutor for VillageEsAdapter {
    async fn execute_report_command(
        &self,
        command: ReportCommandIntent,
    ) -> Result<(), ApplicationError> {
        match command {
            ReportCommandIntent::MarkReportRead {
                village_id,
                command,
            } => self
                .service
                .mark_report_read(village_id, &command)
                .await
                .map_err(Self::map_cqrs_error)?,
        };
        Ok(())
    }
}

#[async_trait]
impl InitialVillageCommandExecutor for VillageEsAdapter {
    async fn found_initial_village(
        &self,
        village_id: u32,
        command: FoundVillage,
    ) -> Result<(), ApplicationError> {
        self.service
            .found_village(village_id, &command)
            .await
            .map_err(Self::map_cqrs_error)
    }

    async fn create_initial_hero(
        &self,
        village_id: u32,
        command: CreateHero,
    ) -> Result<(), ApplicationError> {
        self.service
            .create_hero(village_id, &command)
            .await
            .map(|_| ())
            .map_err(Self::map_cqrs_error)
    }

    async fn set_initial_village_resources(
        &self,
        village_id: u32,
        command: SetVillageResources,
    ) -> Result<(), ApplicationError> {
        self.service
            .set_village_resources(village_id, &command)
            .await
            .map_err(Self::map_cqrs_error)
    }
}

#[async_trait]
impl SchedulerPort for VillageEsAdapter {
    async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError> {
        self.service
            .process_due_actions(before_or_equal, limit)
            .await
            .map_err(Self::map_cqrs_error)
    }
}
