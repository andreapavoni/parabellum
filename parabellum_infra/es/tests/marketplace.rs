use chrono::{Duration, Utc};

use parabellum_app::villages::CreateMarketplaceOffer;
use parabellum_types::{
    common::{ResourceKind, ResourceQuantity},
    map::Position,
    tribe::Tribe,
};

use crate::es::VillageEsService;

use super::fixtures::{
    granary, main_building, marketplace, resources, setup_village, warehouse, with_test_pool,
};

#[tokio::test]
async fn village_es_service_marketplace_offer_create_accept_flow() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_owner_user_id, owner_player_id, owner_village_id) = setup_village(
            &pool,
            &service,
            "owner",
            Position { x: 0, y: 0 },
            Tribe::Gaul,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let (_acceptor_user_id, acceptor_player_id, acceptor_village_id) = setup_village(
            &pool,
            &service,
            "acceptor",
            Position { x: 8, y: 8 },
            Tribe::Roman,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        let (_second_acceptor_user_id, second_acceptor_player_id, second_acceptor_village_id) =
            setup_village(
                &pool,
                &service,
                "acceptor-2",
                Position { x: 10, y: 10 },
                Tribe::Teuton,
                vec![
                    main_building(10),
                    warehouse(20),
                    granary(20),
                    marketplace(10),
                ],
                resources(80_000, 80_000, 80_000, 80_000),
            )
            .await;

        service
            .create_marketplace_offer(
                owner_village_id,
                &CreateMarketplaceOffer {
                    player_id: owner_player_id,
                    offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 1_000),
                    seek_resources: ResourceQuantity::new(ResourceKind::Iron, 900),
                },
            )
            .await
            .expect("offer creation should succeed");

        let owner_after_create = service
            .get_village(owner_village_id)
            .await
            .expect("owner model should be readable");
        assert_eq!(owner_after_create.stocks.lumber, 79_000);
        assert_eq!(owner_after_create.busy_merchants, 2);

        let open_offers = service
            .get_open_marketplace_offers()
            .await
            .expect("open offers query should succeed");
        assert_eq!(open_offers.len(), 1);
        let offer = &open_offers[0];
        assert_eq!(offer.owner_village_id, owner_village_id);

        service
            .accept_marketplace_offer(
                acceptor_village_id,
                acceptor_player_id,
                offer.offer_id,
                Utc::now() + Duration::minutes(5),
                Utc::now() + Duration::minutes(5),
            )
            .await
            .expect("offer acceptance should succeed");

        let second_attempt = service
            .accept_marketplace_offer(
                second_acceptor_village_id,
                second_acceptor_player_id,
                offer.offer_id,
                Utc::now() + Duration::minutes(6),
                Utc::now() + Duration::minutes(6),
            )
            .await;
        assert!(
            second_attempt.is_err(),
            "same offer must not be accepted twice"
        );

        let owner_after_accept = service
            .get_village(owner_village_id)
            .await
            .expect("owner model should be readable");
        assert_eq!(
            owner_after_accept.stocks.lumber, 79_000,
            "owner resources must not be deducted twice on accept"
        );
        assert_eq!(owner_after_accept.busy_merchants, 2);

        let acceptor_after_accept = service
            .get_village(acceptor_village_id)
            .await
            .expect("acceptor model should be readable");
        assert_eq!(acceptor_after_accept.stocks.iron, 79_100);
        assert!(acceptor_after_accept.busy_merchants >= 1);
    })
    .await;
}

#[tokio::test]
async fn village_es_service_marketplace_offer_create_cancel_flow() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());

        let (_owner_user_id, owner_player_id, owner_village_id) = setup_village(
            &pool,
            &service,
            "owner-cancel",
            Position { x: 1, y: 1 },
            Tribe::Roman,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .create_marketplace_offer(
                owner_village_id,
                &CreateMarketplaceOffer {
                    player_id: owner_player_id,
                    offer_resources: ResourceQuantity::new(ResourceKind::Clay, 1_200),
                    seek_resources: ResourceQuantity::new(ResourceKind::Iron, 900),
                },
            )
            .await
            .expect("offer creation should succeed");

        let owner_after_create = service
            .get_village(owner_village_id)
            .await
            .expect("owner model should be readable after create");
        assert_eq!(owner_after_create.stocks.clay, 78_800);
        assert_eq!(owner_after_create.busy_merchants, 3);

        let open_offers = service
            .get_open_marketplace_offers()
            .await
            .expect("open offers query should succeed");
        assert_eq!(open_offers.len(), 1);
        let offer = open_offers[0].clone();

        service
            .cancel_marketplace_offer(owner_village_id, owner_player_id, offer.offer_id)
            .await
            .expect("offer cancellation should succeed");

        let owner_after_cancel = service
            .get_village(owner_village_id)
            .await
            .expect("owner model should be readable after cancel");
        assert_eq!(owner_after_cancel.stocks.clay, 80_000);
        assert_eq!(owner_after_cancel.busy_merchants, 0);

        let offer_after_cancel = service
            .get_marketplace_offer(offer.offer_id)
            .await
            .expect("canceled offer should still be readable");
        assert_eq!(
            offer_after_cancel.status,
            parabellum_app::villages::models::MarketplaceOfferStatus::Canceled
        );

        let open_after_cancel = service
            .get_open_marketplace_offers()
            .await
            .expect("open offers query after cancel should succeed");
        assert!(
            open_after_cancel
                .iter()
                .all(|open| open.offer_id != offer.offer_id),
            "canceled offer must not remain open"
        );
    })
    .await;
}

#[tokio::test]
async fn village_es_service_marketplace_offer_accept_closes_offer_and_rejects_cancel() {
    with_test_pool(|pool| async move {
        let service = VillageEsService::new(pool.clone());
        let (_owner_user_id, owner_player_id, owner_village_id) = setup_village(
            &pool,
            &service,
            "owner-accept",
            Position { x: 2, y: 2 },
            Tribe::Roman,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;
        let (_acceptor_user_id, acceptor_player_id, acceptor_village_id) = setup_village(
            &pool,
            &service,
            "acceptor-accept",
            Position { x: 12, y: 12 },
            Tribe::Gaul,
            vec![
                main_building(10),
                warehouse(20),
                granary(20),
                marketplace(10),
            ],
            resources(80_000, 80_000, 80_000, 80_000),
        )
        .await;

        service
            .create_marketplace_offer(
                owner_village_id,
                &CreateMarketplaceOffer {
                    player_id: owner_player_id,
                    offer_resources: ResourceQuantity::new(ResourceKind::Iron, 1_000),
                    seek_resources: ResourceQuantity::new(ResourceKind::Crop, 900),
                },
            )
            .await
            .unwrap();

        let offer = service.get_open_marketplace_offers().await.unwrap()[0].clone();
        service
            .accept_marketplace_offer(
                acceptor_village_id,
                acceptor_player_id,
                offer.offer_id,
                Utc::now() + Duration::minutes(3),
                Utc::now() + Duration::minutes(3),
            )
            .await
            .unwrap();

        let cancel_after_accept = service
            .cancel_marketplace_offer(owner_village_id, owner_player_id, offer.offer_id)
            .await;
        assert!(
            cancel_after_accept.is_err(),
            "accepted offer must not be cancellable"
        );

        let offer_after_accept = service.get_marketplace_offer(offer.offer_id).await.unwrap();
        assert_eq!(
            offer_after_accept.status,
            parabellum_app::villages::models::MarketplaceOfferStatus::Accepted
        );
        assert!(
            service.get_open_marketplace_offers().await.unwrap().is_empty(),
            "accepted offer must not remain in open marketplace list"
        );

        service
            .process_due_actions(Utc::now() + Duration::minutes(20), 200)
            .await
            .unwrap();

        let owner_after_roundtrip = service.get_village(owner_village_id).await.unwrap();
        let acceptor_after_roundtrip = service.get_village(acceptor_village_id).await.unwrap();
        assert_eq!(owner_after_roundtrip.busy_merchants, 0);
        assert_eq!(acceptor_after_roundtrip.busy_merchants, 0);
    })
    .await;
}
