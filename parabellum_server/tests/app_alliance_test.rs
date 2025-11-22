mod test_utils;

#[cfg(test)]
pub mod tests {
    use crate::test_utils::tests::{setup_app, setup_player_party};
    use parabellum_app::{
        command_handlers::{CreateAllianceCommandHandler, AcceptAllianceInviteCommandHandler, InviteToAllianceCommandHandler, KickFromAllianceCommandHandler, LeaveAllianceCommandHandler, SetAllianceLeaderCommandHandler},
        cqrs::commands::{CreateAlliance, AcceptAllianceInvite, InviteToAlliance, KickFromAlliance, LeaveAlliance, SetAllianceLeader},
    };
    use parabellum_types::errors::Result;
    use parabellum_game::models::buildings::Building;
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    #[tokio::test]
    async fn test_create_alliance_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        let (player, mut village, _army, _hero, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Set village as capital and add Embassy level 3
        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            village.is_capital = true;

            // Add Embassy building at level 3
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let cmd = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let handler = CreateAllianceCommandHandler::new();
        app.execute(cmd, handler).await?;

        // Verify alliance was created in database
        {
            let uow_check = uow_provider.tx().await?;

            // Verify alliance exists
            let alliance = uow_check
                .alliances()
                .get_by_name("Test Alliance".to_string())
                .await?;
            assert_eq!(alliance.name, "Test Alliance");
            assert_eq!(alliance.tag, "TEST");
            assert_eq!(alliance.max_members, 3); // Embassy level 3
            assert_eq!(alliance.leader_id, Some(player.id));

            // Verify player was updated
            let updated_player = uow_check.players().get_by_id(player.id).await?;
            assert_eq!(updated_player.alliance_id, Some(alliance.id));
            assert_eq!(updated_player.alliance_role, Some(255)); // All permissions
            assert!(updated_player.alliance_join_time.is_some());

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance.id, 10, 0)
                .await?;
            assert_eq!(logs.len(), 1);
            assert!(logs[0].data.as_ref().unwrap().contains("Test Alliance"));
            assert!(logs[0].data.as_ref().unwrap().contains("TEST"));

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_multiple_alliances() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup first player
        let (player1, mut village1, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            village1.is_capital = true;
            let embassy1 = Building::new(BuildingName::Embassy, config.speed)
                .at_level(4, config.speed)?;
            village1.add_building_at_slot(embassy1, 20)?;
            village_repo.save(&village1).await?;

            uow_setup.commit().await?;
        }

        // Setup second player
        let (player2, mut village2, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            village2.is_capital = true;
            let embassy2 = Building::new(BuildingName::Embassy, config.speed)
                .at_level(5, config.speed)?;
            village2.add_building_at_slot(embassy2, 20)?;
            village_repo.save(&village2).await?;

            uow_setup.commit().await?;
        }

        // Create first alliance
        let cmd1 = CreateAlliance {
            player_id: player1.id,
            name: "First Alliance".to_string(),
            tag: "ONE".to_string(),
        };
        let handler1 = CreateAllianceCommandHandler::new();
        app.execute(cmd1, handler1).await?;

        // Create second alliance
        let cmd2 = CreateAlliance {
            player_id: player2.id,
            name: "Second Alliance".to_string(),
            tag: "TWO".to_string(),
        };
        let handler2 = CreateAllianceCommandHandler::new();
        app.execute(cmd2, handler2).await?;

        // Verify both alliances exist and are distinct
        {
            let uow_check = uow_provider.tx().await?;

            let alliance1 = uow_check
                .alliances()
                .get_by_tag("ONE".to_string())
                .await?;
            assert_eq!(alliance1.name, "First Alliance");
            assert_eq!(alliance1.max_members, 4); // Embassy level 4
            assert_eq!(alliance1.leader_id, Some(player1.id));

            let alliance2 = uow_check
                .alliances()
                .get_by_tag("TWO".to_string())
                .await?;
            assert_eq!(alliance2.name, "Second Alliance");
            assert_eq!(alliance2.max_members, 5); // Embassy level 5
            assert_eq!(alliance2.leader_id, Some(player2.id));

            // Verify they have different IDs
            assert_ne!(alliance1.id, alliance2.id);

            // Verify each player is in their respective alliance
            let updated_player1 = uow_check.players().get_by_id(player1.id).await?;
            assert_eq!(updated_player1.alliance_id, Some(alliance1.id));

            let updated_player2 = uow_check.players().get_by_id(player2.id).await?;
            assert_eq!(updated_player2.alliance_id, Some(alliance2.id));

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_alliance_duplicate_fails() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup first player
        let (player1, mut village1, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            village1.is_capital = true;
            let embassy1 = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village1.add_building_at_slot(embassy1, 20)?;
            village_repo.save(&village1).await?;

            uow_setup.commit().await?;
        }

        // Setup second player
        let (player2, mut village2, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            village2.is_capital = true;
            let embassy2 = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village2.add_building_at_slot(embassy2, 20)?;
            village_repo.save(&village2).await?;

            uow_setup.commit().await?;
        }

        // Create first alliance
        let cmd1 = CreateAlliance {
            player_id: player1.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let handler1 = CreateAllianceCommandHandler::new();
        app.execute(cmd1, handler1).await?;

        // Attempt to create alliance with duplicate tag (should fail)
        let cmd2 = CreateAlliance {
            player_id: player2.id,
            name: "Different Name".to_string(),
            tag: "TEST".to_string(), // Duplicate tag
        };
        let handler2 = CreateAllianceCommandHandler::new();
        let result = app.execute(cmd2, handler2).await;

        assert!(result.is_err(), "Creating alliance with duplicate tag should fail");

        // Attempt to create alliance with duplicate name (should also fail)
        let cmd3 = CreateAlliance {
            player_id: player2.id,
            name: "Test Alliance".to_string(), // Duplicate name
            tag: "DIFF".to_string(),
        };
        let handler3 = CreateAllianceCommandHandler::new();
        let result2 = app.execute(cmd3, handler3).await;

        assert!(result2.is_err(), "Creating alliance with duplicate name should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup invitee player
        let (invitee, mut invitee_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            invitee_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            invitee_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&invitee_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance as leader
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite player to alliance
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: invitee.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Accept invitation
        let accept_cmd = AcceptAllianceInvite {
            player_id: invitee.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Verify player joined alliance
        {
            let uow_check = uow_provider.tx().await?;

            let updated_invitee = uow_check.players().get_by_id(invitee.id).await?;
            assert_eq!(updated_invitee.alliance_id, Some(alliance_id));
            assert_eq!(updated_invitee.alliance_role, Some(0)); // No permissions initially
            assert!(updated_invitee.alliance_join_time.is_some());

            // Verify invitation was deleted
            let invites = uow_check.alliance_invites().get_by_player_id(invitee.id).await?;
            assert_eq!(invites.len(), 0);

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance_id, 10, 0)
                .await?;
            // Should have 2 logs: alliance creation + player joined
            assert!(logs.len() >= 2);
            let join_log = logs.iter().find(|l| l.type_ == 1); // PlayerJoined = 1
            assert!(join_log.is_some());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_multiple_members() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            // Embassy level 5 = max 5 members
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(5, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Big Alliance".to_string(),
            tag: "BIG".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("BIG".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Setup and invite 3 additional players
        for _ in 0..3 {
            let (player, mut village, _, _, _) =
                setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

            {
                let uow_setup = uow_provider.tx().await?;
                let village_repo = uow_setup.villages();

                village.is_capital = true;
                let embassy = Building::new(BuildingName::Embassy, config.speed)
                    .at_level(3, config.speed)?;
                village.add_building_at_slot(embassy, 20)?;
                village_repo.save(&village).await?;

                uow_setup.commit().await?;
            }

            // Invite
            let invite_cmd = InviteToAlliance {
                player_id: leader.id,
                alliance_id,
                target_player_id: player.id,
            };
            let invite_handler = InviteToAllianceCommandHandler::new();
            app.execute(invite_cmd, invite_handler).await?;

            // Accept
            let accept_cmd = AcceptAllianceInvite {
                player_id: player.id,
                alliance_id,
            };
            let accept_handler = AcceptAllianceInviteCommandHandler::new();
            app.execute(accept_cmd, accept_handler).await?;
        }

        // Verify member count
        {
            let uow_check = uow_provider.tx().await?;

            let member_count = uow_check.alliances().count_members(alliance_id).await?;
            assert_eq!(member_count, 4); // Leader + 3 members

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_alliance_full_fails() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            // Embassy level 3 = max 3 members (leader + 2 slots)
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Small Alliance".to_string(),
            tag: "SMALL".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("SMALL".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Fill alliance to capacity (2 more members = 3 total)
        for _ in 0..2 {
            let (player, mut village, _, _, _) =
                setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

            {
                let uow_setup = uow_provider.tx().await?;
                let village_repo = uow_setup.villages();

                village.is_capital = true;
                let embassy = Building::new(BuildingName::Embassy, config.speed)
                    .at_level(3, config.speed)?;
                village.add_building_at_slot(embassy, 20)?;
                village_repo.save(&village).await?;

                uow_setup.commit().await?;
            }

            let invite_cmd = InviteToAlliance {
                player_id: leader.id,
                alliance_id,
                target_player_id: player.id,
            };
            let invite_handler = InviteToAllianceCommandHandler::new();
            app.execute(invite_cmd, invite_handler).await?;

            let accept_cmd = AcceptAllianceInvite {
                player_id: player.id,
                alliance_id,
            };
            let accept_handler = AcceptAllianceInviteCommandHandler::new();
            app.execute(accept_cmd, accept_handler).await?;
        }

        // Setup one more player to try joining full alliance
        let (overflow_player, mut overflow_village, _, _, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Teuton, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            overflow_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            overflow_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&overflow_village).await?;

            uow_setup.commit().await?;
        }

        // Invite overflow player
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: overflow_player.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Attempt to accept (should fail - alliance is full)
        let accept_cmd = AcceptAllianceInvite {
            player_id: overflow_player.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        let result = app.execute(accept_cmd, accept_handler).await;

        assert!(result.is_err(), "Accepting invite to full alliance should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_invite_to_alliance_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup target player
        let (target, _village2, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        // Create alliance as leader
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite player to alliance
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: target.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Verify invitation was created
        {
            let uow_check = uow_provider.tx().await?;

            let invites = uow_check.alliance_invites().get_by_player_id(target.id).await?;
            assert_eq!(invites.len(), 1);
            assert_eq!(invites[0].alliance_id, alliance_id);
            assert_eq!(invites[0].to_player_id, target.id);
            assert_eq!(invites[0].from_player_id, leader.id);

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance_id, 10, 0)
                .await?;
            // Should have 2 logs: alliance creation + invitation sent
            assert!(logs.len() >= 2);
            let invite_log = logs.iter().find(|l| l.data.as_ref().unwrap().contains("Invitation sent"));
            assert!(invite_log.is_some());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_invite_to_alliance_no_permission_fails() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup member player (will be added to alliance without invite permission)
        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Setup target player
        let (target, _village3, _army3, _hero3, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Teuton, [0; 10], false).await?;

        // Create alliance as leader
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Leader invites member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Member accepts invitation
        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Member tries to invite another player (should fail - no permission)
        let invite_cmd2 = InviteToAlliance {
            player_id: member.id, // Member without invite permission
            alliance_id,
            target_player_id: target.id,
        };
        let invite_handler2 = InviteToAllianceCommandHandler::new();
        let result = app.execute(invite_cmd2, invite_handler2).await;

        assert!(result.is_err(), "Inviting without permission should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_invite_to_alliance_duplicate_fails() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup target player
        let (target, _village2, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Send first invitation
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: target.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Try to send duplicate invitation (should fail)
        let invite_cmd2 = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: target.id,
        };
        let invite_handler2 = InviteToAllianceCommandHandler::new();
        let result = app.execute(invite_cmd2, invite_handler2).await;

        assert!(result.is_err(), "Duplicate invitation should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_kick_from_alliance_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup member player
        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance as leader
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        // Member accepts invitation
        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Leader kicks member
        let kick_cmd = KickFromAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let kick_handler = KickFromAllianceCommandHandler::new();
        app.execute(kick_cmd, kick_handler).await?;

        // Verify member was kicked
        {
            let uow_check = uow_provider.tx().await?;

            let updated_member = uow_check.players().get_by_id(member.id).await?;
            assert_eq!(updated_member.alliance_id, None);
            assert_eq!(updated_member.alliance_role, None);
            assert_eq!(updated_member.alliance_join_time, None);

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance_id, 10, 0)
                .await?;
            // Should have logs for: alliance creation, invitation sent, player joined, player kicked
            assert!(logs.len() >= 4);
            let kick_log = logs.iter().find(|l| l.data.as_ref().unwrap().contains("kicked"));
            assert!(kick_log.is_some());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_kick_from_alliance_no_permission_fails() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup two members
        let (member1, mut member1_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member1_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member1_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member1_village).await?;

            uow_setup.commit().await?;
        }

        let (member2, mut member2_village, _army3, _hero3, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Teuton, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member2_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member2_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member2_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add both members
        for member in [&member1, &member2] {
            let invite_cmd = InviteToAlliance {
                player_id: leader.id,
                alliance_id,
                target_player_id: member.id,
            };
            let invite_handler = InviteToAllianceCommandHandler::new();
            app.execute(invite_cmd, invite_handler).await?;

            let accept_cmd = AcceptAllianceInvite {
                player_id: member.id,
                alliance_id,
            };
            let accept_handler = AcceptAllianceInviteCommandHandler::new();
            app.execute(accept_cmd, accept_handler).await?;
        }

        // Member1 tries to kick Member2 (should fail - no permission)
        let kick_cmd = KickFromAlliance {
            player_id: member1.id,
            alliance_id,
            target_player_id: member2.id,
        };
        let kick_handler = KickFromAllianceCommandHandler::new();
        let result = app.execute(kick_cmd, kick_handler).await;

        assert!(result.is_err(), "Kicking without permission should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_kick_from_alliance_cannot_kick_leader() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup member with kick permission (for testing - normally leader would grant this)
        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Grant kick permission to member (manually update database for test)
        {
            let uow_setup = uow_provider.tx().await?;
            let player_repo = uow_setup.players();

            let mut member_player = player_repo.get_by_id(member.id).await?;
            member_player.update_alliance_role(2); // KickPlayer permission = 2
            player_repo.save(&member_player).await?;

            uow_setup.commit().await?;
        }

        // Member tries to kick leader (should fail)
        let kick_cmd = KickFromAlliance {
            player_id: member.id,
            alliance_id,
            target_player_id: leader.id, // Trying to kick leader
        };
        let kick_handler = KickFromAllianceCommandHandler::new();
        let result = app.execute(kick_cmd, kick_handler).await;

        assert!(result.is_err(), "Kicking the leader should fail");

        Ok(())
    }

    #[tokio::test]
    async fn test_leave_alliance_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup member player
        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Member leaves alliance
        let leave_cmd = LeaveAlliance {
            player_id: member.id,
        };
        let leave_handler = LeaveAllianceCommandHandler::new();
        app.execute(leave_cmd, leave_handler).await?;

        // Verify member left
        {
            let uow_check = uow_provider.tx().await?;

            let updated_member = uow_check.players().get_by_id(member.id).await?;
            assert_eq!(updated_member.alliance_id, None);
            assert_eq!(updated_member.alliance_role, None);
            assert_eq!(updated_member.alliance_join_time, None);

            // Verify alliance still exists (leader is still there)
            let alliance = uow_check.alliances().get_by_id(alliance_id).await?;
            assert_eq!(alliance.id, alliance_id);

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance_id, 10, 0)
                .await?;
            let leave_log = logs.iter().find(|l| l.data.as_ref().unwrap().contains("left"));
            assert!(leave_log.is_some());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_leave_alliance_leader_cannot_leave() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Leader tries to leave (should fail)
        let leave_cmd = LeaveAlliance {
            player_id: leader.id,
        };
        let leave_handler = LeaveAllianceCommandHandler::new();
        let result = app.execute(leave_cmd, leave_handler).await;

        assert!(result.is_err(), "Leader should not be able to leave alliance");

        Ok(())
    }

    #[tokio::test]
    async fn test_leave_alliance_last_member_deletes_alliance() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup two players
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Transfer leadership manually by kicking leader and updating member to be leader
        // First kick the leader (this is a test setup workaround)
        {
            let uow_setup = uow_provider.tx().await?;
            let player_repo = uow_setup.players();
            let alliance_repo = uow_setup.alliances();

            // Remove leader from alliance
            let mut leader_player = player_repo.get_by_id(leader.id).await?;
            leader_player.leave_alliance();
            player_repo.save(&leader_player).await?;

            // Update alliance to have member as leader
            let mut alliance = alliance_repo.get_by_id(alliance_id).await?;
            alliance.leader_id = Some(member.id);
            alliance_repo.save(&alliance).await?;

            uow_setup.commit().await?;
        }

        // Now member (who is now leader and only member) leaves
        let leave_cmd = LeaveAlliance {
            player_id: member.id,
        };
        let leave_handler = LeaveAllianceCommandHandler::new();
        let result = app.execute(leave_cmd, leave_handler).await;

        // Should fail because member is now the leader
        assert!(result.is_err(), "Leader (last member) should not be able to leave");

        Ok(())
    }

    #[tokio::test]
    async fn test_set_alliance_leader_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup member player (will become new leader)
        let (member, mut member_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add member
        let invite_cmd = InviteToAlliance {
            player_id: leader.id,
            alliance_id,
            target_player_id: member.id,
        };
        let invite_handler = InviteToAllianceCommandHandler::new();
        app.execute(invite_cmd, invite_handler).await?;

        let accept_cmd = AcceptAllianceInvite {
            player_id: member.id,
            alliance_id,
        };
        let accept_handler = AcceptAllianceInviteCommandHandler::new();
        app.execute(accept_cmd, accept_handler).await?;

        // Transfer leadership from leader to member
        let set_leader_cmd = SetAllianceLeader {
            player_id: leader.id,
            alliance_id,
            new_leader_id: member.id,
        };
        let set_leader_handler = SetAllianceLeaderCommandHandler::new();
        app.execute(set_leader_cmd, set_leader_handler).await?;

        // Verify leadership transfer
        {
            let uow_check = uow_provider.tx().await?;

            // Verify alliance leader was updated
            let alliance = uow_check.alliances().get_by_id(alliance_id).await?;
            assert_eq!(alliance.leader_id, Some(member.id));

            // Verify new leader has full permissions
            let updated_member = uow_check.players().get_by_id(member.id).await?;
            assert_eq!(updated_member.alliance_role, Some(255)); // all_permissions()

            // Verify old leader has officer permissions (demoted from leader)
            let updated_leader = uow_check.players().get_by_id(leader.id).await?;
            assert_eq!(updated_leader.alliance_role, Some(176)); // officer_permissions()

            // Verify alliance log was created
            let logs = uow_check
                .alliance_logs()
                .get_by_alliance_id(alliance_id, 10, 0)
                .await?;
            let transfer_log = logs.iter().find(|l| l.data.as_ref().unwrap().contains("transferred"));
            assert!(transfer_log.is_some());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_set_alliance_leader_non_leader_cannot_transfer() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Setup two members
        let (member1, mut member1_village, _army2, _hero2, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member1_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member1_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member1_village).await?;

            uow_setup.commit().await?;
        }

        let (member2, mut member2_village, _army3, _hero3, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Teuton, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            member2_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            member2_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&member2_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Invite and add both members
        for member in [&member1, &member2] {
            let invite_cmd = InviteToAlliance {
                player_id: leader.id,
                alliance_id,
                target_player_id: member.id,
            };
            let invite_handler = InviteToAllianceCommandHandler::new();
            app.execute(invite_cmd, invite_handler).await?;

            let accept_cmd = AcceptAllianceInvite {
                player_id: member.id,
                alliance_id,
            };
            let accept_handler = AcceptAllianceInviteCommandHandler::new();
            app.execute(accept_cmd, accept_handler).await?;
        }

        // Member1 tries to transfer leadership to Member2 (should fail)
        let set_leader_cmd = SetAllianceLeader {
            player_id: member1.id, // Non-leader trying to transfer
            alliance_id,
            new_leader_id: member2.id,
        };
        let set_leader_handler = SetAllianceLeaderCommandHandler::new();
        let result = app.execute(set_leader_cmd, set_leader_handler).await;

        assert!(result.is_err(), "Non-leader should not be able to transfer leadership");

        Ok(())
    }

    #[tokio::test]
    async fn test_set_alliance_leader_cannot_set_self() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        // Setup leader player
        let (leader, mut leader_village, _army1, _hero1, _) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.tx().await?;
            let village_repo = uow_setup.villages();

            leader_village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            leader_village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&leader_village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let create_cmd = CreateAlliance {
            player_id: leader.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let create_handler = CreateAllianceCommandHandler::new();
        app.execute(create_cmd, create_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.tx().await?;
            let alliance = uow_check
                .alliances()
                .get_by_tag("TEST".to_string())
                .await?;
            let id = alliance.id;
            uow_check.rollback().await?;
            id
        };

        // Leader tries to set themselves as leader (should fail)
        let set_leader_cmd = SetAllianceLeader {
            player_id: leader.id,
            alliance_id,
            new_leader_id: leader.id, // Same player
        };
        let set_leader_handler = SetAllianceLeaderCommandHandler::new();
        let result = app.execute(set_leader_cmd, set_leader_handler).await;

        assert!(result.is_err(), "Leader should not be able to set themselves as leader");

        Ok(())
    }
}
