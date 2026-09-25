import { api } from "@/lib/api";
import { queryKeys } from "@/query/keys";

export function sessionQueryOptions() {
  return {
    queryKey: queryKeys.session,
    queryFn: () => api.tokenSession(),
  };
}

export function gameContextQueryOptions(villageId: number | undefined) {
  return {
    queryKey: queryKeys.gameContextFor(villageId),
    queryFn: ({ signal }: { signal: AbortSignal }) => {
      if (villageId === undefined) throw new Error("No village selected");
      return api.gameContext(villageId, signal);
    },
  };
}

export function queryErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
