import { useQuery } from "@tanstack/preact-query";
import { api } from "@/lib/api";
import { gameContextQueryOptions, sessionQueryOptions } from "@/query/options";
import { queryKeys } from "@/query/keys";

export function useSessionQuery(enabled = true) {
  return useQuery({
    ...sessionQueryOptions(),
    enabled,
  });
}

export function useGameContextQuery(enabled = true) {
  return useQuery({
    ...gameContextQueryOptions(),
    enabled,
    staleTime: 30_000,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
  });
}

export function useCurrentHeroQuery(enabled = true) {
  return useQuery({
    queryKey: queryKeys.currentHero,
    queryFn: api.currentHero,
    enabled,
  });
}

export function useBuildingQuery(slotId: number) {
  return useQuery({
    queryKey: queryKeys.building(slotId),
    queryFn: () => api.building(slotId),
  });
}

export function useStatsQuery(page: number) {
  return useQuery({
    queryKey: queryKeys.stats(page),
    queryFn: () => api.stats(page),
  });
}

export function usePlayerQuery(playerId: string) {
  return useQuery({
    queryKey: queryKeys.player(playerId),
    queryFn: () => api.player(playerId),
  });
}

export function useReportsQuery(page: number, perPage: number) {
  return useQuery({
    queryKey: queryKeys.reports(page, perPage),
    queryFn: () => api.reports(page, perPage),
  });
}

export function useReportQuery(reportId: string) {
  return useQuery({
    queryKey: queryKeys.report(reportId),
    queryFn: () => api.report(reportId),
  });
}

export function useMapRegionQuery(params?: { x?: number; y?: number; villageId?: number }) {
  return useQuery({
    queryKey: queryKeys.mapRegion(params),
    queryFn: () => api.mapRegion(params),
    placeholderData: (previousData) => previousData,
  });
}

export function useMapFieldQuery(fieldId: number) {
  return useQuery({
    queryKey: queryKeys.mapField(fieldId),
    queryFn: () => api.mapField(fieldId),
  });
}
