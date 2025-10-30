use crate::{
    game::{
        battle::{AttackType, Battle},
        models::{buildings::Building, ResourceGroup},
    },
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        tasks::{ArmyReturnTask, AttackTask},
        Job, JobTask,
    },
};
use anyhow::Result;
use async_trait::async_trait;

pub struct AttackJobHandler {
    payload: AttackTask,
}

impl AttackJobHandler {
    pub fn new(payload: AttackTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for AttackJobHandler {
    async fn handle<'ctx, 'a>(&'ctx self, ctx: &'ctx JobHandlerContext<'a>) -> Result<()> {
        println!("Execute Attack Job for army {}", self.payload.army_id);

        // 1. Hydrate necessary data from db
        let mut attacker_army = ctx
            .uow
            .armies()
            .get_by_id(self.payload.army_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attacker army not found"))?;

        let attacker_village = ctx
            .uow
            .villages()
            .get_by_id(attacker_army.village_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attacker village not found"))?;

        let mut defender_village = ctx
            .uow
            .villages()
            .get_by_id(self.payload.target_village_id as u32)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Defender village not found"))?;

        // 2. Execute domain logic

        // Find catapult targets on target village by looking for their name, or return random buildings
        let mut catapult_targets: Vec<Building> = Vec::new();

        for ct in &self.payload.catapult_targets {
            match defender_village.get_building_by_name(ct.clone()) {
                Some(b) => catapult_targets.push(b.building.clone()),
                None => {
                    let b = defender_village.get_random_buildings(1).pop().unwrap();
                    catapult_targets.push(b.clone())
                }
            }
        }

        let catapult_targets: [Building; 2] = catapult_targets.try_into().unwrap();

        let battle = Battle::new(
            AttackType::Normal,
            attacker_army.clone(),
            attacker_village.clone(),
            defender_village.clone(),
            Some(catapult_targets),
        );
        let battle_report = battle.calculate_battle();

        // 3. Store results on db
        attacker_army.update_units(&battle_report.attacker.survivors);
        ctx.uow.armies().save(&attacker_army).await?; // Salva l'esercito attaccante aggiornato

        // 3.2 Applies changes to defender village
        if let Some(bounty) = &battle_report.bounty {
            defender_village.stocks.remove_resources(bounty);
        }
        defender_village.loyalty = battle_report.loyalty_after;

        // Apply damages to buildings
        defender_village.apply_building_damages(&battle_report)?;

        // Applies combat losses to defender village and its reinforcements
        defender_village.apply_battle_losses(&battle_report);

        // Update village state
        defender_village.update_state();
        ctx.uow.villages().save(&defender_village).await?;

        // Update armies
        if let Some(army) = defender_village.army {
            ctx.uow.armies().save(&army).await?;
        }

        for reinforcement_army in defender_village.reinforcements {
            ctx.uow.armies().save(&reinforcement_army).await?;
        }

        // --- 4. Return job ---
        let return_travel_time = attacker_village
            .position
            .calculate_travel_time_secs(defender_village.position, attacker_army.speed())
            as i64;

        let player_id = attacker_village.player_id;
        let village_id = attacker_village.id as i32;
        let defender_village_id = defender_village.id as i32;

        let return_payload = ArmyReturnTask {
            army_id: attacker_army.id, // Attacking army ID
            resources: battle_report
                .bounty
                .unwrap_or(ResourceGroup::new(0, 0, 0, 0)), // Resources to bring back
            destination_player_id: player_id,
            destination_village_id: village_id,
            from_village_id: defender_village_id,
        };

        let return_job = Job::new(
            player_id,
            village_id,
            return_travel_time,
            JobTask::ArmyReturn(return_payload),
        );

        ctx.uow.jobs().add(&return_job).await?;

        println!(
            "Army return job {} planned. Will arrive at {}.",
            return_job.id, return_job.completed_at
        );

        // 5. Mark job as completed
        // For now we do it in the worker above
        // Or:
        // ctx.uow.jobs().mark_as_completed(self.job_id).await?;

        Ok(())
    }
}
