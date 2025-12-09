use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::SignedCookieJar;
use chrono::Utc;
use dioxus::prelude::*;

use crate::{
    components::{HomePage, LayoutData, LoginPage, PageLayout, RegisterPage, wrap_in_html},
    handlers::helpers::{ensure_not_authenticated, generate_csrf},
    http::AppState,
};

/// GET / - Home page
pub async fn home(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            HomePage {}
        }
    });

    Html(wrap_in_html(&body_content)).into_response()
}

/// GET /login - Login page
pub async fn login_page(jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let (jar, csrf_token) = generate_csrf(jar);

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            LoginPage {
                csrf_token: csrf_token,
                email_value: String::new(),
                error: None,
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}

/// GET /register - Register page
pub async fn register_page(jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let (jar, csrf_token) = generate_csrf(jar);

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            RegisterPage {
                csrf_token: csrf_token,
                username_value: String::new(),
                email_value: String::new(),
                selected_tribe: "Roman".to_string(),
                selected_quadrant: "NorthEast".to_string(),
                error: None,
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}
