export const queryKeys = {
  session: ["session"] as const,
  gameContext: ["gameContext"] as const,
  building: (slotId: number) => ["building", slotId] as const,
  stats: (page: number) => ["stats", page] as const,
  player: (playerId: string) => ["player", playerId] as const,
  reports: (page: number, perPage: number) => ["reports", page, perPage] as const,
  report: (reportId: string) => ["report", reportId] as const,
  mapRegion: (params?: { x?: number; y?: number; villageId?: number }) =>
    ["mapRegion", params?.x ?? null, params?.y ?? null, params?.villageId ?? null] as const,
  mapField: (fieldId: number) => ["mapField", fieldId] as const,
};
