import { createContext } from "preact";
import { useContext, useState } from "preact/hooks";
import type { ComponentChildren } from "preact";
import type { SessionResponse } from "@/types/api";

const emptySession: SessionResponse = { authenticated: false };

type AppStoreValue = {
  session: SessionResponse;
  setSession: (value: SessionResponse) => void;
  clearAuthState: () => void;
};

const AppStoreContext = createContext<AppStoreValue | undefined>(undefined);

export function AppStoreProvider({ children }: { children: ComponentChildren }) {
  const [session, setSession] = useState<SessionResponse>(emptySession);

  const value: AppStoreValue = {
    session,
    setSession,
    clearAuthState: () => {
      setSession(emptySession);
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
