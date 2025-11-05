use parabellum_types::map::{OasisTopology, Position, ValleyTopology};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parabellum_core::GameError;

use super::village::ProductionBonus;
use crate::models::army::Army;

#[derive(Debug, Clone)]
pub enum MapQuadrant {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Valley {
    pub id: u32,
    pub position: Position,
    pub topology: ValleyTopology,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
}

impl Valley {
    pub fn new(position: Position, topology: ValleyTopology) -> Self {
        Self {
            id: position.to_id(100),
            position,
            topology,
            player_id: None,
            village_id: None,
        }
    }
}

impl TryFrom<MapField> for Valley {
    type Error = GameError;

    fn try_from(value: MapField) -> Result<Self, Self::Error> {
        match value.topology {
            MapFieldTopology::Valley(topology) => Ok(Self {
                id: value.id,
                player_id: value.player_id,
                village_id: value.village_id,
                position: value.position,
                topology,
            }),
            _ => Err(GameError::InvalidValley(value.id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Oasis {
    pub id: u32,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
    pub position: Position,
    pub topology: OasisTopology,
    pub reinforcements: Vec<Army>,
}

impl Oasis {
    pub fn bonus(&self) -> ProductionBonus {
        let mut lumber: u8 = 0;
        let mut clay: u8 = 0;
        let mut iron: u8 = 0;
        let mut crop: u8 = 0;

        match &self.topology {
            OasisTopology::Lumber => lumber += 25,
            OasisTopology::LumberCrop => {
                lumber += 25;
                crop += 25;
            }
            OasisTopology::Clay => clay += 25,
            OasisTopology::ClayCrop => {
                clay += 25;
                crop += 25;
            }
            OasisTopology::Iron => iron += 25,
            OasisTopology::IronCrop => {
                iron += 25;
                crop += 25;
            }
            OasisTopology::Crop => crop += 25,
            OasisTopology::Crop50 => crop += 50,
        }

        ProductionBonus {
            lumber,
            clay,
            iron,
            crop,
        }
    }
}

impl TryFrom<MapField> for Oasis {
    type Error = GameError;

    fn try_from(value: MapField) -> Result<Self, Self::Error> {
        match value.topology {
            MapFieldTopology::Oasis(topology) => Ok(Self {
                id: value.id,
                player_id: value.player_id,
                village_id: value.village_id,
                position: value.position,
                topology,
                reinforcements: vec![],
            }),
            _ => Err(GameError::InvalidOasis(value.id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapFieldTopology {
    Oasis(OasisTopology),
    Valley(ValleyTopology),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapField {
    pub id: u32,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
    pub position: Position,
    pub topology: MapFieldTopology,
}

impl From<Valley> for MapField {
    fn from(valley: Valley) -> Self {
        MapField {
            id: valley.id,
            player_id: valley.player_id,
            village_id: valley.village_id,
            position: valley.position,
            topology: MapFieldTopology::Valley(valley.topology),
        }
    }
}

impl From<Oasis> for MapField {
    fn from(oasis: Oasis) -> Self {
        MapField {
            id: oasis.id,
            player_id: oasis.player_id,
            village_id: oasis.village_id,
            position: oasis.position,
            topology: MapFieldTopology::Oasis(oasis.topology),
        }
    }
}

pub fn generate_new_map(world_size: i32) -> Vec<MapField> {
    let mut map: Vec<MapField> = vec![];

    for x in -world_size..world_size {
        for y in -world_size..world_size {
            let mut rng = rand::thread_rng();
            let n = rng.gen_range(0..1001);
            let position = Position { x, y };
            let id = position.to_id(world_size);

            if (x == y && x == 0) || (x == y && x == world_size) || (x == y && x == -world_size) {
                map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                    id,
                    position,
                });
                continue;
            }

            match n {
                0..=10 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(3, 3, 3, 9)),
                    id,
                    position,
                }),
                11..=90 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(3, 4, 5, 6)),
                    id,
                    position,
                }),
                91..=400 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                    id,
                    position,
                }),
                401..=480 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 5, 3, 6)),
                    id,
                    position,
                }),
                481..=560 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(5, 4, 3, 6)),
                    id,
                    position,
                }),
                561..=570 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(1, 1, 1, 15)),
                    id,
                    position,
                }),
                571..=600 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 3, 7)),
                    id,
                    position,
                }),
                601..=630 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(3, 4, 4, 7)),
                    id,
                    position,
                }),
                631..=660 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 3, 4, 7)),
                    id,
                    position,
                }),
                661..=740 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(3, 5, 4, 6)),
                    id,
                    position,
                }),
                741..=820 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 3, 5, 6)),
                    id,
                    position,
                }),
                821..=900 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    topology: MapFieldTopology::Valley(ValleyTopology(5, 3, 4, 6)),
                    id,
                    position,
                }),
                901..=908 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::Lumber),
                    position,
                }),
                909..=924 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::LumberCrop),
                    position,
                }),
                925..=932 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::Clay),
                    position,
                }),
                933..=948 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::ClayCrop),
                    position,
                }),
                949..=956 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::Iron),
                    position,
                }),
                957..=972 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::IronCrop),
                    position,
                }),
                973..=980 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::Crop),
                    position,
                }),
                981..=1000 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    topology: MapFieldTopology::Oasis(OasisTopology::Crop50),
                    position,
                }),
                _ => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    position,
                    topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                }),
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{MapFieldTopology, OasisTopology, ValleyTopology, generate_new_map};
    use crate::models::map::Position;

    #[test]
    fn test_position_id() {
        let p = Position { x: 14, y: 28 };
        assert_eq!(p.to_id(100), 14587);
    }

    #[test]
    fn test_position_distance() {
        let world_size = 200;
        let p = Position { x: 10, y: 10 };

        assert_eq!(p.distance(&Position { x: 10, y: 10 }, world_size), 0);
        assert_eq!(p.distance(&Position { x: -10, y: -10 }, world_size), 28);
        assert_eq!(p.distance(&Position { x: 21, y: 45 }, world_size), 36);
        assert_eq!(p.distance(&Position { x: 110, y: -110 }, world_size), 156);
        assert_eq!(p.distance(&Position { x: 200, y: 200 }, world_size), 268);
    }

    #[test]
    fn test_generate_new_map() {
        let world_size = 100;
        let expected_size = world_size * world_size * 4; // world_size = 10 => 40_000 map fields
        let map = generate_new_map(world_size);

        assert_eq!(map.clone().len(), expected_size as usize);
    }

    #[test]
    // This test it's just for debugging purposes. It prints map fields topology with
    // percentuals about each field type.
    fn test_generated_map_topology() {
        let world_size = 100;
        let map = generate_new_map(world_size);
        let mut oases: HashMap<OasisTopology, u32> = HashMap::new();
        let mut valleys: HashMap<ValleyTopology, u32> = HashMap::new();

        for f in map.clone() {
            match f.topology {
                MapFieldTopology::Oasis(to) => {
                    *oases.entry(to).or_insert(0) += 1;
                }
                MapFieldTopology::Valley(tv) => {
                    *valleys.entry(tv).or_insert(0) += 1;
                }
            }
        }

        let map_size: f64 = map.len() as f64;

        println!("Oases:");
        for (v, o) in oases.clone() {
            println!("\t{:?}: {} ({}%)", v, o, (100.0 * o as f64 / map_size));
        }
        println!("Total: {}", oases.values().sum::<u32>());

        println!("\n\nValleys:");
        for (v, o) in valleys.clone() {
            println!("\t{:?}: {} ({}%)", v, o, (100.0 * o as f64 / map_size));
        }
        println!("Total: {}", valleys.values().sum::<u32>());
    }
}
