const BUILDING_ICON_SIZE = 64;

export const BUILDING_ICON_BY_NAME: Record<string, string> = {
  Academy: "academy.png",
  Bakery: "bakery.png",
  Barracks: "barracks.png",
  Brewery: "brewery.png",
  Brickyard: "brickyard.png",
  CityWall: "city_wall.png",
  ClayPit: "clay_pit.png",
  Cranny: "cranny.png",
  Cropland: "cropland.png",
  EarthWall: "earth_wall.png",
  Embassy: "embassy.png",
  GrainMill: "grain_mill.png",
  Granary: "granary.png",
  GreatBarracks: "great_barracks.png",
  GreatGranary: "great_granary.png",
  GreatStable: "great_stable.png",
  GreatWarehouse: "great_warehouse.png",
  HeroMansion: "hero_mansion.png",
  HorseDrinkingTrough: "horse_drinking_trough.png",
  IronFoundry: "iron_foundry.png",
  IronMine: "iron_mine.png",
  MainBuilding: "main_building.png",
  Marketplace: "marketplace.png",
  Palace: "palace.png",
  Palisade: "palisade.png",
  RallyPoint: "rally_point.png",
  Residence: "residence.png",
  Sawmill: "sawmill.png",
  Smithy: "smithy.png",
  Stable: "stable.png",
  StonemansionLodge: "stonemansion_lodge.png",
  TournamentSquare: "tournament_square.png",
  TownHall: "town_hall.png",
  TradeOffice: "trade_office.png",
  Trapper: "trapper.png",
  Treasury: "treasury.png",
  Warehouse: "warehouse.png",
  WonderOfTheWorld: "wonder_of_the_world.png",
  Woodcutter: "woodcutter.png",
  Workshop: "workshop.png",
};

type BuildingSpriteProps = {
  buildingName?: string;
  label?: string;
  className?: string;
  size?: number;
};

export function BuildingSprite({
  buildingName,
  label,
  className,
  size = BUILDING_ICON_SIZE,
}: BuildingSpriteProps) {
  const filename = buildingName ? BUILDING_ICON_BY_NAME[buildingName] : undefined;
  if (!filename) {
    return (
      <span class="inline-flex h-4 w-4 items-center justify-center text-[10px] text-gray-400">
        ?
      </span>
    );
  }

  return (
    <span
      title={label ?? buildingName}
      class={className ?? "inline-block align-middle"}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        backgroundImage: `url(/static/misc/buildings/${filename})`,
        backgroundRepeat: "no-repeat",
        backgroundPosition: "0 0",
        backgroundSize: `${size}px ${size}px`,
        imageRendering: "pixelated",
      }}
    />
  );
}
