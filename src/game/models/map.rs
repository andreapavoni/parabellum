use rand::Rng;

use super::village::ProductionBonus;

pub const WORLD_MAX_SIZE: i32 = 400;

#[derive(Debug, Clone)]
pub struct Valley {
    pub id: u64,
    pub position: Position,
    pub topology: ValleyTopology,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ValleyTopology(u8, u8, u8, u8);

impl ValleyTopology {
    pub fn lumber(&self) -> u8 {
        self.0
    }
    pub fn clay(&self) -> u8 {
        self.0
    }
    pub fn iron(&self) -> u8 {
        self.0
    }
    pub fn crop(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn to_id(&self, world_size: i32) -> u64 {
        ((world_size - self.y) * (world_size * 2 + 1) + (world_size + self.x + 1)) as u64
    }

    // Returns the distance between two points.
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OasisVariant {
    Lumber,
    LumberCrop,
    Clay,
    ClayCrop,
    Iron,
    IronCrop,
    Crop,
    Crop50,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oasis {
    id: u64,
    position: Position,
    variant: OasisVariant,
}

impl Oasis {
    pub fn bonus(&self) -> ProductionBonus {
        let mut lumber: u8 = 0;
        let mut clay: u8 = 0;
        let mut iron: u8 = 0;
        let mut crop: u8 = 0;

        match &self.variant {
            OasisVariant::Lumber => lumber += 25,
            OasisVariant::LumberCrop => {
                lumber += 25;
                crop += 25;
            }
            OasisVariant::Clay => clay += 25,
            OasisVariant::ClayCrop => {
                clay += 25;
                crop += 25;
            }
            OasisVariant::Iron => iron += 25,
            OasisVariant::IronCrop => {
                iron += 25;
                crop += 25;
            }
            OasisVariant::Crop => crop += 25,
            OasisVariant::Crop50 => crop += 50,
        }

        ProductionBonus {
            lumber,
            clay,
            iron,
            crop,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Oasis(OasisVariant),
    Valley(ValleyTopology),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapField {
    pub id: u64,
    pub player_id: Option<String>,
    pub village_id: Option<u64>,
    pub position: Position,
    pub field: FieldType,
}

// TODO: usare struct per la map (da usare a db). nella map e nella generate bisogna includere tutto quello che serve su db prendendolo da oasis o valley.
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
                    field: FieldType::Valley(ValleyTopology(4, 4, 4, 6)),
                    id,
                    position,
                });
                continue;
            }

            match n {
                0..=10 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(3, 3, 3, 9)),
                    id,
                    position,
                }),
                11..=90 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(3, 4, 5, 6)),
                    id,
                    position,
                }),
                91..=400 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(4, 4, 4, 6)),
                    id,
                    position,
                }),
                401..=480 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(4, 5, 3, 6)),
                    id,
                    position,
                }),
                481..=560 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(5, 4, 3, 6)),
                    id,
                    position,
                }),
                561..=570 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(1, 1, 1, 15)),
                    id,
                    position,
                }),
                571..=600 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(4, 4, 3, 7)),
                    id,
                    position,
                }),
                601..=630 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(3, 4, 4, 7)),
                    id,
                    position,
                }),
                631..=660 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(4, 3, 4, 7)),
                    id,
                    position,
                }),
                661..=740 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(3, 5, 4, 6)),
                    id,
                    position,
                }),
                741..=820 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(4, 3, 5, 6)),
                    id,
                    position,
                }),
                821..=900 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    field: FieldType::Valley(ValleyTopology(5, 3, 4, 6)),
                    id,
                    position,
                }),
                901..=908 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::Lumber),
                    position,
                }),
                909..=924 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::LumberCrop),
                    position,
                }),
                925..=932 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::Clay),
                    position,
                }),
                933..=948 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::ClayCrop),
                    position,
                }),
                949..=956 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::Iron),
                    position,
                }),
                957..=972 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::IronCrop),
                    position,
                }),
                973..=980 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::Crop),
                    position,
                }),
                981..=1000 => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    field: FieldType::Oasis(OasisVariant::Crop50),
                    position,
                }),
                _ => map.push(MapField {
                    player_id: None,
                    village_id: None,
                    id,
                    position,
                    field: FieldType::Valley(ValleyTopology(4, 4, 4, 6)),
                }),
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{generate_new_map, FieldType, OasisVariant, ValleyTopology};
    use crate::game::models::map::Position;

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
    fn test_generated_map_topology() {
        let world_size = 100;
        let map = generate_new_map(world_size);
        let mut oases: HashMap<OasisVariant, u64> = HashMap::new();
        let mut valleys: HashMap<ValleyTopology, u64> = HashMap::new();

        for f in map.clone() {
            match f.field {
                FieldType::Oasis(to) => {
                    *oases.entry(to).or_insert(0) += 1;
                }
                FieldType::Valley(tv) => {
                    *valleys.entry(tv).or_insert(0) += 1;
                }
            }
        }

        let map_size: f64 = map.len() as f64;

        println!("Oases:");
        for (v, o) in oases.clone() {
            println!("\t{:?}: {} ({}%)", v, o, (100.0 * o as f64 / map_size));
        }
        println!("Total: {}", oases.values().sum::<u64>());

        println!("\n\nValleys:");
        for (v, o) in valleys.clone() {
            println!("\t{:?}: {} ({}%)", v, o, (100.0 * o as f64 / map_size));
        }
        println!("Total: {}", valleys.values().sum::<u64>());
    }
}
