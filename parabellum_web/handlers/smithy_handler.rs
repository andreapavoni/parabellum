use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use parabellum_app::{
    command_handlers::ResearchSmithyCommandHandler, cqrs::commands::ResearchSmithy,
};
use parabellum_types::{army::UnitName, buildings::BuildingName};
use rust_i18n::t;
use serde::Deserialize;

use crate::{
    handlers::{CsrfForm, CurrentUser, HasCsrfToken},
    http::AppState,
};

use super::building_handler::{MAX_SLOT_ID, render_with_error};

#[derive(Debug, Deserialize)]
pub struct SmithyResearchForm {
    pub slot_id: u8,
    pub unit_name: UnitName,
    pub csrf_token: String,
}

impl HasCsrfToken for SmithyResearchForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

pub async fn research_smithy(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<SmithyResearchForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_smithy_building").to_string(),
        )
        .await;
    }

    let Some(slot_building) = user.village.get_building_by_slot_id(form.slot_id) else {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_smithy_building").to_string(),
        )
        .await;
    };

    if slot_building.building.name != BuildingName::Smithy {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_smithy_building").to_string(),
        )
        .await;
    }

    let result = state
        .app_bus
        .execute(
            ResearchSmithy {
                unit: form.unit_name.clone(),
                village_id: user.village.id,
            },
            ResearchSmithyCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build?s={}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}
