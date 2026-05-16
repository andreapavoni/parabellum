use parabellum_game::models::map::MapField;
use parabellum_types::tribe::Tribe;

#[derive(Debug, Clone)]
pub struct MapRegionTile {
    pub field: MapField,
    pub village_name: Option<String>,
    pub village_population: Option<i32>,
    pub player_name: Option<String>,
    pub tribe: Option<Tribe>,
}
