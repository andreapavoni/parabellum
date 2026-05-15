//! Query/read helpers for `VillageEsService`.
//!
//! These methods are side-effect free with respect to aggregate mutations:
//! they compose read models, CQRS query projections, and derived views for API use.

use super::*;

impl VillageEsService {
    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, CqrsError> {
        let repo = PostgresVillageMovementRepository::new(self.pool.clone());
        let movements = repo
            .list_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for movement in movements {
            let origin_model = self.get_village(movement.origin_village_id).await.ok();
            let target_model = self.get_village(movement.target_village_id).await.ok();

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
                    .or_else(|| origin_model.as_ref().map(|v| v.village_name.clone())),
                origin_player_id: movement.origin_player_id,
                origin_position: movement
                    .origin_position
                    .or_else(|| origin_model.as_ref().map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                target_village_id: movement.target_village_id,
                target_village_name: movement
                    .target_village_name
                    .or_else(|| target_model.as_ref().map(|v| v.village_name.clone())),
                target_player_id: movement
                    .target_player_id
                    .or_else(|| target_model.as_ref().map(|v| v.player_id))
                    .unwrap_or(movement.origin_player_id),
                target_position: movement
                    .target_position
                    .or_else(|| target_model.as_ref().map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                arrives_at: movement.arrives_at,
                time_seconds: movement.time_seconds.unwrap_or(0),
                units: movement.units,
                tribe: movement
                    .tribe
                    .or_else(|| origin_model.as_ref().map(|v| v.tribe.clone()))
                    .unwrap_or(parabellum_types::tribe::Tribe::Nature),
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
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_hero(
        &self,
        hero_id: uuid::Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, CqrsError> {
        let repo = PostgresHeroRepository::new(self.pool.clone());
        repo.get_by_id(hero_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn player_has_alive_hero(&self, player_id: uuid::Uuid) -> Result<bool, CqrsError> {
        let repo: Arc<dyn HeroRepository> =
            Arc::new(PostgresHeroRepository::new(self.pool.clone()));
        repo.has_alive_for_player(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn player_has_pending_hero_revival(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<bool, CqrsError> {
        PostgresScheduledActionRepository::new(self.pool.clone())
            .has_pending_hero_revival_for_player(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_villages_by_player_id(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_expansion_culture_info(
        &self,
        player_id: uuid::Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<ExpansionCultureInfo, CqrsError> {
        let player_repo = PostgresPlayerRepository::new(self.pool.clone());
        let village_repo = PostgresVillageRepository::new(self.pool.clone());

        let player = player_repo
            .get_by_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let player_culture_points_production = player_repo
            .get_total_culture_points_production(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let villages = village_repo
            .list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let village = villages
            .iter()
            .find(|v| v.village_id == village_id)
            .ok_or_else(|| CqrsError::EventStore(DbError::VillageNotFound(village_id).to_string()))?;

        let speed = match server_speed {
            1 => parabellum_types::common::Speed::X1,
            2 => parabellum_types::common::Speed::X2,
            3 => parabellum_types::common::Speed::X3,
            5 => parabellum_types::common::Speed::X5,
            10 => parabellum_types::common::Speed::X10,
            _ => parabellum_types::common::Speed::X1,
        };
        let next_cp_required = required_cp(speed, villages.len() + 1);

        Ok(ExpansionCultureInfo {
            village_culture_points: village.culture_points,
            village_culture_points_production: village.culture_points_production,
            player_culture_points: player.culture_points as u32,
            player_culture_points_production,
            next_cp_required,
        })
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
            .map_err(|e| CqrsError::EventStore(e.to_string()))?
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

    pub async fn get_village_training_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::TrainUnit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_building_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let mut actions = Vec::new();
        for action_type in [
            ScheduledActionType::AddBuilding,
            ScheduledActionType::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding,
        ] {
            let mut typed = repo
                .list_active_by_village_and_type(village_id, action_type)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            actions.append(&mut typed);
        }
        actions.sort_by_key(|it| it.execute_at);
        Ok(actions)
    }

    pub async fn get_village_smithy_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::ResearchSmithy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_academy_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::ResearchAcademy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, CqrsError> {
        let mut building = Vec::new();
        let building_actions = self.get_village_building_queue(village_id).await?;
        for action in building_actions {
            let Ok(payload) = serde_json::from_value::<ScheduledActionPayload>(action.payload) else {
                continue;
            };
            let (slot_id, building_name, target_level) = match payload {
                ScheduledActionPayload::AddBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                }
                | ScheduledActionPayload::UpgradeBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                }
                | ScheduledActionPayload::DowngradeBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                } => (slot_id, building_name, level),
                _ => continue,
            };
            building.push(BuildingQueueItem {
                job_id: action.id,
                slot_id,
                building_name,
                target_level,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        building.sort_by_key(|it| it.finishes_at);

        let mut training = Vec::new();
        let training_actions = self.get_village_training_queue(village_id).await?;
        for action in training_actions {
            let Ok(ScheduledActionPayload::TrainUnit {
                slot_id,
                unit,
                quantity_remaining,
                time_per_unit,
                ..
            }) = serde_json::from_value::<ScheduledActionPayload>(action.payload) else {
                continue;
            };
            training.push(TrainingQueueItem {
                job_id: action.id,
                slot_id,
                unit,
                quantity: quantity_remaining,
                time_per_unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        training.sort_by_key(|it| it.finishes_at);

        let mut academy = Vec::new();
        let academy_actions = self.get_village_academy_queue(village_id).await?;
        for action in academy_actions {
            let Ok(ScheduledActionPayload::ResearchAcademy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            academy.push(AcademyQueueItem {
                job_id: action.id,
                unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        academy.sort_by_key(|it| it.finishes_at);

        let mut smithy = Vec::new();
        let smithy_actions = self.get_village_smithy_queue(village_id).await?;
        for action in smithy_actions {
            let Ok(ScheduledActionPayload::ResearchSmithy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            smithy.push(SmithyQueueItem {
                job_id: action.id,
                unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        smithy.sort_by_key(|it| it.finishes_at);

        Ok(VillageQueues {
            building,
            training,
            academy,
            smithy,
        })
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
            .map_err(|e| CqrsError::EventStore(e.to_string()))
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
        let all_open_models = self.get_open_marketplace_offers().await?;

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

        let outgoing_merchants = marketplace_repo
            .list_active_outgoing(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let incoming_merchants = marketplace_repo
            .list_active_incoming(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let village_ids = all_open_models
            .iter()
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
        let village_info = self.get_village_info_by_ids(village_ids).await?;

        Ok(MarketplaceData {
            own_offers: all_open_models
                .iter()
                .filter(|m| m.owner_village_id == village_id)
                .cloned()
                .map(to_offer)
                .collect(),
            global_offers: all_open_models
                .into_iter()
                .filter(|m| m.owner_village_id != village_id)
                .map(to_offer)
                .collect(),
            outgoing_merchants,
            incoming_merchants,
            village_info,
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
        let home_army = repo
            .get_home_army(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let reinforcements = repo
            .list_stationed_armies(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let deployed_armies = repo
            .list_deployed_armies(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(VillageArmyStateView {
            home_army,
            reinforcements,
            deployed_armies,
        })
    }

    pub async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageInfo>, CqrsError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let repo = PostgresVillageRepository::new(self.pool.clone());
        let villages = repo
            .list_by_village_ids(&village_ids)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        Ok(villages
            .into_iter()
            .map(|v| {
                (
                    v.village_id,
                    VillageInfo {
                        id: v.village_id,
                        name: v.village_name,
                        position: v.position,
                    },
                )
            })
            .collect())
    }

    pub async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<LeaderboardPage, CqrsError> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = (page - 1) * per_page;
        let repo = PostgresPlayerRepository::new(self.pool.clone());
        let (entries, total_players) = repo
            .leaderboard_page(offset, per_page)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(LeaderboardPage {
            entries,
            total_players,
        })
    }

    pub async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_region(center_x, center_y, radius, world_size)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_map_field(&self, field_id: u32) -> Result<MapField, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_field_by_id(field_id as i32)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<MapRegionTile>, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_region_tile_by_field_id(field_id as i32)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&ListReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                player_id,
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

    pub async fn mark_report_as_read(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let repo = PostgresReportRepository::new(self.pool.clone());
        repo.mark_as_read(report_id, player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
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
        })
    }
}
