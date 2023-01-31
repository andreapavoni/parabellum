use rand::Rng;

use super::village::ProductionBonus;

pub const WORLD_MAX_SIZE: i32 = 400;

pub trait MapField {
    fn id(&self) -> u64;
    fn position(&self) -> &Position;
    fn field(&self) -> Box<dyn MapField>;
}

#[derive(Debug, Clone)]
pub struct Valley {
    pub id: u64,
    pub position: Position,
    pub topology: ValleyTopology,
}

impl MapField for Valley {
    fn id(&self) -> u64 {
        self.id.clone()
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn field(&self) -> Box<dyn MapField> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
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
    pub fn to_id(&self, max: i32) -> u64 {
        ((max - self.y) * (max * 2 + 1) + (max + self.x + 1)) as u64
    }

    pub fn distance(&self, position: &Position, max: i32) -> u32 {
        let mut x_diff = (self.x - position.x).abs();
        let mut y_diff = (self.y - position.y).abs();

        if x_diff > max {
            x_diff = (2 * max + 1) - x_diff;
        }

        if y_diff > max {
            y_diff = (2 * max + 1) - y_diff;
        }

        (((x_diff * x_diff) + (y_diff * y_diff)) as f64).sqrt() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl MapField for Oasis {
    fn id(&self) -> u64 {
        self.id.clone()
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn field(&self) -> Box<dyn MapField> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldVariant {
    Oasis,
    Valley,
}

// TODO: usare struct per la map (da usare a db). nella map e nella generate bisogna includere tutto quello che serve su db prendendolo da oasis o valley.
pub fn generate_new_map(max: i32) -> Vec<Box<dyn MapField>> {
    let mut map: Vec<Box<dyn MapField>> = vec![];

    for x in -max..max {
        for y in -max..max {
            let mut rng = rand::thread_rng();
            let n = rng.gen_range(0..1001);
            let position = Position { x, y };
            let id = position.to_id(max);

            if (x == y && x == 0) || (x == y && x == max) || (x == y && x == -max) {
                map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 4, 4, 6),
                    id,
                    position,
                }));
                continue;
            }

            match n {
                1..=10 => map.push(Box::new(Valley {
                    topology: ValleyTopology(3, 3, 3, 9),
                    id,
                    position,
                })),
                11..=90 => map.push(Box::new(Valley {
                    topology: ValleyTopology(3, 4, 5, 6),
                    id,
                    position,
                })),
                91..=400 => map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 4, 4, 6),
                    id,
                    position,
                })),
                401..=480 => map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 5, 3, 6),
                    id,
                    position,
                })),
                481..=560 => map.push(Box::new(Valley {
                    topology: ValleyTopology(5, 4, 3, 6),
                    id,
                    position,
                })),
                561..=570 => map.push(Box::new(Valley {
                    topology: ValleyTopology(1, 1, 1, 15),
                    id,
                    position,
                })),
                571..=600 => map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 4, 3, 7),
                    id,
                    position,
                })),
                601..=630 => map.push(Box::new(Valley {
                    topology: ValleyTopology(3, 4, 4, 7),
                    id,
                    position,
                })),
                631..=660 => map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 3, 4, 7),
                    id,
                    position,
                })),
                661..=740 => map.push(Box::new(Valley {
                    topology: ValleyTopology(3, 5, 4, 6),
                    id,
                    position,
                })),
                741..=820 => map.push(Box::new(Valley {
                    topology: ValleyTopology(4, 3, 5, 6),
                    id,
                    position,
                })),
                821..=900 => map.push(Box::new(Valley {
                    topology: ValleyTopology(5, 3, 4, 6),
                    id,
                    position,
                })),
                901..=908 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::Lumber,
                    position,
                })),
                909..=924 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::LumberCrop,
                    position,
                })),
                925..=932 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::Clay,
                    position,
                })),
                933..=948 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::ClayCrop,
                    position,
                })),
                949..=956 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::Iron,
                    position,
                })),
                957..=972 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::IronCrop,
                    position,
                })),
                973..=980 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::Crop,
                    position,
                })),
                981..=1000 => map.push(Box::new(Oasis {
                    id,
                    variant: OasisVariant::Crop50,
                    position,
                })),
                _ => map.push(Box::new(Valley {
                    id,
                    position,
                    topology: ValleyTopology(4, 4, 4, 6),
                })),
            }
        }
    }
    map
}
