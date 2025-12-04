use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use parabellum_app::{command_handlers::TrainUnitsCommandHandler, cqrs::commands::TrainUnits};
use parabellum_types::buildings::BuildingName;

use crate::{
    handlers::{CsrfForm, CurrentUser, HasCsrfToken},
    http::AppState,
};

use super::building_handler::render_with_error;
use rust_i18n::t;

#[derive(Debug, Deserialize)]
pub struct TrainUnitsForm {
    pub slot_id: u8,
    pub unit_idx: u8,
    pub quantity: i32,
    pub building_name: BuildingName,
    pub csrf_token: String,
}

impl HasCsrfToken for TrainUnitsForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

pub async fn train_units(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<TrainUnitsForm>,
) -> Response {
    if form.quantity <= 0 {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_training_quantity").to_string(),
        )
        .await;
    }

    let result = state
        .app_bus
        .execute(
            TrainUnits {
                player_id: user.player.id,
                village_id: user.village.id,
                unit_idx: form.unit_idx,
                quantity: form.quantity,
                building_name: form.building_name.clone(),
            },
            TrainUnitsCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build?s={}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}
