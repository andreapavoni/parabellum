//! Merchant and marketplace read-model projection.
//!
//! This module is intentionally projector-specific. It schedules operational
//! merchant actions, applies marketplace fact-carried read-model values
//! directly, and uses `Village` domain helpers only for merchant transfer
//! departure/return effects.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{MarketplaceOfferModel, MarketplaceOfferStatus};
use parabellum_game::models::village::VillageStocks;
use sqlx::{Postgres, Transaction};

use crate::es::consumers::village_projector::VillageProjector;
use crate::es::workflows;

impl VillageProjector {
    pub(super) async fn project_merchant_event_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Option<Result<(), CqrsError>> {
        match event {
            VillageEvent::MerchantsTripScheduled { .. } => {
                Some(self.project_merchants_trip_scheduled(tx, event).await)
            }
            VillageEvent::MerchantTransferAppliedToVillage {
                target_village_id,
                target_stocks,
                ..
            } => Some(
                self.apply_fact_stocks(tx, *target_village_id, target_stocks)
                    .await,
            ),
            VillageEvent::MerchantsReturned {
                source_village_id,
                merchants_used,
                ..
            } => Some(
                self.project_merchants_returned(tx, *source_village_id, *merchants_used)
                    .await,
            ),
            VillageEvent::MarketplaceOfferCreated { .. } => Some(
                self.project_marketplace_offer_created(tx, marketplace_offer_created_model(event))
                    .await,
            ),
            VillageEvent::MarketplaceOfferReservationAppliedToVillage {
                owner_village_id,
                owner_stocks,
                owner_busy_merchants,
                ..
            } => Some(
                self.apply_fact_stocks_and_busy_merchants(
                    tx,
                    *owner_village_id,
                    owner_stocks,
                    *owner_busy_merchants,
                )
                .await,
            ),
            VillageEvent::MarketplaceOfferCanceled {
                offer_id,
                canceled_at,
                ..
            } => Some(
                self.project_marketplace_offer_status(
                    tx,
                    *offer_id,
                    MarketplaceOfferStatus::Canceled,
                    None,
                    None,
                    *canceled_at,
                )
                .await,
            ),
            VillageEvent::MarketplaceOfferReservationReleasedFromVillage {
                owner_village_id,
                owner_stocks,
                owner_busy_merchants,
                ..
            } => Some(
                self.apply_fact_stocks_and_busy_merchants(
                    tx,
                    *owner_village_id,
                    owner_stocks,
                    *owner_busy_merchants,
                )
                .await,
            ),
            VillageEvent::MarketplaceOfferAccepted {
                offer_id,
                accepting_player_id,
                accepting_village_id,
                accepted_at,
                ..
            } => Some(
                self.project_marketplace_offer_status(
                    tx,
                    *offer_id,
                    MarketplaceOfferStatus::Accepted,
                    Some(*accepting_player_id),
                    Some(*accepting_village_id),
                    *accepted_at,
                )
                .await,
            ),
            VillageEvent::MarketplaceOfferAcceptanceAppliedToVillage {
                village_id,
                stocks,
                busy_merchants,
                ..
            } => Some(
                self.apply_fact_stocks_and_busy_merchants(tx, *village_id, stocks, *busy_merchants)
                    .await,
            ),
            _ => None,
        }
    }

    async fn project_merchants_trip_scheduled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &VillageEvent,
    ) -> Result<(), CqrsError> {
        let VillageEvent::MerchantsTripScheduled {
            source_village_id,
            resources,
            merchants_used,
            resources_already_reserved,
            ..
        } = event
        else {
            unreachable!(
                "project_merchants_trip_scheduled called with non-MerchantsTripScheduled event"
            );
        };
        let scheduled = workflows::merchants::scheduled_trip_from_event(event)?;
        self.add_scheduled_action_in_tx(tx, &scheduled.arrival_action)
            .await?;
        self.add_scheduled_action_in_tx(tx, &scheduled.return_action)
            .await?;

        if *resources_already_reserved {
            return Ok(());
        }

        let source = self
            .village
            .get_by_village_id_in_tx(tx, *source_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut source = Self::village_from_model(&source);
        source
            .reserve_merchant_transfer(resources, *merchants_used)
            .map_err(CqrsError::domain_source)?;

        self.set_stored_resources_in_tx(tx, *source_village_id, source.stored_resources())
            .await?;
        self.set_busy_merchants_in_tx(tx, *source_village_id, source.busy_merchants)
            .await
    }

    async fn project_merchants_returned(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        source_village_id: u32,
        merchants_used: u8,
    ) -> Result<(), CqrsError> {
        let source = self
            .village
            .get_by_village_id_in_tx(tx, source_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let mut source = Self::village_from_model(&source);
        source.return_merchants(merchants_used);
        self.set_busy_merchants_in_tx(tx, source_village_id, source.busy_merchants)
            .await
    }

    async fn project_marketplace_offer_created(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        offer: MarketplaceOfferModel,
    ) -> Result<(), CqrsError> {
        self.offers
            .upsert_in_tx(tx, &offer)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn project_marketplace_offer_status(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        offer_id: uuid::Uuid,
        status: MarketplaceOfferStatus,
        accepted_by_player_id: Option<uuid::Uuid>,
        accepted_by_village_id: Option<u32>,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), CqrsError> {
        self.offers
            .set_status_in_tx(
                tx,
                offer_id,
                status,
                accepted_by_player_id,
                accepted_by_village_id,
                at,
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    async fn apply_fact_stocks(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        stocks: &VillageStocks,
    ) -> Result<(), CqrsError> {
        self.set_stored_resources_in_tx(tx, village_id, stocks.stored())
            .await
    }

    async fn apply_fact_stocks_and_busy_merchants(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
        stocks: &VillageStocks,
        busy_merchants: u8,
    ) -> Result<(), CqrsError> {
        self.apply_fact_stocks(tx, village_id, stocks).await?;
        self.set_busy_merchants_in_tx(tx, village_id, busy_merchants)
            .await
    }
}

fn marketplace_offer_created_model(event: &VillageEvent) -> MarketplaceOfferModel {
    let VillageEvent::MarketplaceOfferCreated {
        offer_id,
        owner_player_id,
        owner_village_id,
        offer_resources,
        seek_resources,
        merchants_reserved,
        created_at,
    } = event
    else {
        unreachable!(
            "marketplace_offer_created_model called with non-MarketplaceOfferCreated event"
        );
    };

    MarketplaceOfferModel {
        offer_id: *offer_id,
        owner_player_id: *owner_player_id,
        owner_village_id: *owner_village_id,
        offer_resources: *offer_resources,
        seek_resources: *seek_resources,
        merchants_reserved: *merchants_reserved,
        status: MarketplaceOfferStatus::Open,
        accepted_by_player_id: None,
        accepted_by_village_id: None,
        created_at: *created_at,
        accepted_at: None,
        canceled_at: None,
    }
}
