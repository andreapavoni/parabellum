use parabellum_types::map::Position;

/// Minimal village info for display purposes (name and position)
#[derive(Debug, Clone, PartialEq)]
pub struct VillageInfo {
    pub id: u32,
    pub name: String,
    pub position: Position,
}
