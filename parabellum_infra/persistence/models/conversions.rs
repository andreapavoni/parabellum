use parabellum_game::models;
use parabellum_types::{
    common::{Player, User},
    tribe::Tribe,
};

use crate::persistence::models::{self as db_models};

impl From<db_models::Tribe> for Tribe {
    fn from(db_tribe: db_models::Tribe) -> Self {
        match db_tribe {
            db_models::Tribe::Roman => Tribe::Roman,
            db_models::Tribe::Gaul => Tribe::Gaul,
            db_models::Tribe::Teuton => Tribe::Teuton,
            db_models::Tribe::Natar => Tribe::Natar,
            db_models::Tribe::Nature => Tribe::Nature,
        }
    }
}

impl From<Tribe> for db_models::Tribe {
    fn from(game_tribe: Tribe) -> Self {
        match game_tribe {
            Tribe::Roman => db_models::Tribe::Roman,
            Tribe::Gaul => db_models::Tribe::Gaul,
            Tribe::Teuton => db_models::Tribe::Teuton,
            Tribe::Natar => db_models::Tribe::Natar,
            Tribe::Nature => db_models::Tribe::Nature,
        }
    }
}

impl From<db_models::Player> for Player {
    fn from(player: db_models::Player) -> Self {
        Player {
            id: player.id,
            username: player.username,
            tribe: player.tribe.into(),
            user_id: player.user_id,
            culture_points: player.culture_points as u32,
        }
    }
}

impl From<db_models::User> for User {
    fn from(user: db_models::User) -> Self {
        User::new(user.id, user.email, user.password_hash)
    }
}

impl From<db_models::MapField> for models::map::MapField {
    fn from(map_field: db_models::MapField) -> Self {
        models::map::MapField {
            id: map_field.id as u32,
            village_id: map_field.village_id.map(|id| id as u32),
            player_id: map_field.player_id,
            position: serde_json::from_value(map_field.position).unwrap(),
            topology: serde_json::from_value(map_field.topology).unwrap(),
        }
    }
}
