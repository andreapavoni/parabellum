use askama::Template;
use axum::{
    extract::{Form, FromRef, FromRequest, FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::{
    SignedCookieJar,
    cookie::{Cookie, Key, SameSite},
};
use std::future::Future;
use uuid::Uuid;

use parabellum_app::{
    cqrs::queries::{
        GetPlayerByUserId, GetUserById, GetVillageQueues, GetVillageTroopMovements,
        ListVillagesByPlayerId, VillageQueues, VillageTroopMovements,
    },
    queries_handlers::{
        GetPlayerByUserIdHandler, GetUserByIdHandler, GetVillageQueuesHandler,
        GetVillageTroopMovementsHandler, ListVillagesByPlayerIdHandler,
    },
};
use parabellum_game::models::village::Village;
use parabellum_types::{
    common::{Player as PlayerType, User as UserType},
    errors::ApplicationError,
};

use crate::http::AppState;

/// Helper: render a Template to HTML or return 500 on error
pub fn render_template<T: Template>(template: T, status: Option<StatusCode>) -> impl IntoResponse {
    match template.render() {
        Ok(html) => {
            let status = status.unwrap_or(StatusCode::OK);
            (status, Html(html)).into_response()
        }
        Err(err) => {
            tracing::error!("Template render error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
        }
    }
}

/// Generates a new CSRF token, puts it into a signed cookie,
/// and returns updated cookie jar.
pub fn generate_csrf(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    let token = Uuid::new_v4().to_string();
    let cookie = Cookie::build(Cookie::new("csrf_token", token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .build();
    let jar = jar.add(cookie);
    (jar, token)
}

/// Verify CSRF token from form matches the one in the cookie.
pub fn validate_csrf(jar: &SignedCookieJar, form_token: &str) -> bool {
    jar.get("csrf_token")
        .map(|cookie| cookie.value() == form_token)
        .unwrap_or(false)
}

fn pick_active_village<'a>(villages: &'a [Village]) -> Option<&'a Village> {
    villages
        .iter()
        .find(|v| v.is_capital)
        .or_else(|| villages.first())
}

/// Initialize session cookies (`user_id` and `current_village_id`) for an authenticated user.
pub async fn initialize_session(
    state: &AppState,
    jar: SignedCookieJar,
    user_id: Uuid,
) -> Result<SignedCookieJar, ApplicationError> {
    let player = state
        .app_bus
        .query(
            GetPlayerByUserId { user_id },
            GetPlayerByUserIdHandler::new(),
        )
        .await?;

    let villages = list_player_villages(state, player.id).await?;

    let village_id = pick_active_village(&villages)
        .map(|v| v.id)
        .ok_or_else(|| {
            ApplicationError::Unknown(format!("Player {} has no villages", player.id))
        })?;

    let jar = jar.add(Cookie::new("user_id", user_id.to_string()));
    let jar = jar.add(Cookie::new("current_village_id", village_id.to_string()));
    Ok(jar)
}

/// Trait that exposes a CSRF token field on a form type.
pub trait HasCsrfToken {
    fn csrf_token(&self) -> &str;
}

/// Extractor that wraps a form type and enforces CSRF validation.
/// On success it yields the parsed form and the cookie jar.
/// On failure it returns a 400 response.
pub struct CsrfForm<T> {
    pub form: T,
    pub jar: SignedCookieJar,
}

impl<S, T> FromRequest<S> for CsrfForm<T>
where
    S: Send + Sync,
    Key: FromRef<S>,
    T: HasCsrfToken + serde::de::DeserializeOwned + Send,
{
    type Rejection = Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            use axum::extract::FromRequest;

            // Extract both cookie jar and form in one go
            let (jar, Form(inner)) =
                match <(SignedCookieJar, Form<T>) as FromRequest<S>>::from_request(req, state).await
                {
                    Ok(v) => v,
                    Err(rejection) => return Err(rejection.into_response()),
                };

            if !validate_csrf(&jar, inner.csrf_token()) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid form token. Please try again.",
                )
                    .into_response());
            }

            Ok(CsrfForm { form: inner, jar })
        }
    }
}

/// Loads the currently authenticated session (user, player, villages) from the cookies.
/// Returns `Ok(CurrentUser)` if found, or `Err(Redirect)` to redirect to /login.
pub async fn current_user(
    state: &AppState,
    jar: &SignedCookieJar,
) -> Result<CurrentUser, Redirect> {
    let user_cookie = jar.get("user_id").ok_or_else(|| Redirect::to("/login"))?;
    let user_id = Uuid::parse_str(user_cookie.value()).map_err(|_| Redirect::to("/login"))?;

    let user = state
        .app_bus
        .query(GetUserById { id: user_id }, GetUserByIdHandler::new())
        .await
        .map_err(|e| {
            tracing::error!("Unable to load user {user_id}: {e}");
            Redirect::to("/login")
        })?;

    let player = state
        .app_bus
        .query(
            GetPlayerByUserId { user_id },
            GetPlayerByUserIdHandler::new(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Unable to load player for {user_id}: {e}");
            Redirect::to("/login")
        })?;

    let villages = load_villages(state, player.id).await?;
    let village = select_current_village(jar, &villages, player.id)?;

    Ok(CurrentUser {
        account: user,
        player,
        village,
        villages,
    })
}

fn select_current_village(
    jar: &SignedCookieJar,
    villages: &[Village],
    player_id: Uuid,
) -> Result<Village, Redirect> {
    if let Some(selected) = village_from_cookie(jar, villages, player_id) {
        return Ok(selected.clone());
    }

    pick_active_village(villages).cloned().ok_or_else(|| {
        tracing::error!("Player {player_id} has no villages configured");
        Redirect::to("/login")
    })
}

fn village_from_cookie<'a>(
    jar: &SignedCookieJar,
    villages: &'a [Village],
    player_id: Uuid,
) -> Option<&'a Village> {
    let village_id = jar
        .get("current_village_id")
        .and_then(|cookie| cookie.value().parse::<u32>().ok())?;

    villages
        .iter()
        .find(|v| v.id == village_id && v.player_id == player_id)
}

async fn load_villages(state: &AppState, player_id: Uuid) -> Result<Vec<Village>, Redirect> {
    list_player_villages(state, player_id).await.map_err(|e| {
        tracing::error!("Unable to list villages for player {player_id}: {e}");
        Redirect::to("/logout")
    })
}

async fn list_player_villages(
    state: &AppState,
    player_id: Uuid,
) -> Result<Vec<Village>, ApplicationError> {
    state
        .app_bus
        .query(
            ListVillagesByPlayerId { player_id },
            ListVillagesByPlayerIdHandler::new(),
        )
        .await
}

async fn load_village_queues(
    state: &AppState,
    village_id: u32,
) -> Result<VillageQueues, ApplicationError> {
    state
        .app_bus
        .query(
            GetVillageQueues { village_id },
            GetVillageQueuesHandler::new(),
        )
        .await
}

pub async fn village_queues_or_empty(state: &AppState, village_id: u32) -> VillageQueues {
    match load_village_queues(state, village_id).await {
        Ok(queues) => queues,
        Err(err) => {
            tracing::error!(
                error = ?err.to_string(),
                village_id,
                "Unable to load village queues"
            );
            VillageQueues::default()
        }
    }
}

async fn load_village_movements(
    state: &AppState,
    village_id: u32,
) -> Result<VillageTroopMovements, ApplicationError> {
    state
        .app_bus
        .query(
            GetVillageTroopMovements { village_id },
            GetVillageTroopMovementsHandler::new(),
        )
        .await
}

pub async fn village_movements_or_empty(
    state: &AppState,
    village_id: u32,
) -> VillageTroopMovements {
    match load_village_movements(state, village_id).await {
        Ok(movements) => movements,
        Err(err) => {
            tracing::error!(
                error = ?err.to_string(),
                village_id,
                "Unable to load village troop movements"
            );
            VillageTroopMovements::default()
        }
    }
}

/// Ensures that the requester is not already authenticated.
/// If a `user_id` cookie is found, returns a redirect to `/village`.
pub fn ensure_not_authenticated(jar: &SignedCookieJar) -> Result<(), Redirect> {
    if jar.get("user_id").is_some() {
        Err(Redirect::to("/village"))
    } else {
        Ok(())
    }
}

/// Extractor for authenticated users.
/// Automatically loads the user from the cookie.
/// If no user is found or the user doesn't exist, returns a redirect to `/login`.
#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub account: UserType,
    pub player: PlayerType,
    pub village: Village,
    pub villages: Vec<Village>,
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    Key: FromRef<S>,
    AppState: FromRef<S>,
{
    type Rejection = Redirect;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let jar = SignedCookieJar::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/login"))?;
            let app_state = State::<AppState>::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/login"))?;

            current_user(&app_state, &jar).await
        }
    }
}
