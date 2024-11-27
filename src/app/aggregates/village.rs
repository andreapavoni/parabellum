use std::collections::HashMap;

use crate::{
    app::events::GameEvent,
    game::models::{
        army::Army,
        buildings::Building,
        map::{Oasis, Position},
        village::{StockCapacity, VillageProduction},
        SmithyUpgrades, Tribe,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct VillageAggregate {
    pub id: u32,
    pub name: String,
    pub player_id: Uuid,
    pub position: Position,
    pub tribe: Tribe,
    pub buildings: HashMap<u8, Building>,
    pub oases: Vec<Oasis>,
    pub population: u32,
    pub army: Army,
    pub reinforcements: Vec<Army>,
    pub loyalty: u8,
    pub production: VillageProduction,
    pub is_capital: bool,
    pub smithy: SmithyUpgrades,
    pub stocks: StockCapacity,
    pub updated_at: DateTime<Utc>,
}

// We don't care about the default starting values, any placeholder will be ok, because the real
// data will be loaded/stored when the game is started.
impl Default for VillageAggregate {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Player".to_string(),
            player_id: Uuid::new_v4(),
            position: Position { x: 0, y: 0 },
            tribe: Tribe::Roman,
            buildings: HashMap::new(),
            oases: vec![],
            population: 2,
            army: Army {
                village_id: 0,
                player_id: Uuid::new_v4(),
                units: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                smithy: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                tribe: Tribe::Roman,
            },
            reinforcements: vec![],
            loyalty: 100,
            production: VillageProduction::default(),
            is_capital: true,
            smithy: SmithyUpgrades(0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            stocks: StockCapacity::default(),
            updated_at: DateTime::Utc::now(),
        }
    }
}

#[async_trait]
impl Aggregate for VillageAggregate {
    type Event = GameEvent;

    async fn apply(&mut self, event: &Self::Event) {
        match event {
            // GameEvent::GameStarted {
            //     aggregate_id: _,
            //     player_1,
            //     player_2,
            //     goal,
            // } => {
            //     // Game is started, we can populate the aggregate with the correct data.
            //     self.status = GameStatus::Playing;
            //     self.goal = *goal;
            //     self.player_1 = player_1.clone();
            //     self.player_2 = player_2.clone();
            // }
            GameEvent::PlayerRegistered(_player) => {}
            GameEvent::VillageFounded(village) => {}
            _ => {}
        };
    }

    fn aggregate_id(&self) -> Uuid {
        self.id
    }

    fn set_aggregate_id(&mut self, id: Uuid) {
        self.id = id;
    }
}
