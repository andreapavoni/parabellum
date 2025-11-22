use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;
use parabellum_types::map::Position;
use parabellum_types::map_flag::MapFlagType;

/// Map flag/mark that can be owned by either a player or an alliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapFlag {
    pub id: Uuid,

    // Ownership (mutually exclusive)
    pub alliance_id: Option<Uuid>,
    pub player_id: Option<Uuid>,

    // Target/Location
    pub target_id: Option<Uuid>,     // For types 0 & 1: target player/alliance ID
    pub position: Option<Position>,  // For type 2: map position

    // Properties
    pub flag_type: MapFlagType,
    pub color: i16,
    pub text: Option<String>,

    // Metadata
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MapFlag {
    /// Creates a new player-owned map flag
    pub fn new_player_flag(
        player_id: Uuid,
        flag_type: MapFlagType,
        color: i16,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            alliance_id: None,
            player_id: Some(player_id),
            target_id: None,
            position: None,
            flag_type,
            color,
            text: None,
            created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Creates a new alliance-owned map flag
    pub fn new_alliance_flag(
        alliance_id: Uuid,
        flag_type: MapFlagType,
        color: i16,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            alliance_id: Some(alliance_id),
            player_id: None,
            target_id: None,
            position: None,
            flag_type,
            color,
            text: None,
            created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Sets the target player or alliance for multi-marks (types 0 & 1)
    pub fn with_target(mut self, target_id: Uuid) -> Self {
        self.target_id = Some(target_id);
        self
    }

    /// Sets the position for custom flags (type 2)
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the text label for custom flags (type 2)
    /// Returns error if text exceeds 50 characters
    pub fn with_text(mut self, text: String) -> Result<Self, GameError> {
        if text.len() > 50 {
            return Err(GameError::MapFlagTextTooLong { length: text.len() });
        }
        self.text = Some(text);
        Ok(self)
    }

    /// Validates color range based on flag type and ownership
    /// Multi-marks (types 0 & 1): 0-9
    /// Custom flags (type 2): 0-10 (player) or 10-20 (alliance)
    pub fn validate_color(&self) -> Result<(), GameError> {
        match self.flag_type {
            MapFlagType::PlayerMark | MapFlagType::AllianceMark => {
                // Multi-marks: colors 0-9
                if self.color < 0 || self.color > 9 {
                    return Err(GameError::InvalidMapFlagColor { 
                        color: self.color, 
                        min: 0, 
                        max: 9 
                    });
                }
            }
            MapFlagType::CustomFlag => {
                // Custom flags: different ranges based on ownership
                if self.alliance_id.is_some() {
                    // Alliance-owned: colors 10-20
                    if self.color < 10 || self.color > 20 {
                        return Err(GameError::InvalidMapFlagColor { 
                            color: self.color, 
                            min: 10, 
                            max: 20 
                        });
                    }
                } else {
                    // Player-owned: colors 0-10
                    if self.color < 0 || self.color > 10 {
                        return Err(GameError::InvalidMapFlagColor { 
                            color: self.color, 
                            min: 0, 
                            max: 10 
                        });
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Validates that the flag has correct target/coordinates based on type
    pub fn validate_target(&self) -> Result<(), GameError> {
        match self.flag_type {
            MapFlagType::PlayerMark | MapFlagType::AllianceMark => {
                // Multi-marks must have target_id
                if self.target_id.is_none() {
                    return Err(GameError::MapFlagMissingTarget);
                }
                // Multi-marks should not have position
                if self.position.is_some() {
                    return Err(GameError::MapFlagInvalidCoordinates);
                }
            }
            MapFlagType::CustomFlag => {
                // Custom flags must have position
                if self.position.is_none() {
                    return Err(GameError::MapFlagMissingCoordinates);
                }
                // Custom flags should not have target_id
                if self.target_id.is_some() {
                    return Err(GameError::MapFlagInvalidTarget);
                }
            }
        }
        
        Ok(())
    }

    /// Validates that exactly one ownership field is set
    pub fn validate_ownership(&self) -> Result<(), GameError> {
        match (self.alliance_id, self.player_id) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            _ => Err(GameError::MapFlagInvalidOwnership),
        }
    }

    /// Validates position is within world bounds
    pub fn validate_position_bounds(&self, world_size: i16) -> Result<(), GameError> {
        if let Some(pos) = &self.position {
            let max_coord = world_size as i32;
            if pos.x.abs() > max_coord || pos.y.abs() > max_coord {
                return Err(GameError::MapFlagPositionOutOfBounds {
                    x: pos.x,
                    y: pos.y,
                    world_size,
                });
            }
        }
        Ok(())
    }

    /// Validates text requirements for custom flags
    pub fn validate_text(&self) -> Result<(), GameError> {
        if self.flag_type == MapFlagType::CustomFlag && self.text.is_none() {
            return Err(GameError::MapFlagMissingText);
        }

        Ok(())
    }

    /// Verifies ownership by a specific player
    pub fn verify_ownership_by_player(&self, player_id: Uuid) -> Result<(), GameError> {
        if self.player_id != Some(player_id) {
            return Err(GameError::MapFlagNotOwnedByPlayer);
        }
        Ok(())
    }

    /// Verifies ownership by a specific alliance
    pub fn verify_ownership_by_alliance(&self, alliance_id: Uuid) -> Result<(), GameError> {
        if self.alliance_id != Some(alliance_id) {
            return Err(GameError::MapFlagNotOwnedByAlliance);
        }
        Ok(())
    }

    /// Full validation of the map flag
    pub fn validate(&self, world_size: i16) -> Result<(), GameError> {
        self.validate_ownership()?;
        self.validate_color()?;
        self.validate_target()?;
        self.validate_position_bounds(world_size)?;
        self.validate_text()?;
        Ok(())
    }

    /// Returns whether this flag is owned by an alliance
    pub fn is_alliance_owned(&self) -> bool {
        self.alliance_id.is_some()
    }

    /// Returns whether this flag is owned by a player
    pub fn is_player_owned(&self) -> bool {
        self.player_id.is_some()
    }

    /// Gets the flag type as enum
    pub fn get_flag_type(&self) -> MapFlagType {
        self.flag_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_color_range_multi_marks() {
        let player_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();

        // Player mark with valid color (0-9)
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::PlayerMark,
            5,
            created_by,
        );
        assert!(flag.validate_color().is_ok());

        // Player mark with invalid color
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::PlayerMark,
            10,
            created_by,
        );
        assert!(flag.validate_color().is_err());
    }

    #[test]
    fn test_validate_color_range_custom_flags_player() {
        let player_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        
        // Player-owned custom flag with valid color (0-10)
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::CustomFlag,
            10,
            created_by,
        );
        assert!(flag.validate_color().is_ok());
        
        // Player-owned custom flag with invalid color
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::CustomFlag,
            11,
            created_by,
        );
        assert!(flag.validate_color().is_err());
    }

    #[test]
    fn test_validate_color_range_custom_flags_alliance() {
        let alliance_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        
        // Alliance-owned custom flag with valid color (10-20)
        let flag = MapFlag::new_alliance_flag(
            alliance_id,
            MapFlagType::CustomFlag,
            15,
            created_by,
        );
        assert!(flag.validate_color().is_ok());
        
        // Alliance-owned custom flag with invalid color
        let flag = MapFlag::new_alliance_flag(
            alliance_id,
            MapFlagType::CustomFlag,
            9,
            created_by,
        );
        assert!(flag.validate_color().is_err());
    }

    #[test]
    fn test_validate_target_player_mark() {
        let player_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        
        // Valid player mark with target
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::PlayerMark,
            3,
            created_by,
        ).with_target(target_id);
        assert!(flag.validate_target().is_ok());
        
        // Invalid player mark without target
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::PlayerMark,
            3,
            created_by,
        );
        assert!(flag.validate_target().is_err());
    }

    #[test]
    fn test_validate_target_custom_flag() {
        let player_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        
        // Valid custom flag with coordinates
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::CustomFlag,
            5,
            created_by,
        ).with_position(Position { x: 100, y: 50 });
        assert!(flag.validate_target().is_ok());
        
        // Invalid custom flag without coordinates
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::CustomFlag,
            5,
            created_by,
        );
        assert!(flag.validate_target().is_err());
    }

    #[test]
    fn test_text_too_long() {
        let player_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();

        let long_text = "This is a very long text that exceeds fifty characters limit";
        let result = MapFlag::new_player_flag(
            player_id,
            MapFlagType::CustomFlag,
            2,
            created_by,
        ).with_text(long_text.to_string());

        assert!(result.is_err());
        match result.unwrap_err() {
            GameError::MapFlagTextTooLong { length } => {
                assert_eq!(length, long_text.len());
            },
            _ => panic!("Expected MapFlagTextTooLong error"),
        }
    }

    #[test]
    fn test_validate_ownership() {
        let player_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        
        // Valid player ownership
        let flag = MapFlag::new_player_flag(
            player_id,
            MapFlagType::PlayerMark,
            3,
            created_by,
        );
        assert!(flag.validate_ownership().is_ok());
        assert!(flag.is_player_owned());
        assert!(!flag.is_alliance_owned());
    }
}
