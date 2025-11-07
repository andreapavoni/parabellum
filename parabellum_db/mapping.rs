use parabellum_app::jobs::{Job, JobStatus};
use parabellum_core::DbError;
use parabellum_game::models::{self as game_models};
use parabellum_types::{common::Player, tribe::Tribe};

use crate::models::{self as db_models};

pub struct VillageAggregate {
    pub village: db_models::Village,
    pub player: db_models::Player,
    pub armies: Vec<db_models::Army>,
    pub oases: Vec<db_models::MapField>,
}

impl TryFrom<VillageAggregate> for game_models::village::Village {
    type Error = DbError;

    fn try_from(agg: VillageAggregate) -> Result<Self, Self::Error> {
        let db_village = agg.village;
        let tribe: Tribe = agg.player.tribe.into();

        let mut home_army: Option<game_models::army::Army> = None;
        let mut reinforcements = Vec::new();
        let mut deployed_armies = Vec::new();
        let village_id_u32 = db_village.id as u32;

        for db_army in agg.armies {
            let game_army: game_models::army::Army = db_army.into();

            if game_army.village_id == village_id_u32
                && game_army.current_map_field_id == Some(village_id_u32)
            {
                home_army = Some(game_army);

            // 2. È un rinforzo?
            // (village_id != casa E current_map_field_id = casa)
            } else if game_army.village_id != village_id_u32
                && game_army.current_map_field_id == Some(village_id_u32)
            {
                reinforcements.push(game_army);

            // 3. È un'armata in viaggio (deployed)?
            // (village_id = casa E current_map_field_id != casa [cioè None o un altro ID])
            } else if game_army.village_id == village_id_u32
                && game_army.current_map_field_id != Some(village_id_u32)
            {
                deployed_armies.push(game_army);
            }
            // --- FINE CORREZIONE ---
        }

        let oases: Vec<game_models::map::Oasis> = agg
            .oases
            .into_iter()
            .filter_map(|mf| {
                game_models::map::Oasis::try_from(Into::<game_models::map::MapField>::into(mf)).ok()
            })
            .collect();

        let smithy = serde_json::from_value(db_village.smithy_upgrades)?;
        let academy_research = serde_json::from_value(db_village.academy_research)?;
        let position = serde_json::from_value(db_village.position)?;
        let stocks = serde_json::from_value(db_village.stocks)?;
        let production = serde_json::from_value(db_village.production)?;
        let buildings = serde_json::from_value(db_village.buildings)?;

        let mut village = game_models::village::Village::from_persistence(
            village_id_u32,
            db_village.name,
            db_village.player_id,
            position,
            tribe.clone(),
            buildings,
            oases,
            db_village.population as u32,
            home_army,
            reinforcements,
            deployed_armies,
            db_village.loyalty as u8,
            production,
            db_village.is_capital,
            smithy,
            stocks,
            academy_research,
            db_village.updated_at,
        );

        village.update_state();
        Ok(village)
    }
}

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
        }
    }
}

impl From<db_models::Army> for game_models::army::Army {
    fn from(army: db_models::Army) -> Self {
        game_models::army::Army {
            id: army.id,
            village_id: army.village_id as u32,
            current_map_field_id: army.current_map_field_id.map(|id| id as u32),
            player_id: army.player_id,
            units: serde_json::from_value(army.units).unwrap_or_default(),
            smithy: serde_json::from_value(army.smithy).unwrap_or_default(),
            hero: None, // TODO: load hero through join
            tribe: army.tribe.into(),
        }
    }
}

impl From<db_models::MapField> for game_models::map::MapField {
    fn from(map_field: db_models::MapField) -> Self {
        game_models::map::MapField {
            id: map_field.id as u32,
            village_id: map_field.village_id.map(|id| id as u32),
            player_id: map_field.player_id,
            position: serde_json::from_value(map_field.position).unwrap(),
            topology: serde_json::from_value(map_field.topology).unwrap(),
        }
    }
}

impl From<db_models::Job> for Job {
    fn from(job: db_models::Job) -> Self {
        Job {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: serde_json::from_value(job.task).unwrap(),
            status: match job.status {
                db_models::JobStatus::Pending => JobStatus::Pending,
                db_models::JobStatus::Processing => JobStatus::Processing,
                db_models::JobStatus::Completed => JobStatus::Completed,
                db_models::JobStatus::Failed => JobStatus::Failed,
            },
            completed_at: job.completed_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

impl From<db_models::MarketplaceOffer> for game_models::marketplace::MarketplaceOffer {
    fn from(offer: db_models::MarketplaceOffer) -> Self {
        Self {
            id: offer.id,
            player_id: offer.player_id,
            village_id: offer.village_id as u32,
            offer_resources: serde_json::from_value(offer.offer_resources).unwrap(),
            seek_resources: serde_json::from_value(offer.seek_resources).unwrap(),
            merchants_required: offer.merchants_required as u8,
            created_at: offer.created_at,
        }
    }
}
