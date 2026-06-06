#[cfg(any(test, feature = "test-utils"))]
#[cfg(not(tarpaulin_include))]
pub mod tests {
    use async_trait::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    use parabellum_game::models::map::{MapField, MapFieldTopology, MapQuadrant, Valley};
    use parabellum_types::{
        common::{Player, User},
        errors::{ApplicationError, DbError},
        map::{Position, ValleyTopology},
    };

    use crate::{
        ports::{
            identity::{PlayerRepository, UserRepository},
            map::MapRepository,
        },
        read_models::{MapRegionTile, PlayerLeaderboardEntry},
    };

    #[derive(Default, Clone)]
    pub struct MockPlayerRepository {
        players: Arc<Mutex<HashMap<Uuid, Player>>>,
    }

    #[async_trait]
    impl PlayerRepository for MockPlayerRepository {
        async fn save(&self, player: &Player) -> Result<(), ApplicationError> {
            self.players
                .lock()
                .unwrap()
                .insert(player.id, player.clone());
            Ok(())
        }

        async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
            self.players
                .lock()
                .unwrap()
                .get(&player_id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::PlayerNotFound(player_id)))
        }

        async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
            self.players
                .lock()
                .unwrap()
                .values()
                .find(|p| p.user_id == user_id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))
        }

        async fn leaderboard_page(
            &self,
            offset: i64,
            limit: i64,
        ) -> Result<(Vec<PlayerLeaderboardEntry>, i64), ApplicationError> {
            let mut entries: Vec<PlayerLeaderboardEntry> = self
                .players
                .lock()
                .unwrap()
                .values()
                .map(|player| PlayerLeaderboardEntry {
                    player_id: player.id,
                    username: player.username.clone(),
                    village_count: 0,
                    population: 0,
                    tribe: player.tribe.clone(),
                })
                .collect();

            entries.sort_by(|a, b| {
                b.population
                    .cmp(&a.population)
                    .then_with(|| b.village_count.cmp(&a.village_count))
                    .then_with(|| a.username.cmp(&b.username))
            });

            let total = entries.len() as i64;
            let start = offset.max(0) as usize;
            let end = (start + limit as usize).min(entries.len());
            let page_entries = if start >= entries.len() {
                Vec::new()
            } else {
                entries[start..end].to_vec()
            };
            Ok((page_entries, total))
        }

        async fn update_culture_points(&self, _player_id: Uuid) -> Result<(), ApplicationError> {
            Ok(())
        }

        async fn get_total_culture_points_production(
            &self,
            _player_id: Uuid,
        ) -> Result<u32, ApplicationError> {
            Ok(1)
        }
    }

    #[derive(Default, Clone)]
    pub struct MockMapRepository {
        fields: Arc<Mutex<HashMap<u32, MapField>>>,
    }

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(
            &self,
            _quadrant: &MapQuadrant,
        ) -> Result<Valley, ApplicationError> {
            Ok(MapField {
                id: 100,
                position: Position { x: 10, y: 10 },
                village_id: None,
                topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                player_id: None,
            }
            .try_into()
            .unwrap())
        }

        async fn get_foundation_target_topology(
            &self,
            _field_id: u32,
            _player_id: Uuid,
        ) -> Result<Option<ValleyTopology>, ApplicationError> {
            Ok(Some(ValleyTopology(4, 4, 4, 6)))
        }

        async fn get_field_by_id(&self, _id: i32) -> Result<MapField, ApplicationError> {
            Ok(MapField {
                id: 100,
                position: Position { x: 10, y: 10 },
                village_id: None,
                topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                player_id: None,
            })
        }

        async fn get_region(
            &self,
            center_x: i32,
            center_y: i32,
            radius: i32,
            world_size: i32,
        ) -> Result<Vec<MapRegionTile>, ApplicationError> {
            let fields = self.fields.lock().unwrap();
            let mut region = Vec::new();

            for y in ((center_y - radius)..=(center_y + radius)).rev() {
                let wrapped_y = wrap_coordinate(y, world_size);
                for x in center_x - radius..=center_x + radius {
                    let wrapped_x = wrap_coordinate(x, world_size);
                    let position = Position {
                        x: wrapped_x,
                        y: wrapped_y,
                    };
                    let id = position.to_id(world_size);
                    if let Some(field) = fields.get(&id) {
                        region.push(MapRegionTile {
                            field: field.clone(),
                            village_name: None,
                            village_population: None,
                            player_name: None,
                            tribe: None,
                            is_capital: None,
                        });
                    }
                }
            }

            Ok(region)
        }

        async fn get_region_tile_by_field_id(
            &self,
            field_id: i32,
        ) -> Result<Option<MapRegionTile>, ApplicationError> {
            let fields = self.fields.lock().unwrap();
            Ok(fields.get(&(field_id as u32)).map(|field| MapRegionTile {
                field: field.clone(),
                village_name: None,
                village_population: None,
                player_name: None,
                tribe: None,
                is_capital: None,
            }))
        }

        async fn is_unoccupied_valley(&self, field_id: i32) -> Result<bool, ApplicationError> {
            let fields = self.fields.lock().unwrap();
            Ok(fields
                .get(&(field_id as u32))
                .map(|f| f.village_id.is_none())
                .unwrap_or(false))
        }
    }

    fn wrap_coordinate(value: i32, world_size: i32) -> i32 {
        if world_size <= 0 {
            return value;
        }
        let span = world_size * 2 + 1;
        let mut normalized = (value + world_size) % span;
        if normalized < 0 {
            normalized += span;
        }
        normalized - world_size
    }

    #[derive(Default, Clone)]
    pub struct MockUserRepository {
        users: Arc<Mutex<HashMap<Uuid, User>>>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn save(
            &self,
            email: String,
            _password_hash: String,
        ) -> Result<(), ApplicationError> {
            let user = User::new(Uuid::new_v4(), email, "123".to_string());
            self.users.lock().unwrap().insert(user.id, user);
            Ok(())
        }

        async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError> {
            self.users
                .lock()
                .unwrap()
                .values()
                .find(|u| u.email == email)
                .cloned()
                .ok_or_else(|| {
                    ApplicationError::Db(DbError::UserByEmailNotFound(email.to_string()))
                })
        }

        async fn get_by_username(&self, username: &str) -> Result<User, ApplicationError> {
            self.users
                .lock()
                .unwrap()
                .values()
                .find(|u| u.email == username)
                .cloned()
                .ok_or_else(|| {
                    ApplicationError::Db(DbError::UserByUsernameNotFound(username.to_string()))
                })
        }

        async fn get_by_id(&self, id: Uuid) -> Result<User, ApplicationError> {
            self.users
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or(ApplicationError::Db(DbError::UserByIdNotFound(id)))
        }
    }
}
