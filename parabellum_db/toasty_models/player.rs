use uuid::Uuid;

use parabellum_types::{
    common::Player,
    errors::{ApplicationError, DbError},
    tribe::Tribe,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum PlayerTribe {
    #[column(variant = 1)]
    Roman,
    #[column(variant = 2)]
    Gaul,
    #[column(variant = 3)]
    Teuton,
    #[column(variant = 4)]
    Natar,
    #[column(variant = 5)]
    Nature,
}

#[derive(Debug, Clone, toasty::Model)]
#[table = "players"]
pub struct PlayerRecord {
    #[key]
    pub id: Uuid,

    #[index]
    pub username: String,

    pub tribe: PlayerTribe,
    pub user_id: Uuid,
    pub culture_points: i32,
}

impl TryFrom<PlayerRecord> for Player {
    type Error = ApplicationError;

    fn try_from(player: PlayerRecord) -> Result<Self, Self::Error> {
        Ok(Player {
            id: player.id,
            username: player.username,
            tribe: player.tribe.into(),
            user_id: player.user_id,
            culture_points: u32::try_from(player.culture_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid culture_points value for player {}: {}",
                    player.id, player.culture_points
                )))
            })?,
        })
    }
}

impl TryFrom<&Player> for PlayerRecord {
    type Error = ApplicationError;

    fn try_from(player: &Player) -> Result<Self, Self::Error> {
        Ok(Self {
            id: player.id,
            username: player.username.clone(),
            tribe: player.tribe.clone().into(),
            user_id: player.user_id,
            culture_points: i32::try_from(player.culture_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "culture_points overflow for player {}: {}",
                    player.id, player.culture_points
                )))
            })?,
        })
    }
}

impl From<PlayerTribe> for Tribe {
    fn from(value: PlayerTribe) -> Self {
        match value {
            PlayerTribe::Roman => Tribe::Roman,
            PlayerTribe::Gaul => Tribe::Gaul,
            PlayerTribe::Teuton => Tribe::Teuton,
            PlayerTribe::Natar => Tribe::Natar,
            PlayerTribe::Nature => Tribe::Nature,
        }
    }
}

impl From<Tribe> for PlayerTribe {
    fn from(value: Tribe) -> Self {
        match value {
            Tribe::Roman => PlayerTribe::Roman,
            Tribe::Gaul => PlayerTribe::Gaul,
            Tribe::Teuton => PlayerTribe::Teuton,
            Tribe::Natar => PlayerTribe::Natar,
            Tribe::Nature => PlayerTribe::Nature,
        }
    }
}
