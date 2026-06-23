use parabellum_types::map::Position;

/// Compact village reference for display and relationship labels.
#[derive(Debug, Clone, PartialEq)]
pub struct VillageReference {
    /// Village id.
    pub id: u32,
    /// Village display name.
    pub name: String,
    /// Village map position.
    pub position: Position,
}
