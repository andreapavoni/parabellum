use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum AttackType {
    Raid,   // Raid
    Normal, // Attack / Siege / Conquer
}
