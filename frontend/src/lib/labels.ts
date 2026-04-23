const BUILDING_LABELS: Record<string, string> = {
  Woodcutter: "Woodcutter",
  ClayPit: "Clay Pit",
  IronMine: "Iron Mine",
  Cropland: "Cropland",
  Sawmill: "Sawmill",
  Brickyard: "Brickyard",
  IronFoundry: "Iron Foundry",
  GrainMill: "Grain Mill",
  Bakery: "Bakery",
  Warehouse: "Warehouse",
  Granary: "Granary",
  Smithy: "Smithy",
  TournamentSquare: "Tournament Square",
  MainBuilding: "Main Building",
  RallyPoint: "Rally Point",
  Marketplace: "Marketplace",
  Embassy: "Embassy",
  Barracks: "Barracks",
  Stable: "Stable",
  Workshop: "Workshop",
  Academy: "Academy",
  Cranny: "Cranny",
  TownHall: "Town Hall",
  Residence: "Residence",
  Palace: "Palace",
  Treasury: "Treasury",
  TradeOffice: "Trade Office",
  GreatBarracks: "Great Barracks",
  GreatStable: "Great Stable",
  CityWall: "City Wall",
  EarthWall: "Earth Wall",
  Palisade: "Palisade",
  StonemansionLodge: "Stonemason Lodge",
  Brewery: "Brewery",
  Trapper: "Trapper",
  HeroMansion: "Hero Mansion",
  GreatWarehouse: "Great Warehouse",
  GreatGranary: "Great Granary",
  WonderOfTheWorld: "Wonder of the World",
  AncientConstructionPlan: "Ancient Construction Plan",
  HorseDrinkingTrough: "Horse Drinking Trough",
  GreatWorkshop: "Great Workshop",
  EmptySlot: "Empty Slot",
};

const UNIT_LABELS: Record<string, string> = {
  Legionnaire: "Legionnaire",
  Praetorian: "Praetorian",
  Imperian: "Imperian",
  EquitesLegati: "Equites Legati",
  EquitesImperatoris: "Equites Imperatoris",
  EquitesCaesaris: "Equites Caesaris",
  BatteringRam: "Battering Ram",
  FireCatapult: "Fire Catapult",
  Senator: "Senator",
  Settler: "Settler",
  Maceman: "Maceman",
  Spearman: "Spearman",
  Axeman: "Axeman",
  Scout: "Scout",
  Paladin: "Paladin",
  TeutonicKnight: "Teutonic Knight",
  Ram: "Ram",
  Catapult: "Catapult",
  Chief: "Chief",
  Phalanx: "Phalanx",
  Swordsman: "Swordsman",
  Pathfinder: "Pathfinder",
  TheutatesThunder: "Theutates Thunder",
  Druidrider: "Druidrider",
  Haeduan: "Haeduan",
  Trebuchet: "Trebuchet",
  Chieftain: "Chieftain",
  Rat: "Rat",
  Spider: "Spider",
  Serpent: "Serpent",
  Bat: "Bat",
  WildBoar: "Wild Boar",
  Wolf: "Wolf",
  Bear: "Bear",
  Crocodile: "Crocodile",
  Tiger: "Tiger",
  Elephant: "Elephant",
  Pikeman: "Pikeman",
  ThornedWarrior: "Thorned Warrior",
  Guardsman: "Guardsman",
  BirdsOfPrey: "Birds Of Prey",
  Axerider: "Axerider",
  NatarianKnight: "Natarian Knight",
  Warelephant: "Warelephant",
  Ballista: "Ballista",
  NatarianEmperor: "Natarian Emperor",
};

const TRIBE_LABELS: Record<string, string> = {
  Roman: "Roman",
  Gaul: "Gaul",
  Teuton: "Teuton",
  Nature: "Nature",
  Natar: "Natar",
};

function humanizeIdentifier(value: string): string {
  if (!value) return value;
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/_/g, " ")
    .trim();
}

export function buildingLabel(key: string): string {
  return BUILDING_LABELS[key] ?? humanizeIdentifier(key);
}

export function unitLabel(key: string): string {
  return UNIT_LABELS[key] ?? humanizeIdentifier(key);
}

export function tribeLabel(key: string): string {
  return TRIBE_LABELS[key] ?? humanizeIdentifier(key);
}
