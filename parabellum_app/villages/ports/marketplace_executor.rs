//! Marketplace command/workflow execution gateway.
//!
//! The app use case builds marketplace command intent and merchant workflow
//! timing. Infrastructure implements this port with CQRS/ES command execution,
//! atomic offer claims, and workflow event appends.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::{CreateMarketplaceOffer, SendMerchantsTransfer};

/// Canonical marketplace command/workflow intent produced by app use cases.
#[derive(Debug, Clone)]
pub enum MarketplaceCommandIntent {
    /// Execute a direct merchant transfer from one village to another.
    SendResources {
        /// Aggregate id for the source village.
        source_village_id: u32,
        /// Domain command with validated player intent and planned arrival.
        command: SendMerchantsTransfer,
    },
    /// Create a marketplace offer and reserve owner-side merchants/resources.
    CreateOffer {
        /// Aggregate id for the owner village.
        village_id: u32,
        /// Domain command with validated offer terms.
        command: CreateMarketplaceOffer,
    },
    /// Atomically claim an open marketplace offer and append both merchant workflows.
    AcceptOffer {
        /// Village accepting the offer.
        accepting_village_id: u32,
        /// Player expected to own the accepting village.
        accepting_player_id: Uuid,
        /// Offer to claim.
        offer_id: Uuid,
        /// Arrival time for owner resources traveling to the accepting village.
        owner_arrives_at: DateTime<Utc>,
        /// Arrival time for accepting resources traveling to the owner village.
        accepting_arrives_at: DateTime<Utc>,
    },
    /// Cancel an open offer owned by a village/player pair.
    CancelOffer {
        /// Owner village that created the offer.
        village_id: u32,
        /// Owner player that created the offer.
        player_id: Uuid,
        /// Offer to cancel.
        offer_id: Uuid,
    },
}

/// Executes marketplace commands and workflow intent through infrastructure.
#[async_trait]
pub trait MarketplaceCommandExecutor: Send + Sync {
    /// Persist and execute the already-planned marketplace command intent.
    async fn execute_marketplace_command(
        &self,
        command: MarketplaceCommandIntent,
    ) -> Result<(), ApplicationError>;
}
