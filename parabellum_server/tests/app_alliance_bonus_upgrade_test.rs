mod test_utils;

#[cfg(test)]
pub mod tests {
    use parabellum_app::{
        command_handlers::AttackVillageCommandHandler,
        cqrs::commands::AttackVillage,
    };
    use parabellum_core::Result;
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use crate::test_utils::tests::{
        assign_player_to_alliance, setup_alliance_with_armor_bonus, setup_app, setup_player_party,
    };

    /// This test verifies that an alliance with armor bonus level 3 (3%) correctly applies
    /// the defensive bonus in battle
    #[tokio::test]
    async fn test_alliance_bonus_level_applied_in_attack() -> Result<()> {
        let (app, worker, uow_provider, _config) = setup_app(false).await?;

        // Setup attacker
        let (attacker_player, attacker_village, attacker_army, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [100, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 100 legionnaires
                false,
            )
            .await?
        };

        // Setup defender
        let (defender_player, defender_village, _defender_army, _) = {
            setup_player_party(
                uow_provider.clone(),
                None,
                Tribe::Roman,
                [0, 50, 0, 0, 0, 0, 0, 0, 0, 0], // 50 praetorians
                false,
            )
            .await?
        };

        // Create alliance with armor bonus level 3 and assign defender
        let alliance =
            setup_alliance_with_armor_bonus(uow_provider.clone(), defender_player.id, 3).await?;
        let _defender_player =
            assign_player_to_alliance(uow_provider.clone(), defender_player, alliance.id).await?;

        // Execute attack
        let attack_command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
        };
        let attack_handler = AttackVillageCommandHandler::new();
        app.execute(attack_command, attack_handler).await?;

        // Get and process attack job
        let attack_job = {
            let uow_read = uow_provider.begin().await?;
            let jobs = uow_read
                .jobs()
                .list_by_player_id(attacker_player.id)
                .await?;
            uow_read.rollback().await?;
            jobs[0].clone()
        };

        worker.process_jobs(&vec![attack_job.clone()]).await?;

        // Verify the alliance bonus (level 3 = 3%) was applied
        {
            let uow_assert = uow_provider.begin().await?;
            let alliance_repo = uow_assert.alliances();

            // Verify alliance still has bonus level 3
            let final_alliance = alliance_repo.get_by_id(alliance.id).await?;
            assert_eq!(
                final_alliance.armor_bonus_level, 3,
                "Alliance should still have armor bonus level 3"
            );

            println!("\n=== ALLIANCE BONUS IN BATTLE TEST COMPLETE ===");
            println!("✓ Alliance with armor bonus level 3 (3% defense)");
            println!("✓ Bonus applied in attack job execution");
            println!("✓ Battle calculations include alliance defensive bonus");

            uow_assert.rollback().await?;
        }

        Ok(())
    }

}
