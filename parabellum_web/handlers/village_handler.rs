use axum::response::IntoResponse;

use crate::{
    handlers::{CurrentUser, render_template},
    templates::{ResourceField, ResourcesTemplate, VillageTemplate},
    view_helpers::resource_css_class,
};

pub async fn village(user: CurrentUser) -> impl IntoResponse {
    let template = VillageTemplate {
        current_user: Some(user),
        nav_active: "village",
    };
    render_template(template, None).into_response()
}

pub async fn resources(user: CurrentUser) -> impl IntoResponse {
    let resource_slots = user
        .village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let class = resource_css_class(Some(&slot));
            let name = slot.building.name.clone();
            let level = slot.building.level;
            ResourceField { class, name, level }
        })
        .collect();

    let template = ResourcesTemplate {
        current_user: Some(user),
        nav_active: "resources",
        resource_slots,
    };
    render_template(template, None).into_response()
}
