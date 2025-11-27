use crate::http::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::SignedCookieJar;

/// GET /logout – Log the user out by clearing the auth cookie.
pub async fn logout(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("user_email") {
        let updated_jar = jar.remove(cookie);
        return (updated_jar, Redirect::to("/")).into_response();
    }

    return Redirect::to("/").into_response();
}
