export const queryKeys = {
  session: ["session"] as const,
  gameContext: ["gameContext"] as const,
  currentHero: ["currentHero"] as const,
  gameContextFor: (villageId: number | undefined) => ["gameContext", villageId ?? null] as const,
  buildings: (villageId: number | undefined) => ["building", villageId ?? null] as const,
  building: (villageId: number | undefined, slotId: number) => ["building", villageId ?? null, slotId] as const,
  stats: (page: number) => ["stats", page] as const,
  player: (playerId: string) => ["player", playerId] as const,
  reports: (page: number, perPage: number) => ["reports", page, perPage] as const,
  report: (reportId: string) => ["report", reportId] as const,
  mapRegion: (params?: { x?: number; y?: number; villageId?: number }) =>
    ["mapRegion", params?.x ?? null, params?.y ?? null, params?.villageId ?? null] as const,
  mapField: (fieldId: number) => ["mapField", fieldId] as const,
};
