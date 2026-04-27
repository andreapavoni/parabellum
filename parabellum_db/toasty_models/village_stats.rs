use uuid::Uuid;

#[derive(Debug, Clone, toasty::Model)]
#[table = "villages"]
pub struct VillageStatsRecord {
    #[key]
    pub id: i32,

    #[index]
    pub player_id: Uuid,

    pub population: i32,
    pub culture_points: i32,
    pub culture_points_production: i32,
}
