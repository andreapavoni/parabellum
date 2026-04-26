import { createContext } from "preact";
import { useContext, useState } from "preact/hooks";
import type { ComponentChildren } from "preact";
import type { MeContextResponse, SessionResponse, VillageSummary } from "@/types/api";

const emptySession: SessionResponse = { authenticated: false };

type AppStoreValue = {
  session: SessionResponse;
  setSession: (value: SessionResponse) => void;
  meContext: MeContextResponse | null;
  setMeContext: (value: MeContextResponse | null) => void;
  updateCurrentVillage: (village: VillageSummary) => void;
  clearAuthState: () => void;
};

const AppStoreContext = createContext<AppStoreValue | undefined>(undefined);

export function AppStoreProvider({ children }: { children: ComponentChildren }) {
  const [session, setSession] = useState<SessionResponse>(emptySession);
  const [meContext, setMeContext] = useState<MeContextResponse | null>(null);

  const value: AppStoreValue = {
    session,
    setSession,
    meContext,
    setMeContext,
    updateCurrentVillage: (village) => {
      setMeContext((current) => (current ? { ...current, currentVillage: village } : current));
    },
    clearAuthState: () => {
      setSession(emptySession);
      setMeContext(null);
    },
  };

  return <AppStoreContext.Provider value={value}>{children}</AppStoreContext.Provider>;
}

export function useAppStore() {
  const value = useContext(AppStoreContext);
  if (!value) {
    throw new Error("useAppStore must be used within AppStoreProvider");
  }
  return value;
}

