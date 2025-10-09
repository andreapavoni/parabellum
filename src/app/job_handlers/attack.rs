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
    async fn handle(&self, ctx: &JobHandlerContext) -> Result<()> {
        println!("Execute Attack Job for army {}", self.payload.army_id);

        // 1. Hydrate necessary data from db
        let attacker_army = ctx
            .army_repo
            .get_by_id(self.payload.army_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attacker army not found"))?;

        let attacker_village = ctx
            .village_repo
            .get_by_id(attacker_army.village_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attacker village not found"))?;

        let defender_village = ctx
            .village_repo
            .get_by_id(self.payload.target_village_id as u32)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Defender village not found"))?;

        // 2. Execute domain logic

        // Find catapult targets on target village by looking for their name, or return random buildings
        let mut catapult_targets: Vec<Building> = Vec::new();

        for ct in &self.payload.catapult_targets {
            match defender_village.get_building_by_name(ct.clone()) {
                Some(b) => catapult_targets.push(b.clone()),
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
            catapult_targets,
        );
        let _battle_result = battle.calculate_battle();

        // 3. Store results on db
        // ctx.village_repo.apply_damages(..., battle_result.buildings_damages).await?;
        // ctx.army_repo.apply_losses(..., battle_result.attacker_loss_percentage).await?;
        // ctx.job_repo.create(return_army_new_job).await?;

        // --- 4. Army return job ---
        // Calculate army speed
        let return_travel_time = attacker_village
            .calculate_travel_time_secs(defender_village.position, attacker_army.clone().speed())
            as i64;

        // Creates army return payload
        let player_id = attacker_village.clone().player_id;
        let village_id = attacker_village.id as i32;
        let defender_village_id = defender_village.id as i32;

        let return_payload = ArmyReturnTask {
            army_id: self.payload.army_id,
            // TODO: fix loot from battle
            resources: ResourceGroup::new(0, 0, 0, 0),
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

        ctx.job_repo.add(&return_job).await?;

        println!(
            "Army return job {} planned. Will arrive at {}.",
            return_job.id, return_job.completed_at
        );

        Ok(())
    }
}
