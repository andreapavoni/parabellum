use crate::{
    db::models as db_models,
    game::models::{self as game_models},
};

pub struct VillageAggregate {
    pub village: db_models::Village,
    pub player: db_models::Player,
    pub armies: Vec<db_models::Army>,
    pub oases: Vec<db_models::MapField>,
}

impl TryFrom<VillageAggregate> for game_models::village::Village {
    type Error = anyhow::Error;

    fn try_from(agg: VillageAggregate) -> Result<Self, Self::Error> {
        let db_village = agg.village;
        let tribe: game_models::Tribe = agg.player.tribe.into();

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
            } else if game_army.village_id != village_id_u32
                && game_army.current_map_field_id == Some(village_id_u32)
            {
                reinforcements.push(game_army);
            } else if game_army.village_id == village_id_u32
                && game_army.current_map_field_id != Some(village_id_u32)
            {
                deployed_armies.push(game_army);
            }
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

        let village = game_models::village::Village {
            id: village_id_u32,
            name: db_village.name,
            player_id: db_village.player_id,
            position: position,
            tribe: tribe.clone(),
            buildings: serde_json::from_value(db_village.buildings)?,
            oases,
            population: db_village.population as u32,
            army: home_army,
            reinforcements,
            deployed_armies,
            loyalty: db_village.loyalty as u8,
            production: serde_json::from_value(db_village.production)?,
            is_capital: db_village.is_capital,
            smithy,
            academy_research,
            stocks: serde_json::from_value(db_village.stocks)?,
            updated_at: db_village.updated_at,
        };

        Ok(village)
    }
}

impl From<db_models::Tribe> for game_models::Tribe {
    fn from(db_tribe: db_models::Tribe) -> Self {
        match db_tribe {
            db_models::Tribe::Roman => game_models::Tribe::Roman,
            db_models::Tribe::Gaul => game_models::Tribe::Gaul,
            db_models::Tribe::Teuton => game_models::Tribe::Teuton,
            db_models::Tribe::Natar => game_models::Tribe::Natar,
            db_models::Tribe::Nature => game_models::Tribe::Nature,
        }
    }
}

impl From<game_models::Tribe> for db_models::Tribe {
    fn from(game_tribe: game_models::Tribe) -> Self {
        match game_tribe {
            game_models::Tribe::Roman => db_models::Tribe::Roman,
            game_models::Tribe::Gaul => db_models::Tribe::Gaul,
            game_models::Tribe::Teuton => db_models::Tribe::Teuton,
            game_models::Tribe::Natar => db_models::Tribe::Natar,
            game_models::Tribe::Nature => db_models::Tribe::Nature,
        }
    }
}

impl From<db_models::Player> for game_models::Player {
    fn from(player: db_models::Player) -> Self {
        game_models::Player {
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
            current_map_field_id: Some(army.current_map_field_id as u32),
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

impl From<db_models::Job> for crate::jobs::Job {
    fn from(job: db_models::Job) -> Self {
        crate::jobs::Job {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: serde_json::from_value(job.task).unwrap(),
            status: match job.status {
                db_models::JobStatus::Pending => crate::jobs::JobStatus::Pending,
                db_models::JobStatus::Processing => crate::jobs::JobStatus::Processing,
                db_models::JobStatus::Completed => crate::jobs::JobStatus::Completed,
                db_models::JobStatus::Failed => crate::jobs::JobStatus::Failed,
            },
            completed_at: job.completed_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}
