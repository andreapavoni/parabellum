use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValleyTopology(pub u8, pub u8, pub u8, pub u8);

impl ValleyTopology {
    pub fn lumber(&self) -> u8 {
        self.0
    }
    pub fn clay(&self) -> u8 {
        self.1
    }
    pub fn iron(&self) -> u8 {
        self.2
    }
    pub fn crop(&self) -> u8 {
        self.3
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn to_id(&self, world_size: i32) -> u32 {
        ((world_size - self.y) * (world_size * 2 + 1) + (world_size + self.x + 1)) as u32
    }

    /// Returns the distance between two points.
    pub fn distance(&self, position: &Position, world_size: i32) -> u32 {
        let mut x_diff = (self.x - position.x).abs();
        let mut y_diff = (self.y - position.y).abs();

        if x_diff > world_size {
            x_diff = (2 * world_size + 1) - x_diff;
        }

        if y_diff > world_size {
            y_diff = (2 * world_size + 1) - y_diff;
        }

        (((x_diff * x_diff) + (y_diff * y_diff)) as f64).sqrt() as u32
    }

    pub fn calculate_travel_time_secs(
        &self,
        position: Position,
        speed: u8,
        world_size: i32,
        server_speed: u8,
    ) -> u32 {
        let distance = self.distance(&position, world_size);

        let travel_time_secs = distance as f64 / speed as f64 * 3600.0;

        (travel_time_secs / server_speed as f64).floor() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum OasisTopology {
    Lumber,
    LumberCrop,
    Clay,
    ClayCrop,
    Iron,
    IronCrop,
    Crop,
    Crop50,
}
