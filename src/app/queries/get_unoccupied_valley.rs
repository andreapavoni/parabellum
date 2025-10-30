use std::sync::Arc;

use anyhow::Result;

use crate::{
    game::models::map::{MapQuadrant, Valley},
    repository::MapRepository,
};

#[derive(Clone)]
pub struct GetUnoccupiedValley {
    pub quadrant: MapQuadrant,
}

impl GetUnoccupiedValley {
    pub fn new(quadrant: Option<MapQuadrant>) -> Self {
        Self {
            quadrant: quadrant.unwrap_or(MapQuadrant::NorthEast),
        }
    }
}

pub struct GetUnoccupiedValleyHandler<'a> {
    repo: Arc<dyn MapRepository + 'a>,
}

impl<'a> GetUnoccupiedValleyHandler<'a> {
    pub fn new(repo: Arc<dyn MapRepository + 'a>) -> Self {
        Self { repo }
    }

    pub async fn handle(&self, query: GetUnoccupiedValley) -> Result<Valley> {
        self.repo.find_unoccupied_valley(&query.quadrant).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::models::map::{Position, ValleyTopology},
        repository::MapRepository,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock Repository
    struct MockMapRepository;

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(&self, quadrant: &MapQuadrant) -> Result<Valley> {
            let pos = match quadrant {
                MapQuadrant::NorthEast => Position { x: 10, y: 10 },
                MapQuadrant::SouthEast => Position { x: 10, y: -10 },
                MapQuadrant::SouthWest => Position { x: -10, y: -10 },
                MapQuadrant::NorthWest => Position { x: -10, y: 10 },
            };

            Ok(Valley::new(pos, ValleyTopology(4, 4, 4, 6)))
        }

        async fn get_field_by_id(
            &self,
            _id: i32,
        ) -> Result<Option<crate::game::models::map::MapField>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_get_unoccupied_valley_handler() {
        let mock_repo = Arc::new(MockMapRepository);
        let handler = GetUnoccupiedValleyHandler::new(mock_repo);

        let query = GetUnoccupiedValley::new(Some(MapQuadrant::NorthEast));
        let valley = handler.handle(query).await.unwrap();

        assert_eq!(valley.position.x, 10);
        assert_eq!(valley.position.y, 10);

        let query_sw = GetUnoccupiedValley::new(Some(MapQuadrant::SouthWest));
        let valley_sw = handler.handle(query_sw).await.unwrap();

        assert_eq!(valley_sw.position.x, -10);
        assert_eq!(valley_sw.position.y, -10);
    }
}
