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

const BUILDING_DESCRIPTIONS: Record<string, string[]> = {
  Woodcutter: [
    "The woodcutter cuts down trees in order to produce lumber. The further you extend the woodcutter, the more lumber is produced.",
    "By constructing a sawmill, you can further increase the production.",
  ],
  ClayPit: [
    "Here, clay is produced. By increasing its level, you increase clay production.",
    "By constructing a brickyard, you can further increase the production.",
  ],
  IronMine: [
    "Here, miners gather the precious resource of iron. By increasing the mine's level, you increase its iron production.",
    "By constructing an iron foundry, you can further increase the production.",
  ],
  Cropland: [
    "Your population's food is produced here. By increasing the cropland's level, you increase crop production.",
    "By constructing a grain mill and a bakery, you can further increase the production.",
  ],
  Sawmill: [
    "Lumber cut by your woodcutters is processed here. The Sawmill boosts wood production in the village.",
    "The bonus from the Sawmill and all buildings providing resource bonuses only apply to the village in which the building is built.",
  ],
  Brickyard: [
    "Clay converts into bricks here. The Brickyard boosts clay production in the village.",
    "The bonus from the Brickyard and all buildings providing resource bonuses only apply to the village in which the building is built.",
  ],
  IronFoundry: [
    "Iron melts here. The Iron Foundry boosts iron production in the village.",
    "The bonus from the Iron Foundry and all buildings providing resource bonuses only apply to the village in which the building is built.",
  ],
  GrainMill: [
    "Grain grinds into flour here. The Grain Mill boosts food production in the village.",
    "Use in conjunction with the Bakery for an overall crop production increase of up to 50%.",
  ],
  Bakery: [
    "Bread bakes from flour here. The Bakery boosts food production in the village.",
    "When used in conjunction with the Grain Mill it can increase crop production by up to 50%.",
  ],
  Warehouse: [
    "The resources lumber, clay and iron are stored in the warehouse. By increasing its level, you increase your warehouse's capacity.",
  ],
  Granary: [
    "The crop produced on your farms is stored in the granary. By increasing its level, you increase the granary's capacity.",
  ],
  Smithy: [
    "The weapons of your warriors are enhanced in the smithy's melting furnaces. By increasing its level, you can order the fabrication of even better weapons and armor.",
  ],
  TournamentSquare: [
    "Your troops can increase their stamina at the Tournament Square. The further the building is upgraded, the faster your troops are beyond a minimum distance of 30 squares.",
  ],
  MainBuilding: [
    "The village's master builders live in the main building. The higher its level, the faster your master builders complete the construction of new buildings.",
  ],
  RallyPoint: [
    "Your village's troops gather here. From here, you can send them out to conquer, raid or reinforce other villages.",
    "If there are less attacking units than the level of the rally point, you can see the type of unit attacking.",
  ],
  Marketplace: [
    "At the marketplace, you can trade resources with other players. The higher its level, the more resources can be transported by your merchants at the same time.",
  ],
  Embassy: [
    "The embassy is a place for diplomats. At level 1 you can join an alliance, and after extending it to level 3 you may even found one yourself.",
  ],
  Barracks: [
    "Infantry can be trained in the barracks. The higher its level, the faster the troops are trained.",
  ],
  Stable: [
    "Cavalry can be trained in the stable. The higher its level, the faster the troops are trained.",
  ],
  Workshop: [
    "Siege engines, like catapults and rams, can be built in the workshop. The higher its level, the faster these units are produced.",
  ],
  Academy: [
    "New unit types can be researched in the academy. By increasing its level, you can order the research of better units.",
  ],
  Cranny: [
    "The cranny hides some of your resources in case the village gets attacked. These resources cannot get stolen.",
    "The capacity of Gallic crannies is larger, while Teutonic attackers reduce how much can be hidden.",
  ],
  TownHall: [
    "In the town hall, you can hold pompous celebrations. Such a celebration increases your culture points.",
    "Culture points are necessary to found or conquer new villages.",
  ],
  Residence: [
    "The Residence protects the village against enemy conquests. You can build one residence per village.",
    "Units that can found a new village or conquer existing villages can be trained here. The residence provides expansion slots at levels 10 and 20.",
  ],
  Palace: [
    "The Palace is unique. You can only build one in your whole realm and you can proclaim that village as your capital.",
    "Units that can found a new village or conquer existing villages can be trained here. The palace provides expansion slots at levels 10, 15 and 20.",
  ],
  Treasury: [
    "The riches of your empire are kept in the treasury. A treasury can only store one artefact at a time.",
  ],
  TradeOffice: [
    "In the trade office, the merchants' carts get improved and equipped with more powerful horses. The higher its level, the more your merchants are able to carry.",
  ],
  GreatBarracks: [
    "The Great Barracks allows you to build a second Barracks in the same village, but the troops cost three times the original amount.",
  ],
  GreatStable: [
    "The Great Stable allows you to build a second Stable in the same village, but the troops cost three times the original amount.",
  ],
  CityWall: [
    "Provides a defense bonus for Roman troops in this village. A higher level wall gives a stronger defense bonus.",
  ],
  EarthWall: [
    "Provides a defense bonus for Teuton troops in this village. A higher level earth wall gives a stronger defense bonus.",
  ],
  Palisade: [
    "Provides a defense bonus for Gaul troops in this village. A higher level palisade gives a stronger defense bonus.",
  ],
  StonemansionLodge: [
    "The Stonemason is an expert at cutting stone. The higher the level, the greater the stability of your village's buildings.",
    "This building can only be built in a capital.",
  ],
  Brewery: [
    "Tasty mead is brewed here. Drinks make Teuton soldiers stronger when attacking, but reduce the persuasive power of leaders.",
    "This building can only be built in the capital.",
  ],
  Trapper: [
    "The trapper protects your village with well-hidden traps. Unwary enemies can be imprisoned and prevented from harming your village.",
  ],
  HeroMansion: [
    "The hero's mansion is the home of your glorious hero.",
    "At building levels 10, 15 and 20, you can use your hero to annex an unoccupied oasis to your village.",
  ],
  GreatWarehouse: [
    "The Great Warehouse has three times the capacity of a normal warehouse.",
  ],
  GreatGranary: [
    "The Great Granary has three times the capacity of a normal granary.",
  ],
  WonderOfTheWorld: [
    "A Wonder of the World is as astonishing as it sounds. Every level costs a lot of resources and requires strong protection.",
    "Finishing a Wonder of the World wins the game world.",
  ],
  AncientConstructionPlan: [
    "Construction plans are required to build and complete a Wonder of the World.",
  ],
  HorseDrinkingTrough: [
    "Decreases the training time and upkeep of Roman cavalry.",
    "Cavalry training time improves by 1% per level and some units consume less crop at higher levels.",
  ],
  GreatWorkshop: [
    "The Great Workshop allows you to build a second Workshop in the same village, but catapults and rams cost three times the original amount.",
  ],
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

export function buildingDescriptionParagraphs(key: string): string[] {
  return BUILDING_DESCRIPTIONS[key] ?? [];
}

export function unitLabel(key: string): string {
  return UNIT_LABELS[key] ?? humanizeIdentifier(key);
}

export function tribeLabel(key: string): string {
  return TRIBE_LABELS[key] ?? humanizeIdentifier(key);
}
