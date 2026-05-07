use axum::response::Redirect;
use uuid::Uuid;

use parabellum_app::cqrs::queries::{VillageQueues, VillageTroopMovements};
use parabellum_game::models::village::Village;
use parabellum_types::{
    common::{Player as PlayerType, User as UserType},
    errors::ApplicationError,
};

use crate::http::AppState;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub account: UserType,
    pub player: PlayerType,
    pub village: Village,
    pub villages: Vec<Village>,
}

fn pick_active_village(villages: &[Village]) -> Option<&Village> {
    villages
        .iter()
        .find(|v| v.is_capital)
        .or_else(|| villages.first())
}

pub async fn current_user_by_ids(
    state: &AppState,
    user_id: Uuid,
    selected_village_id: Option<u32>,
) -> Result<CurrentUser, Redirect> {
    load_current_user(state, user_id, selected_village_id).await
}

async fn load_current_user(
    state: &AppState,
    user_id: Uuid,
    selected_village_id: Option<u32>,
) -> Result<CurrentUser, Redirect> {
    let user = state.game_app.get_user_by_id(user_id).await.map_err(|e| {
        tracing::error!("Unable to load user {user_id}: {e}");
        Redirect::to("/login")
    })?;

    let player = state
        .game_app
        .get_player_by_user_id(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Unable to load player for {user_id}: {e}");
            Redirect::to("/login")
        })?;

    let villages = load_villages(state, player.id).await?;
    let village = select_current_village(selected_village_id, &villages, player.id)?;

    Ok(CurrentUser {
        account: user,
        player,
        village,
        villages,
    })
}

fn select_current_village(
    selected_village_id: Option<u32>,
    villages: &[Village],
    player_id: Uuid,
) -> Result<Village, Redirect> {
    if let Some(selected) = village_from_selected_id(selected_village_id, villages, player_id) {
        return Ok(selected.clone());
    }

    pick_active_village(villages).cloned().ok_or_else(|| {
        tracing::error!("Player {player_id} has no villages configured");
        Redirect::to("/login")
    })
}

fn village_from_selected_id<'a>(
    selected_village_id: Option<u32>,
    villages: &'a [Village],
    player_id: Uuid,
) -> Option<&'a Village> {
    let village_id = selected_village_id?;

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
    let models = state
        .game_app
        .list_village_models_by_player_id(player_id)
        .await?;
    models.into_iter().map(Village::try_from).collect()
}

async fn load_village_queues(
    state: &AppState,
    village_id: u32,
) -> Result<VillageQueues, ApplicationError> {
    state.game_app.get_village_queues(village_id).await
}

pub async fn village_queues_or_empty(state: &AppState, village_id: u32) -> VillageQueues {
    match load_village_queues(state, village_id).await {
        Ok(queues) => queues,
        Err(err) => {
            tracing::error!(
                error = ?err,
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
    state.game_app.get_village_troop_movements(village_id).await
}

pub async fn village_movements_or_empty(
    state: &AppState,
    village_id: u32,
) -> VillageTroopMovements {
    match load_village_movements(state, village_id).await {
        Ok(movements) => movements,
        Err(err) => {
            tracing::error!(
                error = ?err,
                village_id,
                "Unable to load village troop movements"
            );
            VillageTroopMovements::default()
        }
    }
}
