//! Scheduled-action payload executor for `VillageEsService`.
//!
//! Each payload variant is mapped to a deterministic completion command.
//! Validation is assumed to have happened at scheduling time; this layer executes
//! payload intent and applies terminal status (`completed`/`failed`) upstream.

use super::*;

/// Executes one scheduled action payload by dispatching exactly one completion flow.
pub(super) async fn execute_action(
    svc: &VillageEsService,
    service: &VillageService<'_, crate::es::VillageCqrsRuntime>,
    action: &parabellum_app::villages::models::ScheduledAction,
) -> Result<(), CqrsError> {
    let payload: ScheduledActionPayload =
        serde_json::from_value(action.payload.clone()).map_err(CqrsError::Serialization)?;
    match payload {
        ScheduledActionPayload::ReinforcementArrival {
            movement_id,
            army_id,
            player_id,
            source_village_id,
            target_village_id,
            army,
            arrives_at,
        } => {
            let command = ReinforcementArrived {
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                arrives_at,
            };
            service
                .reinforcement_arrived(source_village_id, &command)
                .await?;
        }
        ScheduledActionPayload::SettlersArrival {
            action_id,
            movement_id,
            army_id,
            village_id: _,
            source_village_id,
            target_village_id,
            target_position,
            player_id,
            village_name,
            tribe,
            arrives_at,
        } => {
            let field_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rm_map_fields WHERE id = $1)")
                    .bind(target_village_id as i32)
                    .fetch_one(&svc.pool)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            let can_found = if field_exists {
                let claim = sqlx::query(
                    r#"
                        UPDATE rm_map_fields
                        SET player_id = $2,
                            updated_at = NOW()
                        WHERE id = $1
                          AND village_id IS NULL
                          AND (player_id IS NULL OR player_id = $2)
                        "#,
                )
                .bind(target_village_id as i32)
                .bind(player_id)
                .execute(&svc.pool)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                claim.rows_affected() > 0
            } else {
                true
            };

            if can_found {
                let command = CompleteSettlersArrival {
                    action_id,
                    movement_id,
                    army_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    target_position: target_position.clone(),
                    village_name: village_name.clone(),
                    tribe: tribe.clone(),
                    arrives_at,
                };
                service
                    .complete_settlers_arrival(source_village_id, &command)
                    .await?;

                let found = FoundVillage {
                    village_name,
                    position: target_position,
                    tribe,
                    player_id,
                    buildings: vec![],
                };
                if let Err(err) = service.found_village(target_village_id, &found).await {
                    let is_already_founded = err.to_string().contains("is already founded");
                    if !is_already_founded {
                        return Err(err);
                    }
                }
                sqlx::query(
                    r#"
                        UPDATE rm_map_fields
                        SET village_id = $2,
                            player_id = $3,
                            updated_at = NOW()
                        WHERE id = $1
                          AND village_id IS NULL
                          AND player_id = $3
                    "#,
                )
                .bind(target_village_id as i32)
                .bind(target_village_id as i32)
                .bind(player_id)
                .execute(&svc.pool)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            } else {
                let army_repo: Arc<dyn ArmyRepository> =
                    Arc::new(PostgresArmyRepository::new(svc.pool.clone()));
                let army = army_repo
                    .get_moving_army(army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let source = svc.get_village(source_village_id).await?;
                let cfg = parabellum_app::config::Config::from_env();
                let travel_secs = source.position.calculate_travel_time_secs(
                    target_position.clone(),
                    army.speed(),
                    cfg.world_size as i32,
                    cfg.speed as u8,
                ) as i64;
                let returns_at = arrives_at + chrono::Duration::seconds(std::cmp::max(1, travel_secs));
                let return_action_id = uuid::Uuid::new_v4();
                PostgresScheduledActionRepository::new(svc.pool.clone())
                    .add(&ScheduledAction {
                        id: return_action_id,
                        action_type: ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army: army.clone(),
                            bounty: None,
                            returns_at,
                        }
                        .action_type(),
                        execute_at: returns_at,
                        payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                            action_id: return_action_id,
                            movement_id,
                            army_id,
                            village_id: source_village_id,
                            source_village_id,
                            target_village_id,
                            player_id,
                            army,
                            bounty: None,
                            returns_at,
                        })
                        .map_err(CqrsError::Serialization)?,
                        status: ScheduledActionStatus::Pending,
                    })
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            }
        }
        ScheduledActionPayload::AttackArrival {
            action_id,
            movement_id,
            army_id,
            return_action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            attack_type,
            catapult_targets,
            arrives_at,
            returns_at,
        } => {
            let command = CompleteAttackArrival {
                movement_id,
                army_id,
                action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army: army.clone(),
                attack_type: attack_type.clone(),
                catapult_targets,
                arrives_at,
                returns_at,
            };
            service
                .complete_attack_arrival(source_village_id, &command)
                .await?;

            let has_chief = army.units().get(8) > 0;
            if matches!(attack_type, parabellum_types::battle::AttackType::Normal) && has_chief {
                let target = svc.get_village(target_village_id).await?;
                if target.loyalty == 0 {
                    let conquer = ConquerVillage {
                        player_id,
                        village_id: target_village_id,
                    };
                    service.conquer_village(target_village_id, &conquer).await?;
                }
            }
        }
        ScheduledActionPayload::ArmyReturn {
            action_id,
            movement_id,
            army_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            returns_at,
            bounty,
        } => {
            let command = CompleteArmyReturn {
                action_id,
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                bounty,
                returns_at,
            };
            service
                .complete_army_return(source_village_id, &command)
                .await?;
        }
        ScheduledActionPayload::ScoutArrival {
            action_id,
            movement_id,
            army_id,
            return_action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            army,
            target,
            attack_type,
            arrives_at,
            returns_at,
        } => {
            let command = CompleteScoutArrival {
                movement_id,
                army_id,
                action_id,
                return_action_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                target,
                attack_type,
                arrives_at,
                returns_at,
            };
            service
                .complete_scout_arrival(source_village_id, &command)
                .await?;
        }
        ScheduledActionPayload::MerchantsArrival {
            action_id,
            village_id: _,
            source_village_id,
            target_village_id,
            player_id,
            resources,
            merchants_used,
            arrives_at,
        } => {
            let command = CompleteMerchantsArrival {
                action_id,
                player_id,
                source_village_id,
                target_village_id,
                resources,
                merchants_used,
                arrives_at,
            };
            service
                .complete_merchant_arrival(source_village_id, &command)
                .await?;
        }
        ScheduledActionPayload::MerchantsReturn {
            action_id,
            village_id: _,
            source_village_id,
            player_id,
            merchants_used,
            returns_at,
        } => {
            let command = CompleteMerchantsReturn {
                action_id,
                player_id,
                source_village_id,
                merchants_used,
                returns_at,
            };
            service
                .complete_merchant_return(source_village_id, &command)
                .await?;
        }
        ScheduledActionPayload::AddBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let command = CompleteAddBuilding {
                action_id: action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            };
            service.complete_add_building(village_id, &command).await?;
        }
        ScheduledActionPayload::UpgradeBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let command = CompleteUpgradeBuilding {
                action_id: action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            };
            service
                .complete_upgrade_building(village_id, &command)
                .await?;
        }
        ScheduledActionPayload::DowngradeBuilding {
            village_id,
            player_id,
            slot_id,
            building_name,
            level,
            speed,
        } => {
            let command = CompleteDowngradeBuilding {
                action_id: action.id,
                player_id,
                village_id,
                slot_id,
                building_name,
                level,
                speed,
            };
            service
                .complete_downgrade_building(village_id, &command)
                .await?;
        }
        ScheduledActionPayload::TrainUnit {
            action_id,
            village_id,
            player_id,
            slot_id,
            unit,
            time_per_unit,
            quantity_remaining,
            execute_at,
        } => {
            let command = CompleteTrainUnit {
                action_id,
                player_id,
                village_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            };
            service.complete_train_unit(village_id, &command).await?;
        }
        ScheduledActionPayload::ResearchAcademy {
            action_id,
            village_id,
            player_id,
            unit,
        } => {
            let command = CompleteAcademyResearch {
                action_id,
                player_id,
                village_id,
                unit,
            };
            service
                .complete_academy_research(village_id, &command)
                .await?;
        }
        ScheduledActionPayload::ResearchSmithy {
            action_id,
            village_id,
            player_id,
            unit,
        } => {
            let command = CompleteSmithyResearch {
                action_id,
                player_id,
                village_id,
                unit,
            };
            service
                .complete_smithy_research(village_id, &command)
                .await?;
        }
        ScheduledActionPayload::HeroRevival {
            action_id,
            village_id,
            player_id,
            hero,
            reset,
            revive_at,
        } => {
            let command = CompleteHeroRevival {
                action_id,
                player_id,
                village_id,
                hero,
                reset,
                revived_at: revive_at,
            };
            service.complete_hero_revival(village_id, &command).await?;
        }
    }
    Ok(())
}
