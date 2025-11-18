use crate::http::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

/// GET /logout – Log the user out by clearing the auth cookie.
pub async fn logout(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    // Remove the signed authentication cookie (if it exists)
    let updated_jar = jar.remove(Cookie::build("user_email"));
    // Redirect to home page (logged-out state)
    (updated_jar, Redirect::to("/"))
}
