import { QueryClient } from "@tanstack/preact-query";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      gcTime: 5 * 60_000,
      refetchOnReconnect: true,
      refetchOnWindowFocus: false,
    },
  },
});

declare global {
  interface Window {
    __TANSTACK_QUERY_CLIENT__:
    import('@tanstack/query-core').QueryClient
  }
}

window.__TANSTACK_QUERY_CLIENT__ = queryClient;

export { queryClient };
