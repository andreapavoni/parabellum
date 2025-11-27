use parabellum_game::models::{map::MapField, village::Village};
use parabellum_types::{
    common::{Player, User},
    map::Position,
};
use uuid::Uuid;

use crate::cqrs::Query;

/// Checks if a user is authenticates with email and password.
pub struct AuthenticateUser {
    pub email: String,
    pub password: String,
}

impl Query for AuthenticateUser {
    type Output = User;
}

/// Fetch a user by email without checking password (for authenticated sessions).
pub struct GetUserByEmail {
    pub email: String,
}

impl Query for GetUserByEmail {
    type Output = User;
}

/// Fetch a user by id (for authenticated sessions).
pub struct GetUserById {
    pub id: Uuid,
}

impl Query for GetUserById {
    type Output = User;
}

/// Fetch the player entity associated to a user id.
pub struct GetPlayerByUserId {
    pub user_id: Uuid,
}

impl Query for GetPlayerByUserId {
    type Output = Player;
}

/// Fetch a village by id.
pub struct GetVillageById {
    pub id: u32,
}

impl Query for GetVillageById {
    type Output = Village;
}

/// List all villages for a player.
pub struct ListVillagesByPlayerId {
    pub player_id: Uuid,
}

impl Query for ListVillagesByPlayerId {
    type Output = Vec<Village>;
}

/// Fetch a square region of the world map.
pub struct GetMapRegion {
    pub center: Position,
    pub radius: i32,
    pub world_size: i32,
}

impl Query for GetMapRegion {
    type Output = Vec<MapField>;
}
