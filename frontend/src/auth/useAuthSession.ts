import { useEffect, useState } from "preact/hooks";
import { useQueryClient } from "@tanstack/preact-query";
import { api } from "@/lib/api";
import type { SessionResponse, TokenAuthResponse } from "@/types/api";
import { useAppStore } from "@/state/appStore";
import { queryKeys } from "@/query/keys";

const emptySession: SessionResponse = {
  authenticated: false,
};

export function useAuthSession() {
  const { session, setSession, clearAuthState } = useAppStore();
  const queryClient = useQueryClient();
  const [booting, setBooting] = useState(true);
  const [authError, setAuthError] = useState<string | null>(null);

  function setSessionData(value: SessionResponse) {
    setSession(value);
    queryClient.setQueryData(queryKeys.session, value);
  }

  function setTokenSession(token: TokenAuthResponse) {
    setSessionData({
      authenticated: true,
      user: token.user,
      currentVillageId: token.currentVillageId,
    });
  }

  async function refreshSession() {
    if (!api.hasAccessToken() && api.hasRefreshToken()) {
      try {
        const next = await api.tokenRefresh();
        setTokenSession(next);
        return {
          authenticated: true,
          user: next.user,
          currentVillageId: next.currentVillageId,
        } satisfies SessionResponse;
      } catch {
        setSessionData(emptySession);
        return emptySession;
      }
    }

    try {
      const current = await api.tokenSession();
      setSessionData(current);
      return current;
    } catch {
      try {
        const next = await api.tokenRefresh();
        setTokenSession(next);
        return {
          authenticated: true,
          user: next.user,
          currentVillageId: next.currentVillageId,
        } satisfies SessionResponse;
      } catch {
        setSessionData(emptySession);
        return emptySession;
      }
    }
  }

  async function logout() {
    await api.tokenLogout();
    clearAuthState();
    queryClient.clear();
  }

  useEffect(() => {
    let alive = true;
    setBooting(true);
    refreshSession()
      .catch((error: Error) => {
        if (alive) setAuthError(error.message);
      })
      .finally(() => {
        if (alive) setBooting(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  return {
    session,
    booting,
    authError,
    setAuthError,
    setTokenSession,
    refreshSession,
    logout,
  };
}
