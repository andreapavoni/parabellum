import { useEffect, useMemo, useState } from "preact/hooks";
import { api } from "@/lib/api";
import { isProtectedRoute, navigate, parseRoute } from "@/lib/router";
import type { BootstrapResponse, SessionResponse } from "@/types/api";
import { Layout } from "@/components/Layout";
import { Loading } from "@/components/Loading";
import { HomePage } from "@/pages/HomePage";
import { LoginPage } from "@/pages/LoginPage";
import { RegisterPage } from "@/pages/RegisterPage";
import { VillagePage } from "@/pages/VillagePage";
import { ResourcesPage } from "@/pages/ResourcesPage";
import { StatsPage } from "@/pages/StatsPage";
import { PlayerPage } from "@/pages/PlayerPage";
import { ReportDetailPage, ReportsPage } from "@/pages/ReportsPage";
import { MapPage } from "@/pages/MapPage";
import { MapFieldPage } from "@/pages/MapFieldPage";
import { BuildingPage } from "@/pages/BuildingPage";
import { usePageData } from "@/hooks/usePageData";

const emptySession: SessionResponse = {
  authenticated: false,
};

export function App() {
  const [route, setRoute] = useState(() => parseRoute(window.location));
  const [session, setSession] = useState<SessionResponse>(emptySession);
  const [bootstrap, setBootstrap] = useState<BootstrapResponse | null>(null);
  const [booting, setBooting] = useState(true);
  const [authError, setAuthError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    const onPopState = () => setRoute(parseRoute(window.location));
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  useEffect(() => {
    setRoute(parseRoute(window.location));
  }, [reloadKey]);

  async function refreshSession() {
    try {
      const current = await api.tokenSession();
      setSession(current);
      return current;
    } catch {
      try {
        await api.tokenRefresh();
        const current = await api.tokenSession();
        setSession(current);
        return current;
      } catch {
        setSession(emptySession);
        return emptySession;
      }
    }
  }

  async function refreshBootstrap() {
    const data = await api.bootstrap();
    setBootstrap(data);
    return data;
  }

  useEffect(() => {
    let alive = true;
    setBooting(true);
    refreshSession()
      .then(async (current) => {
        if (!alive) return;
        if (current.authenticated) {
          await refreshBootstrap();
        } else {
          setBootstrap(null);
        }
      })
      .catch((error: Error) => {
        if (alive) setAuthError(error.message);
      })
      .finally(() => {
        if (alive) setBooting(false);
      });
    return () => {
      alive = false;
    };
  }, [reloadKey]);

  useEffect(() => {
    if (booting) return;
    if (isProtectedRoute(route) && !session.authenticated) {
      navigate("/login", true);
      return;
    }
    if (
      session.authenticated &&
      (route.name === "login" || route.name === "register" || route.name === "home")
    ) {
      navigate("/village", true);
    }
  }, [booting, route, session.authenticated]);

  const page = useMemo(() => {
    const runMutation = async () => {
      await Promise.all([refreshSession(), refreshBootstrap()]);
      setReloadKey((value) => value + 1);
    };

    switch (route.name) {
      case "village":
        return <ProtectedVillage reloadKey={reloadKey} />;
      case "resources":
        return <ProtectedResources reloadKey={reloadKey} />;
      case "building":
        return (
          <ProtectedBuilding
            slotId={route.slotId}
            reloadKey={reloadKey}
            onMutate={runMutation}
          />
        );
      case "stats":
        return <ProtectedStats page={route.page} reloadKey={reloadKey} />;
      case "player":
        return <ProtectedPlayer playerId={route.playerId} reloadKey={reloadKey} />;
      case "reports":
        return <ProtectedReports reloadKey={reloadKey} />;
      case "report":
        return <ProtectedReport reportId={route.reportId} reloadKey={reloadKey} />;
      case "map":
        return (
          <ProtectedMap
            worldSize={bootstrap?.worldSize ?? 100}
            centerX={route.x}
            centerY={route.y}
            homeVillageId={bootstrap?.village.id}
            homeX={bootstrap?.village.x}
            homeY={bootstrap?.village.y}
          />
        );
      case "mapField":
        return <ProtectedMapField fieldId={route.fieldId} reloadKey={reloadKey} />;
      case "login":
        return (
          <LoginPage
            error={authError}
            onSubmit={async (payload) => {
              setAuthError(null);
              try {
                const next = await api.tokenLogin(payload);
                setSession({
                  authenticated: true,
                  user: next.user,
                  currentVillageId: next.currentVillageId,
                });
                await refreshBootstrap();
                navigate("/village", true);
              } catch (error) {
                setAuthError((error as Error).message);
                throw error;
              }
            }}
          />
        );
      case "register":
        return (
          <RegisterPage
            error={authError}
            onSubmit={async (payload) => {
              setAuthError(null);
              try {
                const next = await api.tokenRegister(payload);
                setSession({
                  authenticated: true,
                  user: next.user,
                  currentVillageId: next.currentVillageId,
                });
                await refreshBootstrap();
                navigate("/village", true);
              } catch (error) {
                setAuthError((error as Error).message);
                throw error;
              }
            }}
          />
        );
      case "home":
        return <HomePage />;
      default:
        return <div class="mx-auto max-w-4xl px-4 py-10 text-sm text-gray-500">Page not found.</div>;
    }
  }, [route, bootstrap?.worldSize, authError, reloadKey]);

  if (booting) {
    return <Loading label="Booting application..." />;
  }

  return (
    <Layout
      session={session}
      bootstrap={bootstrap}
      active={
        route.name === "report"
          ? "reports"
          : route.name === "mapField"
            ? "map"
          : route.name === "player"
            ? "stats"
            : route.name
      }
      onLogout={async () => {
        await api.tokenLogout();
        setSession(emptySession);
        setBootstrap(null);
        setReloadKey((value) => value + 1);
        navigate("/login", true);
      }}
      onSwitchVillage={async (villageId) => {
        await api.switchVillage({ villageId });
        await Promise.all([refreshSession(), refreshBootstrap()]);
        setReloadKey((value) => value + 1);
      }}
    >
      {page}
    </Layout>
  );
}

function ProtectedVillage({ reloadKey }: { reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.village(), [reloadKey]);
  if (loading) return <Loading label="Loading village..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load village."} />;
  return <VillagePage data={data} />;
}

function ProtectedResources({ reloadKey }: { reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.resources(), [reloadKey]);
  if (loading) return <Loading label="Loading resources..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load resources."} />;
  return <ResourcesPage data={data} />;
}

function ProtectedStats({ page, reloadKey }: { page: number; reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.stats(page), [page, reloadKey]);
  if (loading) return <Loading label="Loading leaderboard..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load leaderboard."} />;
  return <StatsPage data={data} />;
}

function ProtectedPlayer({ playerId, reloadKey }: { playerId: string; reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.player(playerId), [playerId, reloadKey]);
  if (loading) return <Loading label="Loading player profile..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load player profile."} />;
  return <PlayerPage data={data} />;
}

function ProtectedReports({ reloadKey }: { reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.reports(), [reloadKey]);
  if (loading) return <Loading label="Loading reports..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load reports."} />;
  return <ReportsPage data={data} />;
}

function ProtectedReport({ reportId, reloadKey }: { reportId: string; reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.report(reportId), [reportId, reloadKey]);
  if (loading) return <Loading label="Loading report..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load report."} />;
  return <ReportDetailPage data={data} />;
}

function ProtectedMap({
  worldSize,
  centerX,
  centerY,
  homeVillageId,
  homeX,
  homeY,
}: {
  worldSize: number;
  centerX?: number;
  centerY?: number;
  homeVillageId?: number;
  homeX?: number;
  homeY?: number;
}) {
  return (
    <MapPage
      worldSize={worldSize}
      initialCenterX={centerX}
      initialCenterY={centerY}
      homeVillageId={homeVillageId}
      homeX={homeX}
      homeY={homeY}
    />
  );
}

function ProtectedMapField({ fieldId, reloadKey }: { fieldId: number; reloadKey: number }) {
  const { data, error, loading } = usePageData(() => api.mapField(fieldId), [fieldId, reloadKey]);
  if (loading) return <Loading label="Loading field..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load field."} />;
  return <MapFieldPage data={data} />;
}

function ProtectedBuilding({
  slotId,
  reloadKey,
  onMutate,
}: {
  slotId: number;
  reloadKey: number;
  onMutate: () => Promise<void>;
}) {
  const { data, error, loading } = usePageData(() => api.building(slotId), [slotId, reloadKey]);
  if (loading) return <Loading label="Loading building..." />;
  if (error || !data) return <ErrorState message={error ?? "Unable to load building."} />;
  return <BuildingPage data={data} onMutate={onMutate} />;
}

function ErrorState({ message }: { message: string }) {
  return <div class="mx-auto max-w-4xl px-4 py-10 text-sm text-red-700">{message}</div>;
}
