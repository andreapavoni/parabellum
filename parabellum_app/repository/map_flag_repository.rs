use parabellum_core::ApplicationError;
use parabellum_game::models::map_flag::MapFlag;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait MapFlagRepository: Send + Sync {
    /// Save a new map flag
    async fn save(&self, flag: &MapFlag) -> Result<(), ApplicationError>;
    
    /// Get a map flag by ID
    async fn get_by_id(&self, id: Uuid) -> Result<MapFlag, ApplicationError>;
    
    /// Get all map flags owned by a player
    async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError>;
    
    /// Get all map flags owned by an alliance
    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError>;
    
    /// Get all map flags at specific coordinates
    async fn get_by_coordinates(&self, x: i32, y: i32) -> Result<Vec<MapFlag>, ApplicationError>;
    
    /// Get all map flags targeting a specific player or alliance
    async fn get_by_target_id(&self, target_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError>;
    
    /// Count map flags by owner and optionally by type
    /// If player_id is Some, counts player flags; if alliance_id is Some, counts alliance flags
    /// If flag_type is Some, filters by that type; otherwise counts all types
    async fn count_by_owner(
        &self,
        player_id: Option<Uuid>,
        alliance_id: Option<Uuid>,
        flag_type: Option<i16>,
    ) -> Result<i64, ApplicationError>;
    
    /// Update an existing map flag (color and/or text)
    async fn update(&self, flag: &MapFlag) -> Result<(), ApplicationError>;
    
    /// Delete a map flag
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
}
