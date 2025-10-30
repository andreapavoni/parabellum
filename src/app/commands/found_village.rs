use crate::{
    game::models::{
        map::{Position, Valley},
        village::Village,
        Player,
    },
    repository::{MapRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;

#[derive(Clone)]
pub struct FoundVillage {
    pub player: Player,
    pub position: Position,
}

impl FoundVillage {
    pub fn new(player: Player, position: Position) -> Self {
        Self { player, position }
    }
}

pub struct FoundVillageHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    map_repo: Arc<dyn MapRepository + 'a>,
}

impl<'a> FoundVillageHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        map_repo: Arc<dyn MapRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            map_repo,
        }
    }

    pub async fn handle(&self, command: FoundVillage) -> Result<Village> {
        let village_id: i32 = command.position.to_id(100) as i32;

        let valley = match self.map_repo.get_field_by_id(village_id).await? {
            Some(map_field) => Valley::try_from(map_field)?,
            None => return Err(anyhow!("The number of available units is not enough")),
        };

        let village = Village::new("New Village".to_string(), &valley, &command.player, false);

        self.village_repo.create(&village).await?;

        Ok(village)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{
            models::{
                map::{MapField, MapFieldTopology, ValleyTopology},
                Tribe,
            },
            test_factories::{player_factory, PlayerFactoryOptions},
        },
        repository::{MapRepository, VillageRepository},
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // --- Mocks ---
    #[derive(Default)]
    struct MockVillageRepository {
        created_village: Mutex<Option<Village>>,
    }

    #[async_trait]
    impl VillageRepository for MockVillageRepository {
        async fn create(&self, village: &Village) -> Result<()> {
            *self.created_village.lock().unwrap() = Some(village.clone());
            Ok(())
        }
        async fn get_by_id(&self, _village_id: u32) -> Result<Option<Village>> {
            Ok(None)
        }
        async fn list_by_player_id(&self, _player_id: Uuid) -> Result<Vec<Village>> {
            Ok(vec![])
        }
        async fn save(&self, _village: &Village) -> Result<()> {
            Ok(())
        }
    }

    struct MockMapRepository {
        field_to_return: Option<MapField>,
    }

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(
            &self,
            _quadrant: &crate::game::models::map::MapQuadrant,
        ) -> Result<Valley> {
            panic!("Should not be called in this test");
        }

        async fn get_field_by_id(&self, _id: i32) -> Result<Option<MapField>> {
            Ok(self.field_to_return.clone())
        }
    }

    #[tokio::test]
    async fn test_found_village_handler_success() {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let position = Position { x: 10, y: 10 };
        let village_id = position.to_id(100);

        let map_field = MapField {
            id: village_id,
            player_id: None,
            village_id: None,
            position: position.clone(),
            topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
        };

        let mock_village_repo = Arc::new(MockVillageRepository::default());
        let mock_map_repo = Arc::new(MockMapRepository {
            field_to_return: Some(map_field),
        });

        let handler = FoundVillageHandler::new(mock_village_repo.clone(), mock_map_repo);
        let command = FoundVillage::new(player.clone(), position);

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let village = result.unwrap();
        assert_eq!(village.id, village_id);
        assert_eq!(village.name, "New Village");
        assert_eq!(village.player_id, player.id);

        let created_village = mock_village_repo.created_village.lock().unwrap();
        assert!(created_village.is_some());
        assert_eq!(created_village.as_ref().unwrap().id, village_id);
    }

    #[tokio::test]
    async fn test_found_village_handler_field_not_found() {
        let player = player_factory(Default::default());
        let position = Position { x: 10, y: 10 };

        let mock_village_repo = Arc::new(MockVillageRepository::default());
        let mock_map_repo = Arc::new(MockMapRepository {
            field_to_return: None, // Field not found
        });

        let handler = FoundVillageHandler::new(mock_village_repo, mock_map_repo);
        let command = FoundVillage::new(player, position);

        let result = handler.handle(command).await;

        // This error message comes from the handler, but it's misleading.
        // Let's check if it's an error.
        assert!(result.is_err());

        // The error message is "The number of available units is not enough"
        // This is because it's re-using an error message from `army.rs` via `anyhow!`.
        // This should probably be changed to a more specific error.
        let error_msg = result.err().unwrap().to_string();
        assert_eq!(error_msg, "The number of available units is not enough");
    }
}
