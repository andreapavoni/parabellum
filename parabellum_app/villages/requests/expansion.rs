use uuid::Uuid;

/// Request expansion culture information for one village owner.
#[derive(Debug, Clone, Copy)]
pub struct GetExpansionCultureInfoRequest {
    /// Player requesting expansion information.
    pub player_id: Uuid,
    /// Village used as the expansion context.
    pub village_id: u32,
    /// Server speed used for next culture-point requirement calculation.
    pub server_speed: i8,
}
