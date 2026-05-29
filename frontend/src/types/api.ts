export type ResourceAmounts = {
  lumber: number;
  clay: number;
  iron: number;
  crop: number;
};

export type ProductionPerHour = {
  lumber: number;
  clay: number;
  iron: number;
  crop: number;
};

export type SessionUser = {
  userId: string;
  playerId: string;
  username: string;
  email: string;
  tribe: string;
};

export type SessionResponse = {
  authenticated: boolean;
  user?: SessionUser;
  currentVillageId?: number;
};

export type TokenAuthResponse = {
  accessToken: string;
  expiresIn: number;
  refreshToken: string;
  user: SessionUser;
  currentVillageId: number;
};

export type BuildingType = "empty" | "generic" | "training" | "expansion" | "academy" | "smithy" | "marketplace" | "rally_point";

export type VillageSummary = {
  id: number;
  name: string;
  x: number;
  y: number;
  isCapital: boolean;
  loyalty: number;
  population: number;
  warehouseCapacity: number;
  granaryCapacity: number;
  resources: ResourceAmounts;
  productionPerHour: ProductionPerHour;
};

export type VillageListItem = {
  id: number;
  name: string;
  x: number;
  y: number;
  isCapital: boolean;
  isCurrent: boolean;
};

export type MeContextResponse = {
  serverTime: number;
  worldSize: number;
  serverSpeed: number;
  player: {
    id: string;
    username: string;
    tribe: string;
  };
  currentVillage: VillageSummary;
  villages: VillageListItem[];
};

export type BuildingQueueItem = {
  slotId: number;
  buildingName: string;
  targetLevel: number;
  timeSeconds: number;
  isProcessing: boolean;
};

export type BuildingSlot = {
  slotId: number;
  buildingName?: string;
  level: number;
  inQueue?: boolean;
};

export type ResourceSlot = {
  slotId: number;
  buildingName: string;
  level: number;
  inQueue?: boolean;
};

export type VillageOverviewResponse = {
  serverTime: number;
  village: VillageSummary;
  buildingSlots: BuildingSlot[];
  buildingQueue: BuildingQueueItem[];
};

export type VillageResourcesResponse = {
  serverTime: number;
  village: VillageSummary;
  resourceSlots: ResourceSlot[];
  buildingQueue: BuildingQueueItem[];
  currentTroops: {
    unitName: string;
    count: number;
  }[];
  troopMovementSummary: {
    incomingAttacksRaids: number;
    incomingReturnsReinforcements: number;
    outgoingAttacksRaids: number;
    outgoingReinforcements: number;
  };
};

export type LeaderboardEntry = {
  playerId: string;
  rank: number;
  username: string;
  tribe: string;
  villageCount: number;
  population: number;
};

export type Pagination = {
  page: number;
  perPage: number;
  totalPlayers: number;
  totalPages: number;
};

export type StatsResponse = {
  serverTime: number;
  entries: LeaderboardEntry[];
  pagination: Pagination;
};

export type PlayerVillage = {
  villageId: number;
  name: string;
  x: number;
  y: number;
  isCapital: boolean;
  population: number;
  distanceFromCurrent: number;
};

export type PlayerProfileResponse = {
  serverTime: number;
  playerId: string;
  username: string;
  villages: PlayerVillage[];
};

export type ReportListItem = {
  id: string;
  reportType: string;
  payload: unknown;
  createdAt: number;
  isRead: boolean;
};

export type ReportsResponse = {
  serverTime: number;
  reports: ReportListItem[];
  pagination: {
    page: number;
    perPage: number;
    hasMore: boolean;
  };
};

export type ReportDetailResponse = {
  serverTime: number;
  id: string;
  reportType: string;
  createdAt: number;
  payload: unknown;
};

export type MapPoint = {
  x: number;
  y: number;
};

export type MapTile = {
  x: number;
  y: number;
  fieldId: number;
  villageId?: number;
  playerId?: string;
  villageName?: string;
  villagePopulation?: number;
  isCapital?: boolean;
  playerName?: string;
  tribe?: string;
  tileType: "village" | "valley" | "oasis";
  valley?: {
    lumber: number;
    clay: number;
    iron: number;
    crop: number;
  };
  oasis?: string;
};

export type MapRegionResponse = {
  center: MapPoint;
  radius: number;
  tiles: MapTile[];
};

export type MapFieldDetailResponse = {
  id: number;
  x: number;
  y: number;
  tileType: "village" | "valley" | "oasis";
  villageId?: number;
  playerId?: string;
  villageName?: string;
  playerName?: string;
  villagePopulation?: number;
  isCapital?: boolean;
  valley?: {
    lumber: number;
    clay: number;
    iron: number;
    crop: number;
  };
  oasis?: string;
};

export type MovementPreviewResponse = {
  arrivesAt: string;
  detectedKind: "attack_or_raid" | "scout_only" | "reinforcement" | "found_village";
  supportsScoutingTargetChoice: boolean;
  hasCatapultUnits: boolean;
};

export type Requirement = {
  buildingName: string;
  requiredLevel: number;
};

export type EmptySlotBuildOption = {
  buildingName: string;
  cost: ResourceAmounts;
  timeSecs: number;
  missingRequirements: Requirement[];
};

export type TrainingUnitOption = {
  unitIdx: number;
  name: string;
  cost: ResourceAmounts;
  upkeep: number;
  attack: number;
  defenseInfantry: number;
  defenseCavalry: number;
  speed: number;
  capacity: number;
  timeSecs: number;
};

export type TrainingQueueItem = {
  quantity: number;
  unitName: string;
  timePerUnit: number;
  finishesAt: string;
};

export type AcademyResearchOption = {
  unitName: string;
  cost: ResourceAmounts;
  timeSecs: number;
  missingRequirements: Requirement[];
};

export type AcademyQueueItem = {
  unitName: string;
  finishesAt: string;
  isProcessing: boolean;
};

export type SmithyUpgradeOption = {
  unitName: string;
  currentLevel: number;
  maxLevel: number;
  cost: ResourceAmounts;
  timeSecs: number;
  canUpgrade: boolean;
};

export type SmithyQueueItem = {
  unitName: string;
  targetLevel: number;
  finishesAt: string;
  isProcessing: boolean;
};

export type BuildingDetail = {
  slotId: number;
  villageId: number;
  buildingName: string;
  buildingType: BuildingType;
  currentLevel: number;
  population: number;
  currentUpkeep: number;
  nextLevel: number;
  nextUpkeep: number;
  timeSecs: number;
  queueFull: boolean;
  atMaxLevel: boolean;
  nextValue?: string;
  cost: ResourceAmounts;
  storedResources: ResourceAmounts;
  emptySlot?: {
    buildableBuildings: EmptySlotBuildOption[];
    lockedBuildings: EmptySlotBuildOption[];
    hasQueueForSlot: boolean;
    queuedBuildingName?: string;
    queuedTargetLevel?: number;
    queuedNextLevel?: number;
    queuedCanUpgrade?: boolean;
    queuedUpgradePreview?: {
      buildingName: string;
      currentLevel: number;
      nextLevel: number;
      currentUpkeep: number;
      nextUpkeep: number;
      timeSecs: number;
      atMaxLevel: boolean;
      nextValue?: string;
      cost: ResourceAmounts;
    };
  };
  training?: {
    trainingSpeedPercent: number;
    units: TrainingUnitOption[];
    queue: TrainingQueueItem[];
  };
  expansion?: {
    loyalty: number;
    villageCulturePointsProduction: number;
    accountCulturePointsProduction: number;
    accountCulturePoints: number;
    nextCpRequired: number;
    maxFoundationSlots: number;
    childVillagesCount: number;
    settlersAtHome: number;
    settlersDeployed: number;
    maxSettlersTrainable: number;
  };
  academy?: {
    readyUnits: AcademyResearchOption[];
    lockedUnits: AcademyResearchOption[];
    researchedUnits: string[];
    queue: AcademyQueueItem[];
    queueFull: boolean;
  };
  smithy?: {
    units: SmithyUpgradeOption[];
    queue: SmithyQueueItem[];
    queueFull: boolean;
  };
  marketplace?: {
    availableMerchants: number;
    totalMerchants: number;
    ownOffers: MarketplaceOffer[];
    globalOffers: MarketplaceOffer[];
    merchantMovements: MerchantMovement[];
  };
  rallyPoint?: {
    cards: RallyCard[];
    sendableUnits: RallySendableUnit[];
  };
  descriptionParagraphs: string[];
};

export type BuildingPageResponse = {
  serverTime: number;
  village: VillageSummary;
  villages: VillageListItem[];
  detail: BuildingDetail;
};

export type Position = {
  x: number;
  y: number;
};

export type MarketplaceOffer = {
  offerId: string;
  villageId: number;
  villageName: string;
  position: Position;
  offerResources: ResourceAmounts;
  seekResources: ResourceAmounts;
  merchantsRequired: number;
  createdAt: number;
};

export type MerchantMovementDirection = "outgoing" | "incoming";
export type MerchantMovementKind = "going" | "return";

export type MerchantMovement = {
  jobId: string;
  direction: MerchantMovementDirection;
  kind: MerchantMovementKind;
  originName: string;
  originPosition?: Position;
  destinationName: string;
  destinationPosition?: Position;
  resources: ResourceAmounts;
  merchantsUsed: number;
  arrivesAt: string;
};

export type RallyCardCategory = "stationed" | "reinforcement" | "deployed" | "incoming" | "outgoing";
export type RallyMovementKind = "attack" | "raid" | "scout" | "reinforcement" | "return" | "found_village";
export type RallyAction = "recall" | "release";

export type RallyCard = {
  villageId: number;
  villageName?: string;
  position?: Position;
  tribe: string;
  units: number[];
  upkeep: number;
  category: RallyCardCategory;
  movementKind?: RallyMovementKind;
  arrivesAt?: string;
  bounty?: ResourceAmounts;
  action?: RallyAction;
  actionId?: string;
};

export type RallySendableUnit = {
  unitIdx: number;
  name: string;
  available: number;
  isResearched: boolean;
};
