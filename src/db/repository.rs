use std::env;

use anyhow::{Error, Result};
use polodb_core::{bson::doc, Collection, CollectionT, Database};
use uuid::Uuid;

use crate::game::models::{
    map::{generate_new_map, MapField, Oasis, Position, Quadrant, Valley},
    village::Village,
    Player, Tribe,
};

// TODO: everything here could/should be managed through commands/queries, no need for a Repository object

pub struct Repository {
    db: Database,
}

impl Repository {
    pub async fn new_from_env() -> Result<Self> {
        let path = env::var("DATABASE_PATH").expect("DATABASE_PATH is not set");
        let db = Database::open_path(path)?;

        Ok(Self { db })
    }

    pub async fn new(path: String) -> Result<Self> {
        let db = Database::open_path(path)?;

        Ok(Self { db })
    }
}
#[async_trait::async_trait]
impl crate::repository::Repository for Repository {
    async fn bootstrap_new_map(&self, size: u32) -> Result<(), Error> {
        let map = generate_new_map(size as i32);
        let tx = self.db.start_transaction()?;

        print!("Generating a map of {} fields... ", size * size * 4);

        for mf in map {
            let map_fields = tx.collection::<MapField>("map_fields");
            map_fields.insert_one(mf)?;
        }
        tx.commit()?;
        println!("done!");

        Ok(())
    }

    async fn get_unoccupied_valley(&self, quadrant: Option<Quadrant>) -> Result<Valley> {
        let map_fields = self.db.collection::<MapField>("map_fields");

        let query = match quadrant {
            Some(Quadrant::NorthEast) => {
                map_fields.find(doc! {
                   "player_id": { "$eq": None },
                   "village_id": { "$eq": None },
                   "position.x": { "$gte": 0 },
                   "position.y": { "$gte": 0 },
                   "topology": {"$eq": MapFieldTopology::Valley(ValleyTopology(4,4,4,6))},
                }) // TODO: order by random
            }
            Some(Quadrant::EastSouth) => {
                map_fields.find(doc! {
                   "player_id": { "$eq": None },
                   "village_id": { "$eq": None },
                   "position.x": { "$gte": 0 },
                   "position.y": { "$lt": 0 },
                   "topology": {"$eq": MapFieldTopology::Valley(ValleyTopology(4,4,4,6))},
                }) // TODO: order by random
            }
            Some(Quadrant::SouthWest) => {
                map_fields.find(doc! {
                   "player_id": { "$eq": None },
                   "village_id": { "$eq": None },
                   "position.x": { "$lt": 0 },
                   "position.y": { "$lt": 0 },
                   "topology": {"$eq": MapFieldTopology::Valley(ValleyTopology(4,4,4,6))},
                }) // TODO: order by random
            }
            Some(Quadrant::WestNorth) => {
                map_fields.find(doc! {
                   "player_id": { "$eq": None },
                   "village_id": { "$eq": None },
                   "position.x": { "$lt": 0 },
                   "position.y": { "$gte": 0 },
                   "topology": {"$eq": MapFieldTopology::Valley(ValleyTopology(4,4,4,6))},
                }) // TODO: order by random
            }
            None => map_fields.find(doc! {
               "player_id": { "$eq": None },
               "village_id": { "$eq": None },
            }),
        };
        let valley: MapField = query.limit(1).run()?;

        Ok(valley.try_into()?)
    }

    async fn register_player(&self, username: String, tribe: Tribe) -> Result<Player> {
        let tx = self.db.start_transaction()?;
        let collection = tx.collection::<Player>("players");

        if let Ok(_) = collection
            .find(doc! { "username": { "$eq": username.clone() }, })
            .run()
        {
            return Err(Error::msg("Username already used."));
        }

        let player = Player {
            id: Uuid::new_v4(),
            username,
            tribe,
        };

        collection.insert_one(player)?;
        tx.commit()?;

        Ok(player.into())
    }

    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player> {
        let collection = self.db.collection::<Player>("players");
        let player: Player = collection
            .find(doc! { "id": { "$eq": player_id }, })
            .run()?;

        Ok(player.into())
    }

    async fn get_player_by_username(&self, username: String) -> Result<Player> {
        let collection = self.db.collection::<Player>("players");
        let player: Player = collection
            .find(doc! { "username": { "$eq": username }, })
            .run()?;

        Ok(player.into())
    }

    async fn get_village_by_id(&self, village_id: u32) -> Result<GameVillage> {
        let collection = self.db.collection::<Village>("villages");
        let village: Village = collection
            .find(doc! { "id": { "$eq": village_id }, })
            .run()?;

        Ok(village.into())
    }

    async fn get_valley_by_id(&self, valley_id: u32) -> Result<Valley> {
        let collection = self.db.collection::<MapField>("map_fields");
        let valley: MapField = collection
            .find(doc! { "id": { "$eq": valley_id }, })
            .run()?;

        Ok(valley.try_into()?)
    }

    async fn get_oasis_by_id(&self, oasis_id: u32) -> Result<Oasis> {
        let collection = self.db.collection::<MapField>("map_fields");
        let oasis: MapField = collection.find(doc! { "id": { "$eq": oasis_id }, }).run()?;

        Ok(oasis.try_into()?)
    }
}
