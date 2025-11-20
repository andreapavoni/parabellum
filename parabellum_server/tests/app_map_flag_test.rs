mod test_utils;

#[cfg(test)]
pub mod tests {
    use crate::test_utils::tests::{setup_app, setup_player_party};
    use parabellum_app::{
        command_handlers::{
            CreateCustomFlagCommandHandler, CreateMultiMarkCommandHandler,
            UpdateMapFlagCommandHandler, DeleteMapFlagCommandHandler,
            CreateAllianceCommandHandler,
        },
        cqrs::commands::{
            CreateCustomFlag, CreateMultiMark, UpdateMapFlag, DeleteMapFlag,
            CreateAlliance,
        },
    };
    use parabellum_core::Result;
    use parabellum_game::models::{buildings::Building, map_flag::MapFlagType};
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    #[tokio::test]
    async fn test_create_player_custom_flag_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Create a custom flag
        let cmd = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 5,
            text: "My Custom Flag".to_string(),
        };
        let handler = CreateCustomFlagCommandHandler::new();
        app.execute(cmd, handler).await?;

        // Verify flag was created in database
        {
            let uow_check = uow_provider.begin().await?;

            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 1);
            assert_eq!(flags[0].player_id, Some(player.id));
            assert_eq!(flags[0].alliance_id, None);
            assert_eq!(flags[0].x, Some(100));
            assert_eq!(flags[0].y, Some(50));
            assert_eq!(flags[0].color, 5);
            assert_eq!(flags[0].text, Some("My Custom Flag".to_string()));
            assert_eq!(flags[0].flag_type, MapFlagType::CustomFlag.as_i16());
            assert_eq!(flags[0].created_by, player.id);

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_player_mark_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        let (target_player, _village2, _army2, _hero2) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        // Create a player mark
        let cmd = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_player.id,
            mark_type: 0, // PlayerMark
            color: 3,
        };
        let handler = CreateMultiMarkCommandHandler::new();
        app.execute(cmd, handler).await?;

        // Verify mark was created in database
        {
            let uow_check = uow_provider.begin().await?;

            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 1);
            assert_eq!(flags[0].player_id, Some(player.id));
            assert_eq!(flags[0].target_id, Some(target_player.id));
            assert_eq!(flags[0].color, 3);
            assert_eq!(flags[0].flag_type, MapFlagType::PlayerMark.as_i16());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_update_map_flag_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Create a custom flag
        let cmd = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 5,
            text: "Original Text".to_string(),
        };
        let handler = CreateCustomFlagCommandHandler::new();
        app.execute(cmd, handler).await?;

        // Get the flag ID
        let flag_id = {
            let uow_check = uow_provider.begin().await?;
            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            let id = flags[0].id;
            uow_check.rollback().await?;
            id
        };

        // Update the flag
        let update_cmd = UpdateMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id,
            color: 8,
            text: Some("Updated Text".to_string()),
        };
        let update_handler = UpdateMapFlagCommandHandler::new();
        app.execute(update_cmd, update_handler).await?;

        // Verify flag was updated in database
        {
            let uow_check = uow_provider.begin().await?;

            let updated_flag = uow_check.map_flags().get_by_id(flag_id).await?;
            assert_eq!(updated_flag.color, 8);
            assert_eq!(updated_flag.text, Some("Updated Text".to_string()));

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_map_flag_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Create a custom flag
        let cmd = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 5,
            text: "To Be Deleted".to_string(),
        };
        let handler = CreateCustomFlagCommandHandler::new();
        app.execute(cmd, handler).await?;

        // Get the flag ID
        let flag_id = {
            let uow_check = uow_provider.begin().await?;
            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 1);
            let id = flags[0].id;
            uow_check.rollback().await?;
            id
        };

        // Delete the flag
        let delete_cmd = DeleteMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id,
        };
        let delete_handler = DeleteMapFlagCommandHandler::new();
        app.execute(delete_cmd, delete_handler).await?;

        // Verify flag was deleted from database
        {
            let uow_check = uow_provider.begin().await?;

            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 0);

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_alliance_custom_flag_with_permissions_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        let (player, mut village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Set up alliance
        {
            let uow_setup = uow_provider.begin().await?;
            let village_repo = uow_setup.villages();

            village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let alliance_cmd = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let alliance_handler = CreateAllianceCommandHandler::new();
        app.execute(alliance_cmd, alliance_handler).await?;

        // Get alliance ID
        let alliance_id = {
            let uow_check = uow_provider.begin().await?;
            let updated_player = uow_check.players().get_by_id(player.id).await?;
            let id = updated_player.alliance_id.unwrap();
            uow_check.rollback().await?;
            id
        };

        // Create an alliance custom flag
        let flag_cmd = CreateCustomFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            x: 200,
            y: -100,
            color: 15,
            text: "Alliance Flag".to_string(),
        };
        let flag_handler = CreateCustomFlagCommandHandler::new();
        app.execute(flag_cmd, flag_handler).await?;

        // Verify alliance flag was created in database
        {
            let uow_check = uow_provider.begin().await?;

            let flags = uow_check.map_flags().get_by_alliance_id(alliance_id).await?;
            assert_eq!(flags.len(), 1);
            assert_eq!(flags[0].alliance_id, Some(alliance_id));
            assert_eq!(flags[0].player_id, None);
            assert_eq!(flags[0].color, 15);
            assert_eq!(flags[0].text, Some("Alliance Flag".to_string()));
            assert_eq!(flags[0].created_by, player.id);

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_alliance_mark_full_flow() -> Result<()> {
        let (app, _worker, uow_provider, config) = setup_app(false).await?;

        let (player, mut village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Set up alliance
        {
            let uow_setup = uow_provider.begin().await?;
            let village_repo = uow_setup.villages();

            village.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village.add_building_at_slot(embassy, 20)?;
            village_repo.save(&village).await?;

            uow_setup.commit().await?;
        }

        // Create alliance
        let alliance_cmd = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };
        let alliance_handler = CreateAllianceCommandHandler::new();
        app.execute(alliance_cmd, alliance_handler).await?;

        // Get alliance ID
        let player_alliance_id = {
            let uow_check = uow_provider.begin().await?;
            let updated_player = uow_check.players().get_by_id(player.id).await?;
            let id = updated_player.alliance_id.unwrap();
            uow_check.rollback().await?;
            id
        };

        // Create another alliance to mark
        let (player2, mut village2, _army2, _hero2) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        {
            let uow_setup = uow_provider.begin().await?;
            let village_repo = uow_setup.villages();

            village2.is_capital = true;
            let embassy = Building::new(BuildingName::Embassy, config.speed)
                .at_level(3, config.speed)?;
            village2.add_building_at_slot(embassy, 20)?;
            village_repo.save(&village2).await?;

            uow_setup.commit().await?;
        }

        let target_alliance_cmd = CreateAlliance {
            player_id: player2.id,
            name: "Target Alliance".to_string(),
            tag: "TRGT".to_string(),
        };
        let target_alliance_handler = CreateAllianceCommandHandler::new();
        app.execute(target_alliance_cmd, target_alliance_handler).await?;

        let target_alliance_id = {
            let uow_check = uow_provider.begin().await?;
            let updated_player = uow_check.players().get_by_id(player2.id).await?;
            let id = updated_player.alliance_id.unwrap();
            uow_check.rollback().await?;
            id
        };

        // Create an alliance mark
        let mark_cmd = CreateMultiMark {
            player_id: player.id,
            alliance_id: Some(player_alliance_id),
            target_id: target_alliance_id,
            mark_type: 1, // AllianceMark
            color: 7,
        };
        let mark_handler = CreateMultiMarkCommandHandler::new();
        app.execute(mark_cmd, mark_handler).await?;

        // Verify alliance mark was created in database
        {
            let uow_check = uow_provider.begin().await?;

            let flags = uow_check.map_flags().get_by_alliance_id(player_alliance_id).await?;
            assert_eq!(flags.len(), 1);
            assert_eq!(flags[0].alliance_id, Some(player_alliance_id));
            assert_eq!(flags[0].target_id, Some(target_alliance_id));
            assert_eq!(flags[0].color, 7);
            assert_eq!(flags[0].flag_type, MapFlagType::AllianceMark.as_i16());

            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_flag_limit_enforcement() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Create 5 custom flags (the limit)
        for i in 0..5 {
            let cmd = CreateCustomFlag {
                player_id: player.id,
                alliance_id: None,
                x: i * 10,
                y: i * 10,
                color: 5,
                text: format!("Flag {}", i),
            };
            let handler = CreateCustomFlagCommandHandler::new();
            app.execute(cmd, handler).await?;
        }

        // Try to create a 6th flag (should fail)
        let cmd = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 100,
            color: 5,
            text: "Sixth Flag".to_string(),
        };
        let handler = CreateCustomFlagCommandHandler::new();
        let result = app.execute(cmd, handler).await;

        assert!(result.is_err());

        // Verify only 5 flags exist in database
        {
            let uow_check = uow_provider.begin().await?;
            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 5);
            uow_check.rollback().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_multi_mark_limit_enforcement() -> Result<()> {
        let (app, _worker, uow_provider, _config) = setup_app(false).await?;

        let (player, _village, _army, _hero) =
            setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

        // Create 10 player marks (the limit)
        for _ in 0..10 {
            let (target_player, _v, _a, _h) =
                setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

            let cmd = CreateMultiMark {
                player_id: player.id,
                alliance_id: None,
                target_id: target_player.id,
                mark_type: 0,
                color: 3,
            };
            let handler = CreateMultiMarkCommandHandler::new();
            app.execute(cmd, handler).await?;
        }

        // Try to create an 11th mark (should fail)
        let (target_player, _v, _a, _h) =
            setup_player_party(uow_provider.clone(), None, Tribe::Gaul, [0; 10], false).await?;

        let cmd = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_player.id,
            mark_type: 0,
            color: 3,
        };
        let handler = CreateMultiMarkCommandHandler::new();
        let result = app.execute(cmd, handler).await;

        assert!(result.is_err());

        // Verify only 10 marks exist in database
        {
            let uow_check = uow_provider.begin().await?;
            let flags = uow_check.map_flags().get_by_player_id(player.id).await?;
            assert_eq!(flags.len(), 10);
            uow_check.rollback().await?;
        }

        Ok(())
    }
}
