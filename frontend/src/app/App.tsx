import { useCallback, useEffect, useMemo, useRef, useState } from "preact/hooks";
import { useQueryClient } from "@tanstack/preact-query";
import { api } from "@/lib/api";
import { isProtectedRoute, navigate, parseRoute } from "@/lib/router";
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
import { useAuthSession } from "@/auth/useAuthSession";
import {
  useBuildingQuery,
  useGameContextQuery,
  useMapFieldQuery,
  usePlayerQuery,
  useReportQuery,
  useReportsQuery,
  useStatsQuery,
} from "@/query/hooks";
import { queryErrorMessage } from "@/query/options";
import { queryKeys } from "@/query/keys";

export function App() {
  const {
    session,
    booting,
    authError,
    setAuthError,
    setTokenSession,
    refreshSession,
    logout,
  } = useAuthSession();
  const queryClient = useQueryClient();
  const [route, setRoute] = useState(() => parseRoute(window.location));
  const queueRefreshInFlightRef = useRef(false);
  const gameContextQuery = useGameContextQuery(session.authenticated && !booting);
  const meContext = gameContextQuery.data ?? null;

  useEffect(() => {
    const onPopState = () => setRoute(parseRoute(window.location));
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  async function invalidateVisibleGameState() {
    const villageId = session.currentVillageId ?? meContext?.currentVillage.id;
    const invalidations = [queryClient.invalidateQueries({ queryKey: queryKeys.gameContext })];
    if (route.name === "building") {
      invalidations.push(queryClient.invalidateQueries({ queryKey: queryKeys.building(route.slotId) }));
    }
    if (route.name === "mapField") {
      invalidations.push(queryClient.invalidateQueries({ queryKey: queryKeys.mapField(route.fieldId) }));
    }
    if (route.name === "reports") {
      invalidations.push(queryClient.invalidateQueries({ queryKey: queryKeys.reports(route.page, route.perPage) }));
    }
    if (route.name === "report") {
      invalidations.push(
        queryClient.invalidateQueries({ queryKey: queryKeys.report(route.reportId) }),
        queryClient.invalidateQueries({ queryKey: ["reports"] }),
      );
    }
    await Promise.all(invalidations);
    if (session.authenticated) {
      await queryClient.fetchQuery({
        queryKey: queryKeys.gameContext,
        queryFn: () => api.gameContext(),
      });
    }
  }

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

  const switchVillage = useCallback(async (villageId: number) => {
    await api.switchVillage({ villageId });
    await refreshSession();
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.gameContext }),
      queryClient.invalidateQueries({ queryKey: ["building"] }),
    ]);
  }, [queryClient, refreshSession]);

  const page = useMemo(() => {
    const activeVillageId = session.currentVillageId ?? meContext?.currentVillage.id;

    const refreshFromQueueElapsed = async () => {
      if (queueRefreshInFlightRef.current) {
        return;
      }
      queueRefreshInFlightRef.current = true;
      try {
        await invalidateVisibleGameState();
      } finally {
        queueRefreshInFlightRef.current = false;
      }
    };

    const runMutation = async () => {
      await invalidateVisibleGameState();
    };

    switch (route.name) {
      case "village":
        if (!activeVillageId || !meContext) return <Loading label="Loading village..." />;
        return (
          <VillagePage
            data={{
              serverTime: meContext.serverTime,
              village: meContext.currentVillage,
              buildingSlots: meContext.buildingSlots,
              buildingQueue: meContext.buildingQueue,
              villages: meContext.villages,
            }}
            onQueueElapsed={refreshFromQueueElapsed}
            onVillageRenamed={runMutation}
            onSwitchVillage={(villageId) => {
              void switchVillage(villageId);
            }}
          />
        );
      case "resources":
        if (!activeVillageId || !meContext) return <Loading label="Loading resources..." />;
        return (
          <ResourcesPage
            data={{
              serverTime: meContext.serverTime,
              village: meContext.currentVillage,
              resourceSlots: meContext.resourceSlots,
              buildingQueue: meContext.buildingQueue,
              currentTroops: meContext.currentTroops,
              troopMovementSummary: meContext.troopMovementSummary,
              villages: meContext.villages,
            }}
            onQueueElapsed={refreshFromQueueElapsed}
            onVillageRenamed={runMutation}
            onSwitchVillage={(villageId) => {
              void switchVillage(villageId);
            }}
          />
        );
      case "building":
        return (
          <ProtectedBuilding
            slotId={route.slotId}
            onMutate={runMutation}
          />
        );
      case "stats":
        return <ProtectedStats page={route.page} currentPlayerId={session.user?.playerId} />;
      case "player":
        return <ProtectedPlayer playerId={route.playerId} />;
      case "reports":
        return <ProtectedReports page={route.page} perPage={route.perPage} />;
      case "report":
        return <ProtectedReport reportId={route.reportId} />;
      case "map":
        return (
          <ProtectedMap
            worldSize={meContext?.worldSize ?? 100}
            centerX={route.x}
            centerY={route.y}
            homeVillageId={meContext?.currentVillage.id}
            homeX={meContext?.currentVillage.x}
            homeY={meContext?.currentVillage.y}
            currentPlayerId={session.authenticated ? session.user?.playerId : undefined}
          />
        );
      case "mapField":
        return (
          <ProtectedMapField
            fieldId={route.fieldId}
            onMutate={runMutation}
            currentPlayerId={session.authenticated ? session.user?.playerId : undefined}
          />
        );
      case "login":
        return (
          <LoginPage
            error={authError}
            onSubmit={async (payload) => {
              setAuthError(null);
              try {
                const next = await api.tokenLogin(payload);
                setTokenSession(next);
                await queryClient.invalidateQueries({ queryKey: queryKeys.gameContext });
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
                setTokenSession(next);
                await queryClient.invalidateQueries({ queryKey: queryKeys.gameContext });
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
  }, [
    route,
    meContext?.worldSize,
    meContext?.currentVillage.id,
    meContext?.currentVillage.x,
    meContext?.currentVillage.y,
    gameContextQuery.dataUpdatedAt,
    session.authenticated,
    session.currentVillageId,
    session.user?.playerId,
    authError,
    switchVillage,
  ]);

  if (booting) {
    return <Loading label="Booting application..." />;
  }

  return (
    <Layout
      session={session}
      meContext={meContext}
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
        await logout();
        navigate("/login", true);
      }}
      onSwitchVillage={switchVillage}
    >
      {page}
    </Layout>
  );
}

function ProtectedStats({ page, currentPlayerId }: { page: number; currentPlayerId?: string }) {
  const query = useStatsQuery(page);
  if (query.isPending) return <Loading label="Loading leaderboard..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load leaderboard.")} />;
  }
  return <StatsPage data={query.data} currentPlayerId={currentPlayerId} />;
}

function ProtectedPlayer({ playerId }: { playerId: string }) {
  const query = usePlayerQuery(playerId);
  if (query.isPending) return <Loading label="Loading player profile..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load player profile.")} />;
  }
  return <PlayerPage data={query.data} />;
}

function ProtectedReports({ page, perPage }: { page: number; perPage: number }) {
  const query = useReportsQuery(page, perPage);
  if (query.isPending) return <Loading label="Loading reports..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load reports.")} />;
  }
  return <ReportsPage data={query.data} />;
}

function ProtectedReport({ reportId }: { reportId: string }) {
  const queryClient = useQueryClient();
  const query = useReportQuery(reportId);
  useEffect(() => {
    if (!query.data) return;
    void Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.gameContext }),
      queryClient.invalidateQueries({ queryKey: ["reports"] }),
    ]);
  }, [query.data, queryClient]);
  if (query.isPending) return <Loading label="Loading report..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load report.")} />;
  }
  return <ReportDetailPage data={query.data} />;
}

function ProtectedMap({
  worldSize,
  centerX,
  centerY,
  homeVillageId,
  homeX,
  homeY,
  currentPlayerId,
}: {
  worldSize: number;
  centerX?: number;
  centerY?: number;
  homeVillageId?: number;
  homeX?: number;
  homeY?: number;
  currentPlayerId?: string;
}) {
  return (
    <MapPage
      worldSize={worldSize}
      initialCenterX={centerX}
      initialCenterY={centerY}
      homeVillageId={homeVillageId}
      homeX={homeX}
      homeY={homeY}
      currentPlayerId={currentPlayerId}
    />
  );
}

function ProtectedMapField({
  fieldId,
  onMutate,
  currentPlayerId,
}: {
  fieldId: number;
  onMutate: () => Promise<void>;
  currentPlayerId?: string;
}) {
  const query = useMapFieldQuery(fieldId);
  if (query.isPending) return <Loading label="Loading field..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load field.")} />;
  }
  return <MapFieldPage data={query.data} onMutate={onMutate} currentPlayerId={currentPlayerId} />;
}

function ProtectedBuilding({
  slotId,
  onMutate,
}: {
  slotId: number;
  onMutate: () => Promise<void>;
}) {
  const query = useBuildingQuery(slotId);
  if (query.isPending) return <Loading label="Loading building..." />;
  if (query.error || !query.data) {
    return <ErrorState message={queryErrorMessage(query.error, "Unable to load building.")} />;
  }
  return <BuildingPage data={query.data} onMutate={onMutate} />;
}

function ErrorState({ message }: { message: string }) {
  return <div class="mx-auto max-w-4xl px-4 py-10 text-sm text-red-700">{message}</div>;
}
