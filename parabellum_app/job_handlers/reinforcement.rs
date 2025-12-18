use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::models::army::Army;
use parabellum_types::{
    buildings::BuildingName,
    errors::ApplicationError,
    reports::{ReinforcementReportPayload, ReportPayload},
};

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::ReinforcementTask,
};
use crate::repository::{NewReport, ReportAudience};

pub struct ReinforcementJobHandler {
    payload: ReinforcementTask,
}

impl ReinforcementJobHandler {
    pub fn new(payload: ReinforcementTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ReinforcementJobHandler {
    #[instrument(skip_all, fields(
        task_type = "Reinforcement",
        army_id = %self.payload.army_id,
        target_village_id = %self.payload.village_id,
        player_id = %self.payload.player_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing Reinforcement job: Army arriving at village.");
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let hero_repo = ctx.uow.heroes();

        let mut target_village = village_repo
            .get_by_id(self.payload.village_id as u32)
            .await?;
        let mut reinforcement = army_repo.get_by_id(self.payload.army_id).await?;

        // To switch village, hero should be alone and target village should have HeroMansion
        if target_village.player_id == self.payload.player_id
            && reinforcement.units().immensity() == 0
            && target_village
                .get_building_by_name(&BuildingName::HeroMansion)
                .is_some()
        {
            if let Some(mut hero) = reinforcement.hero() {
                reinforcement.set_hero(None);
                hero.village_id = target_village.id;
                hero_repo.save(&hero).await?;
                army_repo.save(&reinforcement).await?;

                if let Some(garrison) = target_village.army() {
                    let mut home_army = garrison.clone();
                    home_army.set_hero(Some(hero.clone()));
                    target_village.set_army(Some(&home_army))?;
                    army_repo.remove(reinforcement.id).await?;
                    army_repo.save(&home_army).await?;
                } else {
                    let mut new_army = Army::new_village_army(&target_village);
                    new_army.set_hero(Some(hero.clone()));
                    army_repo.save(&new_army).await?;
                    target_village.set_army(Some(&new_army))?;
                }
            }
        } else {
            // Or everything goes into target village reinforcements (merge if same sender)
            let existing_idx = target_village.reinforcements().iter().position(|r| {
                r.player_id == reinforcement.player_id && r.village_id == reinforcement.village_id
            });

            reinforcement.current_map_field_id = Some(target_village.id);
            target_village.add_reinforcements(&reinforcement)?;

            // Persist merged or new reinforcement
            if let Some(idx) = existing_idx {
                let merged = target_village.reinforcements()[idx].clone();
                army_repo.save(&merged).await?;
                // Remove the incoming record to avoid duplicates
                if merged.id != reinforcement.id {
                    army_repo.remove(reinforcement.id).await?;
                }
            } else {
                army_repo.save(&reinforcement).await?;
            }
        }

        village_repo.save(&target_village).await?;

        info!(
            army_id = %reinforcement.id,
            new_location_id = %self.payload.village_id,
            "Army reinforcement has arrived and is now stationed at new location."
        );

        // Create reinforcement report
        let player_repo = ctx.uow.players();
        let report_repo = ctx.uow.reports();

        let sender_village = village_repo.get_by_id(reinforcement.village_id).await?;
        let sender_player = player_repo.get_by_id(sender_village.player_id).await?;
        let receiver_player = player_repo.get_by_id(target_village.player_id).await?;

        let reinforcement_payload = ReinforcementReportPayload {
            sender_player: sender_player.username.clone(),
            sender_village: sender_village.name.clone(),
            sender_position: sender_village.position.clone(),
            receiver_player: receiver_player.username.clone(),
            receiver_village: target_village.name.clone(),
            receiver_position: target_village.position.clone(),
            tribe: reinforcement.tribe.clone(),
            units: reinforcement.units().clone(),
        };

        let new_report = NewReport {
            report_type: "reinforcement".to_string(),
            payload: ReportPayload::Reinforcement(reinforcement_payload),
            actor_player_id: sender_village.player_id,
            actor_village_id: Some(sender_village.id),
            target_player_id: Some(target_village.player_id),
            target_village_id: Some(target_village.id),
        };

        let mut audiences = vec![ReportAudience {
            player_id: sender_village.player_id,
            read_at: None,
        }];

        // Only add receiver to audience if different from sender
        if target_village.player_id != sender_village.player_id {
            audiences.push(ReportAudience {
                player_id: target_village.player_id,
                read_at: None,
            });
        }

        report_repo.add(&new_report, &audiences).await?;

        Ok(())
    }
}
