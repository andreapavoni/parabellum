use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::repositories::VillageModelRepository;
use parabellum_game::battle::Battle;
use parabellum_game::models::army::Army;
use parabellum_game::models::buildings::Building;
use parabellum_game::models::village::Village;
use parabellum_types::common::ResourceGroup;
use parabellum_types::reports::{
    BattlePartyPayload, BattleReportPayload, MarketplaceDeliveryReportPayload,
    ReinforcementReportPayload, ReportPayload,
};
use sqlx::{PgPool, Row};
use tracing::warn;
use uuid::Uuid;

use crate::es::{
    NewProjectedReport, PostgresReportReadModelRepository, PostgresVillageModelRepository,
};

#[derive(Debug, Clone)]
pub struct ReportProjector {
    pool: PgPool,
    villages: PostgresVillageModelRepository,
    reports: PostgresReportReadModelRepository,
}

impl ReportProjector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            villages: PostgresVillageModelRepository::new(pool.clone()),
            reports: PostgresReportReadModelRepository::new(pool.clone()),
            pool,
        }
    }

    fn village_from_model(model: &VillageModel) -> Village {
        Village::try_from(model.clone()).expect("VillageModel to Village conversion must succeed")
    }

    async fn player_username(&self, player_id: Uuid) -> Result<String, CqrsError> {
        let row = sqlx::query("SELECT username FROM players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(row.get::<String, _>("username"))
    }

    async fn try_village(&self, village_id: u32) -> Result<Option<VillageModel>, CqrsError> {
        match self.villages.get_by_village_id(village_id).await {
            Ok(v) => Ok(Some(v)),
            Err(_) => {
                warn!(
                    village_id,
                    "ReportProjector skipping event because village read model was not found"
                );
                Ok(None)
            }
        }
    }

    async fn project_reinforcement_arrived(
        &self,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        army: &Army,
    ) -> Result<(), CqrsError> {
        let Some(source) = self.try_village(source_village_id).await? else {
            return Ok(());
        };
        let Some(target) = self.try_village(target_village_id).await? else {
            return Ok(());
        };

        let payload = ReportPayload::Reinforcement(ReinforcementReportPayload {
            sender_player: self.player_username(source.player_id).await?,
            sender_village: source.village_name.clone(),
            sender_position: source.position.clone(),
            receiver_player: self.player_username(target.player_id).await?,
            receiver_village: target.village_name.clone(),
            receiver_position: target.position.clone(),
            tribe: army.tribe.clone(),
            units: army.units().clone(),
        });

        let mut audiences = vec![player_id];
        if target.player_id != player_id {
            audiences.push(target.player_id);
        }

        self.reports
            .add(
                &NewProjectedReport {
                    report_type: "reinforcement".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: source.player_id,
                    actor_village_id: Some(source_village_id),
                    target_player_id: Some(target.player_id),
                    target_village_id: Some(target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_merchants_arrived(
        &self,
        player_id: Uuid,
        source_village_id: u32,
        target_village_id: u32,
        resources: &ResourceGroup,
        merchants_used: u8,
    ) -> Result<(), CqrsError> {
        let Some(source) = self.try_village(source_village_id).await? else {
            return Ok(());
        };
        let Some(target) = self.try_village(target_village_id).await? else {
            return Ok(());
        };

        let payload = ReportPayload::MarketplaceDelivery(MarketplaceDeliveryReportPayload {
            sender_player: self.player_username(source.player_id).await?,
            sender_village: source.village_name.clone(),
            sender_position: source.position.clone(),
            receiver_player: self.player_username(target.player_id).await?,
            receiver_village: target.village_name.clone(),
            receiver_position: target.position.clone(),
            resources: resources.clone(),
            merchants_used,
        });

        let mut audiences = vec![player_id];
        if target.player_id != player_id {
            audiences.push(target.player_id);
        }

        self.reports
            .add(
                &NewProjectedReport {
                    report_type: "marketplace_delivery".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: source.player_id,
                    actor_village_id: Some(source_village_id),
                    target_player_id: Some(target.player_id),
                    target_village_id: Some(target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

impl EventConsumer for ReportProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }

        let domain_event = event.get_payload::<VillageEvent>()?;
        if let VillageEvent::ReinforcementArrived {
            player_id,
            source_village_id,
            target_village_id,
            army,
            ..
        } = &domain_event
        {
            return self
                .project_reinforcement_arrived(
                    *player_id,
                    *source_village_id,
                    *target_village_id,
                    army,
                )
                .await;
        }
        if let VillageEvent::MerchantsArrived {
            player_id,
            source_village_id,
            target_village_id,
            resources,
            merchants_used,
            ..
        } = &domain_event
        {
            return self
                .project_merchants_arrived(
                    *player_id,
                    *source_village_id,
                    *target_village_id,
                    resources,
                    *merchants_used,
                )
                .await;
        }
        if let VillageEvent::ScoutArrived {
            player_id,
            source_village_id,
            target_village_id,
            army_id,
            army,
            target,
            attack_type,
            ..
        } = &domain_event
        {
            let Some(source) = self.try_village(*source_village_id).await? else {
                return Ok(());
            };
            let Some(target_village) = self.try_village(*target_village_id).await? else {
                return Ok(());
            };

            let attacker_village = Self::village_from_model(&source);
            let defender_village = Self::village_from_model(&target_village);
            let attacker_army = Army::new(
                Some(*army_id),
                army.village_id,
                army.current_map_field_id,
                army.player_id,
                army.tribe.clone(),
                army.units(),
                army.smithy(),
                army.hero(),
            );
            let battle = Battle::new(
                attack_type.clone(),
                attacker_army,
                attacker_village.clone(),
                defender_village.clone(),
                None,
            );
            let battle_report = battle.calculate_scout_battle(target.clone());

            let attacker_payload = BattlePartyPayload {
                tribe: army.tribe.clone(),
                army_before: battle_report.attacker.army_before.units().clone(),
                survivors: battle_report.attacker.survivors.clone(),
                losses: battle_report.attacker.losses,
            };
            let scouting_success = battle_report.scouting.as_ref().is_some_and(|_| {
                battle_report
                    .attacker
                    .survivors
                    .units()
                    .iter()
                    .any(|&u| u > 0)
            });

            let payload = ReportPayload::Battle(BattleReportPayload {
                attack_type: attack_type.clone(),
                attacker_player: self.player_username(*player_id).await?,
                attacker_village: source.village_name.clone(),
                attacker_position: source.position.clone(),
                defender_player: self.player_username(target_village.player_id).await?,
                defender_village: target_village.village_name.clone(),
                defender_position: target_village.position.clone(),
                success: battle_report
                    .attacker
                    .survivors
                    .units()
                    .iter()
                    .any(|&u| u > 0),
                bounty: ResourceGroup::new(0, 0, 0, 0),
                attacker: Some(attacker_payload),
                defender: if scouting_success {
                    Some(BattlePartyPayload {
                        tribe: target_village.tribe.clone(),
                        army_before: target_village
                            .army
                            .as_ref()
                            .map(|a| a.units().clone())
                            .unwrap_or_default(),
                        survivors: target_village
                            .army
                            .as_ref()
                            .map(|a| a.units().clone())
                            .unwrap_or_default(),
                        losses: parabellum_types::army::TroopSet::default(),
                    })
                } else {
                    None
                },
                reinforcements: if scouting_success {
                    target_village
                        .reinforcements
                        .iter()
                        .map(|r| BattlePartyPayload {
                            tribe: r.tribe.clone(),
                            army_before: r.units().clone(),
                            survivors: r.units().clone(),
                            losses: parabellum_types::army::TroopSet::default(),
                        })
                        .collect()
                } else {
                    vec![]
                },
                scouting: battle_report.scouting.clone(),
                wall_damage: None,
                catapult_damage: vec![],
            });

            let mut audiences = vec![*player_id];
            if let Some(scouting) = &battle_report.scouting
                && scouting.was_detected
                && target_village.player_id != *player_id
            {
                audiences.push(target_village.player_id);
            }

            return self
                .reports
                .add(
                    &NewProjectedReport {
                        report_type: "battle".to_string(),
                        payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                        actor_player_id: *player_id,
                        actor_village_id: Some(*source_village_id),
                        target_player_id: Some(target_village.player_id),
                        target_village_id: Some(*target_village_id),
                    },
                    &audiences,
                )
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()));
        }

        let VillageEvent::AttackArrived {
            player_id,
            source_village_id,
            target_village_id,
            army_id,
            army,
            attack_type,
            catapult_targets,
            ..
        } = domain_event
        else {
            return Ok(());
        };

        let Some(source) = self.try_village(source_village_id).await? else {
            return Ok(());
        };
        let Some(target) = self.try_village(target_village_id).await? else {
            return Ok(());
        };

        let attacker_village = Self::village_from_model(&source);
        let defender_village = Self::village_from_model(&target);
        let attacker_army = Army::new(
            Some(army_id),
            army.village_id,
            army.current_map_field_id,
            army.player_id,
            army.tribe.clone(),
            army.units(),
            army.smithy(),
            army.hero(),
        );

        let mut selected_targets: Vec<Building> = Vec::new();
        for name in catapult_targets {
            match defender_village.get_building_by_name(&name) {
                Some(slot) => selected_targets.push(slot.building.clone()),
                None => {
                    if let Some(random) = defender_village.get_random_buildings(1).pop() {
                        selected_targets.push(random);
                    }
                }
            }
        }
        let selected_targets = selected_targets.try_into().ok();
        let battle = Battle::new(
            attack_type.clone(),
            attacker_army,
            attacker_village.clone(),
            defender_village,
            selected_targets,
        );
        let report = battle.calculate_battle();
        let bounty = report
            .bounty
            .clone()
            .unwrap_or_else(|| ResourceGroup::new(0, 0, 0, 0));
        let success = report
            .defender
            .as_ref()
            .map(|def| def.survivors.immensity() == 0)
            .unwrap_or(true);

        let attacker_payload = BattlePartyPayload {
            tribe: report.attacker.army_before.tribe.clone(),
            army_before: report.attacker.army_before.units().clone(),
            survivors: report.attacker.survivors,
            losses: report.attacker.losses,
        };
        let defender_payload = report.defender.as_ref().map(|def| BattlePartyPayload {
            tribe: def.army_before.tribe.clone(),
            army_before: def.army_before.units().clone(),
            survivors: def.survivors.clone(),
            losses: def.losses.clone(),
        });
        let reinforcements_payload: Vec<BattlePartyPayload> = report
            .reinforcements
            .iter()
            .map(|reinf| BattlePartyPayload {
                tribe: reinf.army_before.tribe.clone(),
                army_before: reinf.army_before.units().clone(),
                survivors: reinf.survivors.clone(),
                losses: reinf.losses.clone(),
            })
            .collect();
        let payload = ReportPayload::Battle(BattleReportPayload {
            attack_type,
            attacker_player: self.player_username(player_id).await?,
            attacker_village: source.village_name.clone(),
            attacker_position: source.position.clone(),
            defender_player: self.player_username(target.player_id).await?,
            defender_village: target.village_name.clone(),
            defender_position: target.position.clone(),
            success,
            bounty,
            attacker: Some(attacker_payload),
            defender: defender_payload,
            reinforcements: reinforcements_payload,
            scouting: report.scouting,
            wall_damage: report.wall_damage,
            catapult_damage: report.catapult_damage,
        });

        let mut audiences = vec![player_id];
        if target.player_id != player_id {
            audiences.push(target.player_id);
        }
        self.reports
            .add(
                &NewProjectedReport {
                    report_type: "battle".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: player_id,
                    actor_village_id: Some(source_village_id),
                    target_player_id: Some(target.player_id),
                    target_village_id: Some(target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        Ok(())
    }
}
