use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum BuildingGroup {
    Infrastructure,
    Resources,
    Military,
}

#[derive(Debug, Clone)]
pub struct BuildingRequirement(pub BuildingName, pub u8);

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum BuildingName {
    Woodcutter,
    ClayPit,
    IronMine,
    Cropland,
    Sawmill,
    Brickyard,
    IronFoundry,
    GrainMill,
    Bakery,
    Warehouse,
    Granary,
    Smithy,
    TournamentSquare,
    MainBuilding,
    RallyPoint,
    Marketplace,
    Embassy,
    Barracks,
    Stable,
    Workshop,
    Academy,
    Cranny,
    TownHall,
    Residence,
    Palace,
    Treasury,
    TradeOffice,
    GreatBarracks,
    GreatStable,
    CityWall,
    EarthWall,
    Palisade,
    StonemansionLodge,
    Brewery,
    Trapper,
    HeroMansion,
    GreatWarehouse,
    GreatGranary,
    WonderOfTheWorld,
    AncientConstructionPlan,
    HorseDrinkingTrough,
    GreatWorkshop,
}

impl fmt::Display for BuildingName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            BuildingName::Woodcutter => "Woodcutter",
            BuildingName::ClayPit => "Clay Pit",
            BuildingName::IronMine => "Iron Mine",
            BuildingName::Cropland => "Cropland",
            BuildingName::Sawmill => "Sawmill",
            BuildingName::Brickyard => "Brickyard",
            BuildingName::IronFoundry => "Iron Foundry",
            BuildingName::GrainMill => "Grain Mill",
            BuildingName::Bakery => "Bakery",
            BuildingName::Warehouse => "Warehouse",
            BuildingName::Granary => "Granary",
            BuildingName::Smithy => "Smithy",
            BuildingName::TournamentSquare => "Tournament Square",
            BuildingName::MainBuilding => "Main Building",
            BuildingName::RallyPoint => "Rally Point",
            BuildingName::Marketplace => "Marketplace",
            BuildingName::Embassy => "Embassy",
            BuildingName::Barracks => "Barracks",
            BuildingName::Stable => "Stable",
            BuildingName::Workshop => "Workshop",
            BuildingName::Academy => "Academy",
            BuildingName::Cranny => "Cranny",
            BuildingName::TownHall => "Town Hall",
            BuildingName::Residence => "Residence",
            BuildingName::Palace => "Palace",
            BuildingName::Treasury => "Treasury",
            BuildingName::TradeOffice => "Trade Office",
            BuildingName::GreatBarracks => "Great Barracks",
            BuildingName::GreatStable => "Great Stable",
            BuildingName::CityWall => "City Wall",
            BuildingName::EarthWall => "Earth Wall",
            BuildingName::Palisade => "Palisade",
            BuildingName::StonemansionLodge => "Stonemason Lodge",
            BuildingName::Brewery => "Brewery",
            BuildingName::Trapper => "Trapper",
            BuildingName::HeroMansion => "Hero Mansion",
            BuildingName::GreatWarehouse => "Great Warehouse",
            BuildingName::GreatGranary => "Great Granary",
            BuildingName::WonderOfTheWorld => "Wonder of the World",
            BuildingName::AncientConstructionPlan => "Ancient Construction Plan",
            BuildingName::HorseDrinkingTrough => "Horse Drinking Trough",
            BuildingName::GreatWorkshop => "Great Workshop",
        };

        f.write_str(name)
    }
}
