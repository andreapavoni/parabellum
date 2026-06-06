import { api } from "@/lib/api";
import { queryKeys } from "@/query/keys";

export function sessionQueryOptions() {
  return {
    queryKey: queryKeys.session,
    queryFn: () => api.tokenSession(),
  };
}

export function gameContextQueryOptions() {
  return {
    queryKey: queryKeys.gameContext,
    queryFn: () => api.gameContext(),
  };
}

export function queryErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
