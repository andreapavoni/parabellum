mod test_utils;

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use parabellum_app::{
        app::AppBus,
        command_handlers::{
            AcceptMarketplaceOfferCommandHandler, CancelMarketplaceOfferCommandHandler,
            CreateMarketplaceOfferCommandHandler, SendResourcesCommandHandler,
        },
        config::Config,
        cqrs::commands::{
            AcceptMarketplaceOffer, CancelMarketplaceOffer, CreateMarketplaceOffer, SendResources,
        },
        jobs::worker::JobWorker,
        test_utils::tests::set_village_resources,
        uow::UnitOfWorkProvider,
    };
    use parabellum_game::models::{buildings::Building, village::Village};
    use parabellum_types::{
        Result,
        army::TroopSet,
        common::{Player, ResourceGroup},
        reports::ReportPayload,
        tribe::Tribe,
    };

    use crate::test_utils::tests::{setup_app, setup_player_party};

    async fn setup_marketplace_env() -> Result<
        (
            Arc<dyn UnitOfWorkProvider>,
            Arc<Config>,
            AppBus,
            Arc<JobWorker>,
            Player,
            Village,
            Player,
            Village,
        ),
        parabellum_types::errors::ApplicationError,
    > {
        let (app, worker, uow_provider, config) = setup_app(false).await?;

        let (player_a, mut village_a, _, _, _) = setup_player_party(
            uow_provider.clone(),
            None,
            Tribe::Gaul,
            TroopSet::default(),
            false,
        )
        .await?;
        let (player_b, mut village_b, _, _, _) = setup_player_party(
            uow_provider.clone(),
            None,
            Tribe::Roman,
            TroopSet::default(),
            false,
        )
        .await?;

        let uow = uow_provider.tx().await?;

        let granary = Building::new(
            parabellum_types::buildings::BuildingName::Granary,
            config.speed,
        )
        .at_level(10, config.speed)?;
        let warehouse = Building::new(
            parabellum_types::buildings::BuildingName::Warehouse,
            config.speed,
        )
        .at_level(10, config.speed)?;
        let marketplace = Building::new(
            parabellum_types::buildings::BuildingName::Marketplace,
            config.speed,
        )
        .at_level(10, config.speed)?;

        village_a.add_building_at_slot(granary.clone(), 23)?;
        village_a.add_building_at_slot(warehouse.clone(), 24)?;
        village_a.add_building_at_slot(marketplace.clone(), 25)?;
        set_village_resources(&mut village_a, ResourceGroup(5000, 5000, 5000, 5000));

        village_b.add_building_at_slot(granary, 23)?;
        village_b.add_building_at_slot(warehouse, 24)?;
        village_b.add_building_at_slot(marketplace, 25)?;
        set_village_resources(&mut village_b, ResourceGroup(5000, 5000, 5000, 5000));

        uow.villages().save(&village_a).await?;
        uow.villages().save(&village_b).await?;
        uow.commit().await?;

        Ok((
            uow_provider,
            config,
            app,
            worker,
            player_a,
            village_a,
            player_b,
            village_b,
        ))
    }

    #[tokio::test]
    async fn test_marketplace_offer_accept_creates_reports() -> Result<()> {
        let (uow_provider, _config, app, worker, player_a, village_a, player_b, village_b) =
            setup_marketplace_env().await?;

        let offer_resources = ResourceGroup(1200, 0, 0, 0);
        let seek_resources = ResourceGroup(0, 600, 0, 0);

        app.execute(
            CreateMarketplaceOffer {
                village_id: village_a.id,
                offer_resources: offer_resources.clone(),
                seek_resources: seek_resources.clone(),
            },
            CreateMarketplaceOfferCommandHandler::new(),
        )
        .await?;

        let offer_id = {
            let uow = uow_provider.tx().await?;
            let offers = uow.marketplace().list_by_village(village_a.id).await?;
            assert_eq!(offers.len(), 1);
            offers[0].id
        };

        app.execute(
            AcceptMarketplaceOffer {
                player_id: player_b.id,
                village_id: village_b.id,
                offer_id,
            },
            AcceptMarketplaceOfferCommandHandler::new(),
        )
        .await?;

        let (going_jobs, initial_a, initial_b) = {
            let uow = uow_provider.tx().await?;

            let all_offers = uow.marketplace().list_all().await?;
            assert!(
                all_offers.iter().all(|offer| offer.id != offer_id),
                "Offer should be removed after acceptance"
            );

            let offerer_jobs = uow.jobs().list_by_player_id(player_a.id).await?;
            let acceptor_jobs = uow.jobs().list_by_player_id(player_b.id).await?;
            assert_eq!(offerer_jobs.len(), 1);
            assert_eq!(acceptor_jobs.len(), 1);

            let village_a_state = uow.villages().get_by_id(village_a.id).await?;
            let village_b_state = uow.villages().get_by_id(village_b.id).await?;

            let mut jobs = Vec::new();
            jobs.push(offerer_jobs[0].clone());
            jobs.push(acceptor_jobs[0].clone());

            (jobs, village_a_state, village_b_state)
        };

        worker.process_jobs(&going_jobs).await?;

        let uow_assert = uow_provider.tx().await?;
        let village_a_after = uow_assert.villages().get_by_id(village_a.id).await?;
        let village_b_after = uow_assert.villages().get_by_id(village_b.id).await?;

        assert_eq!(
            village_a_after.stored_resources().clay(),
            initial_a.stored_resources().clay() + seek_resources.clay()
        );
        assert_eq!(
            village_b_after.stored_resources().lumber(),
            initial_b.stored_resources().lumber() + offer_resources.lumber()
        );

        let report_repo = uow_assert.reports();
        let reports_a = report_repo.list_for_player(player_a.id, 10).await?;
        let reports_b = report_repo.list_for_player(player_b.id, 10).await?;

        let delivery_a: Vec<_> = reports_a
            .iter()
            .filter(|report| report.report_type == "marketplace_delivery")
            .collect();
        let delivery_b: Vec<_> = reports_b
            .iter()
            .filter(|report| report.report_type == "marketplace_delivery")
            .collect();

        assert_eq!(
            delivery_a.len(),
            2,
            "Player A should receive 2 delivery reports"
        );
        assert_eq!(
            delivery_b.len(),
            2,
            "Player B should receive 2 delivery reports"
        );

        let resources_a: Vec<_> = delivery_a
            .iter()
            .filter_map(|report| match report.payload {
                ReportPayload::MarketplaceDelivery(ref payload) => Some(payload.resources.clone()),
                _ => None,
            })
            .collect();
        assert!(resources_a.contains(&offer_resources));
        assert!(resources_a.contains(&seek_resources));

        Ok(())
    }

    #[tokio::test]
    async fn test_marketplace_offer_cancel_returns_resources() -> Result<()> {
        let (uow_provider, _config, app, _worker, player_a, village_a, _player_b, _village_b) =
            setup_marketplace_env().await?;

        let offer_resources = ResourceGroup(800, 0, 0, 0);
        let seek_resources = ResourceGroup(0, 300, 0, 0);

        app.execute(
            CreateMarketplaceOffer {
                village_id: village_a.id,
                offer_resources: offer_resources.clone(),
                seek_resources,
            },
            CreateMarketplaceOfferCommandHandler::new(),
        )
        .await?;

        let (offer_id, before_cancel) = {
            let uow = uow_provider.tx().await?;
            let offers = uow.marketplace().list_by_village(village_a.id).await?;
            let village = uow.villages().get_by_id(village_a.id).await?;
            (offers[0].id, village)
        };

        app.execute(
            CancelMarketplaceOffer {
                player_id: player_a.id,
                village_id: village_a.id,
                offer_id,
            },
            CancelMarketplaceOfferCommandHandler::new(),
        )
        .await?;

        let uow_assert = uow_provider.tx().await?;
        let offers_after = uow_assert
            .marketplace()
            .list_by_village(village_a.id)
            .await?;
        assert!(offers_after.is_empty(), "Offer should be deleted on cancel");

        let village_after = uow_assert.villages().get_by_id(village_a.id).await?;
        assert_eq!(
            village_after.stored_resources().lumber(),
            before_cancel.stored_resources().lumber() + offer_resources.lumber()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_send_resources_creates_report() -> Result<()> {
        let (uow_provider, _config, app, worker, player_a, village_a, player_b, village_b) =
            setup_marketplace_env().await?;

        let resources = ResourceGroup(500, 200, 0, 0);

        app.execute(
            SendResources {
                player_id: player_a.id,
                village_id: village_a.id,
                target_village_id: village_b.id,
                resources: resources.clone(),
            },
            SendResourcesCommandHandler::new(),
        )
        .await?;

        let going_job = {
            let uow = uow_provider.tx().await?;
            let jobs = uow.jobs().list_by_player_id(player_a.id).await?;
            assert_eq!(jobs.len(), 1);
            assert_eq!(jobs[0].task.task_type, "MerchantGoing");
            jobs[0].clone()
        };

        worker.process_jobs(&vec![going_job.clone()]).await?;

        let uow_assert = uow_provider.tx().await?;
        let reports = uow_assert.reports().list_for_player(player_b.id, 5).await?;
        let delivery_reports: Vec<_> = reports
            .iter()
            .filter(|report| report.report_type == "marketplace_delivery")
            .collect();

        assert_eq!(delivery_reports.len(), 1);
        if let ReportPayload::MarketplaceDelivery(ref payload) = delivery_reports[0].payload {
            assert_eq!(payload.resources, resources);
            assert_eq!(payload.receiver_village, village_b.name);
        } else {
            panic!("Expected marketplace delivery report payload");
        }

        Ok(())
    }
}
