use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use parabellum_app::{
    command_handlers::{
        AcceptMarketplaceOfferCommandHandler, CancelMarketplaceOfferCommandHandler,
        CreateMarketplaceOfferCommandHandler, SendResourcesCommandHandler,
    },
    cqrs::commands::{
        AcceptMarketplaceOffer, CancelMarketplaceOffer, CreateMarketplaceOffer, SendResources,
    },
};
use parabellum_types::{buildings::BuildingName, common::ResourceGroup, map::Position};

use crate::{
    handlers::{
        building::{MAX_SLOT_ID, render_with_error},
        helpers::{CsrfForm, CurrentUser, HasCsrfToken},
    },
    http::AppState,
};

fn invalid_marketplace_message() -> String {
    "You can only use marketplace actions from the Marketplace.".to_string()
}

#[derive(Debug, Deserialize)]
pub struct SendResourcesForm {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
    pub csrf_token: String,
}

impl HasCsrfToken for SendResourcesForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateOfferForm {
    pub slot_id: u8,
    pub offer_lumber: u32,
    pub offer_clay: u32,
    pub offer_iron: u32,
    pub offer_crop: u32,
    pub seek_lumber: u32,
    pub seek_clay: u32,
    pub seek_iron: u32,
    pub seek_crop: u32,
    pub csrf_token: String,
}

impl HasCsrfToken for CreateOfferForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

#[derive(Debug, Deserialize)]
pub struct OfferActionForm {
    pub slot_id: u8,
    pub csrf_token: String,
}

impl HasCsrfToken for OfferActionForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /marketplace/send
pub async fn send_resources(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<SendResourcesForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let Some(slot_building) = user.village.get_building_by_slot_id(form.slot_id) else {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    };

    if slot_building.building.name != BuildingName::Marketplace {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let resources = ResourceGroup(form.lumber, form.clay, form.iron, form.crop);
    let target_position = Position {
        x: form.target_x,
        y: form.target_y,
    };
    let target_village_id = target_position.to_id(state.world_size);

    let result = state
        .app_bus
        .execute(
            SendResources {
                player_id: user.player.id,
                village_id: user.village.id,
                target_village_id,
                resources,
            },
            SendResourcesCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}

/// POST /marketplace/offer/create
pub async fn create_offer(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<CreateOfferForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let Some(slot_building) = user.village.get_building_by_slot_id(form.slot_id) else {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    };

    if slot_building.building.name != BuildingName::Marketplace {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let offer_resources = ResourceGroup(
        form.offer_lumber,
        form.offer_clay,
        form.offer_iron,
        form.offer_crop,
    );
    let seek_resources = ResourceGroup(
        form.seek_lumber,
        form.seek_clay,
        form.seek_iron,
        form.seek_crop,
    );

    let result = state
        .app_bus
        .execute(
            CreateMarketplaceOffer {
                village_id: user.village.id,
                offer_resources,
                seek_resources,
            },
            CreateMarketplaceOfferCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}

/// POST /marketplace/offer/accept/{offer_id}
pub async fn accept_offer(
    State(state): State<AppState>,
    Path(offer_id): Path<Uuid>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<OfferActionForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let Some(slot_building) = user.village.get_building_by_slot_id(form.slot_id) else {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    };

    if slot_building.building.name != BuildingName::Marketplace {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let result = state
        .app_bus
        .execute(
            AcceptMarketplaceOffer {
                player_id: user.player.id,
                village_id: user.village.id,
                offer_id,
            },
            AcceptMarketplaceOfferCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}

/// POST /marketplace/offer/cancel/{offer_id}
pub async fn cancel_offer(
    State(state): State<AppState>,
    Path(offer_id): Path<Uuid>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<OfferActionForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let Some(slot_building) = user.village.get_building_by_slot_id(form.slot_id) else {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    };

    if slot_building.building.name != BuildingName::Marketplace {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            invalid_marketplace_message(),
        )
        .await;
    }

    let result = state
        .app_bus
        .execute(
            CancelMarketplaceOffer {
                player_id: user.player.id,
                village_id: user.village.id,
                offer_id,
            },
            CancelMarketplaceOfferCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}
