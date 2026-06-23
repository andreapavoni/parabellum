//! Battle and scout report projection.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::projection_repositories::ProjectedReport;
use parabellum_game::battle::{BattlePartyReport, BattleReport};
use parabellum_types::army::TroopSet;
use parabellum_types::common::ResourceGroup;
use parabellum_types::reports::{BattlePartyPayload, BattleReportPayload, ReportPayload};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::es::consumers::report_projector::{ReportProjector, SourceTargetReportContext};

impl ReportProjector {
    pub(super) async fn project_battle_report_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::ScoutBattleResolved { .. } => Some(
                self.project_scout_battle_resolved(tx, projected_report_id, event)
                    .await,
            ),
            VillageEvent::AttackBattleResolved { .. } => Some(
                self.project_attack_battle_resolved(tx, projected_report_id, event)
                    .await,
            ),
            _ => None,
        }
    }

    async fn project_scout_battle_resolved(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::ScoutBattleResolved {
            player_id,
            source_village_id,
            target_village_id,
            report,
            ..
        } = event
        else {
            unreachable!("project_scout_battle_resolved called with non-ScoutBattleResolved event");
        };
        let Some(context) = self
            .source_target_context_in_tx(tx, *source_village_id, *target_village_id)
            .await?
        else {
            return Ok(());
        };
        let payload =
            scout_battle_payload(report, &context, self.player_username(*player_id).await?);
        let audiences = scout_battle_audiences(*player_id, context.target.player_id, report);
        self.reports
            .add_projected_in_tx(
                tx,
                &ProjectedReport {
                    id: projected_report_id,
                    report_type: "battle".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: *player_id,
                    actor_village_id: Some(*source_village_id),
                    target_player_id: Some(context.target.player_id),
                    target_village_id: Some(*target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }

    async fn project_attack_battle_resolved(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projected_report_id: Uuid,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::AttackBattleResolved {
            player_id,
            source_village_id,
            target_village_id,
            report,
            ..
        } = event
        else {
            unreachable!(
                "project_attack_battle_resolved called with non-AttackBattleResolved event"
            );
        };
        let Some(context) = self
            .source_target_context_in_tx(tx, *source_village_id, *target_village_id)
            .await?
        else {
            return Ok(());
        };
        let payload =
            attack_battle_payload(report, &context, self.player_username(*player_id).await?);
        let audiences = attack_battle_audiences(*player_id, context.target.player_id, report);
        self.reports
            .add_projected_in_tx(
                tx,
                &ProjectedReport {
                    id: projected_report_id,
                    report_type: "battle".to_string(),
                    payload: serde_json::to_value(payload).map_err(CqrsError::Serialization)?,
                    actor_player_id: *player_id,
                    actor_village_id: Some(*source_village_id),
                    target_player_id: Some(context.target.player_id),
                    target_village_id: Some(*target_village_id),
                },
                &audiences,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}

fn battle_party_payload(report: &BattlePartyReport) -> BattlePartyPayload {
    BattlePartyPayload {
        tribe: report.army_before.tribe.clone(),
        army_before: report.army_before.units().clone(),
        survivors: report.survivors.clone(),
        losses: report.losses.clone(),
        has_hero: report.army_before.hero().is_some(),
    }
}

fn scout_battle_payload(
    report: &BattleReport,
    context: &SourceTargetReportContext,
    attacker_player: String,
) -> ReportPayload {
    let scouting_success = report
        .scouting
        .as_ref()
        .is_some_and(|_| report.attacker.survivors.units().iter().any(|&u| u > 0));

    ReportPayload::Battle(BattleReportPayload {
        attack_type: report.attack_type.clone(),
        attacker_player,
        attacker_village: context.source.village_name.clone(),
        attacker_position: context.source.position.clone(),
        defender_player: context.target_player.clone(),
        defender_village: context.target.village_name.clone(),
        defender_position: context.target.position.clone(),
        success: report.attacker.survivors.units().iter().any(|&u| u > 0),
        bounty: ResourceGroup::new(0, 0, 0, 0),
        attacker: Some(battle_party_payload(&report.attacker)),
        defender: if scouting_success {
            Some(BattlePartyPayload {
                tribe: context.target.tribe.clone(),
                army_before: context
                    .target_home_army
                    .as_ref()
                    .map(|a| a.units().clone())
                    .unwrap_or_default(),
                survivors: context
                    .target_home_army
                    .as_ref()
                    .map(|a| a.units().clone())
                    .unwrap_or_default(),
                losses: TroopSet::default(),
                has_hero: context
                    .target_home_army
                    .as_ref()
                    .is_some_and(|army| army.hero().is_some()),
            })
        } else {
            None
        },
        reinforcements: if scouting_success {
            context
                .target_reinforcements
                .iter()
                .map(|r| BattlePartyPayload {
                    tribe: r.tribe.clone(),
                    army_before: r.units().clone(),
                    survivors: r.units().clone(),
                    losses: TroopSet::default(),
                    has_hero: r.hero().is_some(),
                })
                .collect()
        } else {
            vec![]
        },
        scouting: if scouting_success {
            report.scouting.clone()
        } else {
            None
        },
        wall_damage: None,
        catapult_damage: vec![],
        loyalty_before: None,
        loyalty_after: None,
        conquered: None,
        trapped: None,
        freed: None,
    })
}

fn scout_battle_audiences(
    attacker_player_id: Uuid,
    target_player_id: Uuid,
    report: &BattleReport,
) -> Vec<Uuid> {
    let mut audiences = vec![attacker_player_id];
    if let Some(scouting) = &report.scouting
        && scouting.was_detected
        && target_player_id != attacker_player_id
    {
        audiences.push(target_player_id);
    }
    audiences
}

fn attack_battle_payload(
    report: &BattleReport,
    context: &SourceTargetReportContext,
    attacker_player: String,
) -> ReportPayload {
    let success = attack_battle_success(report);

    ReportPayload::Battle(BattleReportPayload {
        attack_type: report.attack_type.clone(),
        attacker_player,
        attacker_village: context.source.village_name.clone(),
        attacker_position: context.source.position.clone(),
        defender_player: context.target_player.clone(),
        defender_village: context.target.village_name.clone(),
        defender_position: context.target.position.clone(),
        success,
        bounty: report
            .bounty
            .clone()
            .unwrap_or_else(|| ResourceGroup::new(0, 0, 0, 0)),
        attacker: Some(battle_party_payload(&report.attacker)),
        defender: report.defender.as_ref().map(battle_party_payload),
        reinforcements: report
            .reinforcements
            .iter()
            .map(battle_party_payload)
            .collect(),
        scouting: report.scouting.clone(),
        wall_damage: report.wall_damage.clone(),
        catapult_damage: report.catapult_damage.clone(),
        loyalty_before: Some(report.loyalty_before),
        loyalty_after: Some(report.loyalty_after),
        conquered: Some(success && report.loyalty_after == 0),
        trapped: report.trapped.as_ref().map(|trapped| {
            parabellum_types::reports::TrapCapturePayload {
                trapped_units: trapped.trapped_units.clone(),
                traps_used: trapped.traps_used,
            }
        }),
        freed: report
            .freed
            .as_ref()
            .map(|freed| parabellum_types::reports::TrapFreePayload {
                units_before: freed.units_before.clone(),
                deaths: freed.deaths.clone(),
                survivors: freed.survivors.clone(),
                traps_destroyed: freed.traps_destroyed,
            }),
    })
}

fn attack_battle_success(report: &BattleReport) -> bool {
    let attacker_survivors = report.attacker.survivors.immensity();
    let defender_survivors = report
        .defender
        .as_ref()
        .map(|def| def.survivors.immensity())
        .unwrap_or(0);
    let reinforcements_survivors: u32 = report
        .reinforcements
        .iter()
        .map(|reinf| reinf.survivors.immensity())
        .sum();

    attacker_survivors > 0 && defender_survivors + reinforcements_survivors == 0
}

fn attack_battle_audiences(
    attacker_player_id: Uuid,
    target_player_id: Uuid,
    report: &BattleReport,
) -> Vec<Uuid> {
    let mut audiences = ReportProjector::audience_with_target(attacker_player_id, target_player_id);
    for reinforcement in &report.reinforcements {
        let owner = reinforcement.army_before.player_id;
        if !audiences.contains(&owner) {
            audiences.push(owner);
        }
    }
    audiences
}
