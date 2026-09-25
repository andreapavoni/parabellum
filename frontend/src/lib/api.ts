import type {
  BuildingPageResponse,
  GameContextResponse,
  Hero,
  MapFieldDetailResponse,
  MovementPreviewResponse,
  SendResourcesPreviewResponse,
  MapRegionResponse,
  PlayerProfileResponse,
  ReportDetailResponse,
  ReportsResponse,
  SessionResponse,
  StatsResponse,
  TokenAuthResponse,
} from "@/types/api";

type RawMapTile = {
  x: number;
  y: number;
  field_id: number;
  village_id?: number;
  player_id?: string;
  village_name?: string;
  village_population?: number;
  is_capital?: boolean;
  player_name?: string;
  tribe?: string;
  tile_type: "village" | "valley" | "oasis";
  valley?: {
    lumber: number;
    clay: number;
    iron: number;
    crop: number;
  };
  oasis?: string;
};

type RawMapRegionResponse = {
  center: {
    x: number;
    y: number;
  };
  radius: number;
  tiles: RawMapTile[];
};

type ApiErrorPayload = {
  code: string;
  message: string;
  fieldErrors?: Record<string, string>;
};

export class ApiError extends Error {
  status: number;
  code: string;
  fieldErrors?: Record<string, string>;

  constructor(status: number, payload: ApiErrorPayload) {
    super(payload.message);
    this.status = status;
    this.code = payload.code;
    this.fieldErrors = payload.fieldErrors;
  }
}

let accessToken: string | null = null;
let currentVillageId: number | undefined;
let villageSwitchQueue: Promise<unknown> = Promise.resolve();
let refreshToken: string | null = null;
let refreshInFlight: Promise<void> | null = null;
const REFRESH_TOKEN_STORAGE_KEY = "parabellum_refresh_token";
const API_BASE_URL = (import.meta.env?.VITE_API_BASE_URL ?? "/api/v1").replace(/\/+$/, "");

if (typeof window !== "undefined") {
  refreshToken = window.localStorage.getItem(REFRESH_TOKEN_STORAGE_KEY);
}

function setTokens(tokenResponse: TokenAuthResponse) {
  accessToken = tokenResponse.accessToken;
  currentVillageId = tokenResponse.currentVillageId;
  refreshToken = tokenResponse.refreshToken;
  if (typeof window !== "undefined") {
    window.localStorage.setItem(REFRESH_TOKEN_STORAGE_KEY, refreshToken);
  }
}

function updateAccessToken(access: string) {
  accessToken = access;
}

function clearTokens() {
  accessToken = null;
  currentVillageId = undefined;
  refreshToken = null;
  if (typeof window !== "undefined") {
    window.localStorage.removeItem(REFRESH_TOKEN_STORAGE_KEY);
  }
}

async function rawRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body) headers.set("Content-Type", "application/json");
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);

  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers,
  });

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as ApiErrorPayload | null;
    throw new ApiError(
      response.status,
      payload ?? {
        code: "unknown_error",
        message: `Request failed with status ${response.status}`,
      },
    );
  }

  return (await response.json()) as T;
}

async function ensureRefreshed() {
  if (refreshInFlight) {
    await refreshInFlight;
    return;
  }

  refreshInFlight = (async () => {
    if (!refreshToken) {
      throw new ApiError(401, { code: "refresh_expired", message: "Refresh token missing" });
    }
    const payload = { refreshToken };
    const refreshed = await rawRequest<TokenAuthResponse>("/auth/refresh", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    setTokens(refreshed);
  })();

  try {
    await refreshInFlight;
  } finally {
    refreshInFlight = null;
  }
}

async function request<T>(path: string, init: RequestInit = {}, retry = true): Promise<T> {
  // Capture the target once; refresh retries must not adopt a newly selected village.
  const headers = new Headers(init.headers);
  if (currentVillageId !== undefined && !path.startsWith("/auth/") &&
      path !== "/me/session" && path !== "/me/village/current" && !headers.has("X-Village-Id")) {
    headers.set("X-Village-Id", String(currentVillageId));
  }
  init = { ...init, headers };
  init.signal?.throwIfAborted();
  try {
    return await rawRequest<T>(path, init);
  } catch (error) {
    if (!(error instanceof ApiError) || !retry) throw error;
    if (error.status !== 401) throw error;
    if (!["token_expired", "unauthorized", "refresh_expired", "session_revoked"].includes(error.code)) {
      throw error;
    }

    await ensureRefreshed();
    init.signal?.throwIfAborted();
    return rawRequest<T>(path, init);
  }
}

function villageHeaders(villageId: number | undefined): HeadersInit {
  return villageId === undefined ? {} : { "X-Village-Id": String(villageId) };
}

export const api = {
  currentVillageId: () => currentVillageId,
  hasAccessToken: () => Boolean(accessToken),
  hasRefreshToken: () => Boolean(refreshToken),
  tokenSession: () => request<SessionResponse>("/me/session", {}, false),
  tokenLogin: (payload: { username: string; password: string }) =>
    request<TokenAuthResponse>("/auth/token/login", {
      method: "POST",
      body: JSON.stringify(payload),
    }, false).then((res) => {
      setTokens(res);
      return res;
    }),
  tokenRegister: (payload: {
    username: string;
    email: string;
    password: string;
    tribe: string;
    quadrant: string;
  }) =>
    request<TokenAuthResponse>("/auth/token/register", {
      method: "POST",
      body: JSON.stringify(payload),
    }, false).then((res) => {
      setTokens(res);
      return res;
    }),
  tokenRefresh: () =>
    refreshToken
      ? request<TokenAuthResponse>(
        "/auth/refresh",
        {
          method: "POST",
          body: JSON.stringify({ refreshToken }),
        },
        false,
      ).then((res) => {
        setTokens(res);
        return res;
      })
      : Promise.reject(new ApiError(401, { code: "refresh_expired", message: "Refresh token missing" })),
  tokenLogout: async () => {
    if (!refreshToken) {
      clearTokens();
      return;
    }
    await request<{ success: boolean }>(
      "/auth/token/logout",
      {
        method: "POST",
        body: JSON.stringify({ refreshToken }),
      },
      false,
    );
    clearTokens();
  },
  gameContext: (villageId: number, signal?: AbortSignal) => request<GameContextResponse>("/game/context", { signal, headers: { "X-Village-Id": String(villageId) } }),
  building: (villageId: number, slotId: number, signal?: AbortSignal) => request<BuildingPageResponse>(`/buildings/${slotId}`, { signal, headers: { "X-Village-Id": String(villageId) } }),
  switchVillage: (payload: { villageId: number }) => {
    const operation = villageSwitchQueue.catch(() => undefined).then(async () => {
      if (refreshInFlight) await refreshInFlight;
      const res = await request<{ villageId: number; accessToken?: string; expiresIn?: number }>("/me/village/current", {
        method: "POST", body: JSON.stringify(payload),
      });
      if (res.accessToken) updateAccessToken(res.accessToken);
      currentVillageId = res.villageId;
      return res;
    });
    villageSwitchQueue = operation;
    return operation;
  },
  stats: (page = 1) => request<StatsResponse>(`/stats?page=${page}`),
  player: (playerId: string) => request<PlayerProfileResponse>(`/players/${playerId}`),
  reports: (page = 1, perPage = 25) =>
    request<ReportsResponse>(`/reports?page=${page}&per_page=${perPage}`),
  report: (reportId: string) => request<ReportDetailResponse>(`/reports/${reportId}`),
  mapRegion: async (params?: { x?: number; y?: number; villageId?: number }) => {
    const search = new URLSearchParams();
    if (params?.x !== undefined) search.set("x", String(params.x));
    if (params?.y !== undefined) search.set("y", String(params.y));
    if (params?.villageId !== undefined) {
      search.set("village_id", String(params.villageId));
    }
    const suffix = search.toString() ? `?${search.toString()}` : "";
    const res = await request<RawMapRegionResponse>(`/map/region${suffix}`);
    return ({
      center: res.center,
      radius: res.radius,
      tiles: res.tiles.map((tile) => ({
        x: tile.x,
        y: tile.y,
        fieldId: tile.field_id,
        villageId: tile.village_id,
        playerId: tile.player_id,
        villageName: tile.village_name,
        villagePopulation: tile.village_population,
        isCapital: tile.is_capital,
        playerName: tile.player_name,
        tribe: tile.tribe,
        tileType: tile.tile_type,
        valley: tile.valley,
        oasis: tile.oasis,
      })),
    });
  },
  mapField: (fieldId: number) => request<MapFieldDetailResponse>(`/map/fields/${fieldId}`),
  addBuilding: (payload: { slotId: number; buildingName: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/buildings/add", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  upgradeBuilding: (payload: { slotId: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/buildings/upgrade", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  downgradeBuilding: (payload: { slotId: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/buildings/downgrade", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  cancelBuildingConstruction: (payload: { actionId: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/buildings/cancel", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  renameVillage: (payload: { villageId: number; villageName: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/villages/rename", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  trainUnits: (payload: {
    slotId: number;
    unitIdx: number;
    quantity: number;
    buildingName: string;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/train", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  currentHero: () => request<Hero>("/hero/current"),
  assignHeroPoints: (payload: {
    heroId: string;
    villageId: number;
    strength: number;
    offBonus: number;
    defBonus: number;
    regeneration: number;
    resources: number;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/hero/points", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  resetHeroPoints: (payload: { heroId: string; villageId: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/hero/points/reset", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  setHeroResourceFocus: (payload: {
    heroId: string;
    villageId: number;
    focus: Hero["resourceFocus"];
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/hero/resource-focus", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  reviveHero: (payload: { heroId: string; villageId: number; reset: boolean }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/hero/revive", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  researchAcademy: (payload: { slotId: number; unitName: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/academy/research", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  researchSmithy: (payload: { slotId: number; unitName: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/smithy/research", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  sendResources: (payload: {
    slotId: number;
    targetX: number;
    targetY: number;
    lumber: number;
    clay: number;
    iron: number;
    crop: number;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/marketplace/send", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  previewSendResources: (payload: {
    slotId: number;
    targetX: number;
    targetY: number;
    lumber: number;
    clay: number;
    iron: number;
    crop: number;
  }) =>
    request<SendResourcesPreviewResponse>("/marketplace/send/preview", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  createMarketplaceOffer: (payload: {
    slotId: number;
    offerLumber: number;
    offerClay: number;
    offerIron: number;
    offerCrop: number;
    seekLumber: number;
    seekClay: number;
    seekIron: number;
    seekCrop: number;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/marketplace/offers", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  acceptMarketplaceOffer: (payload: { offerId: string; slotId: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>(`/marketplace/offers/${payload.offerId}/accept`, {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify({ slotId: payload.slotId }),
    }),
  cancelMarketplaceOffer: (payload: { offerId: string; slotId: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>(`/marketplace/offers/${payload.offerId}/cancel`, {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify({ slotId: payload.slotId }),
    }),
  sendTroops: (payload: {
    slotId: number;
    targetX: number;
    targetY: number;
    movement: "attack" | "raid" | "reinforcement";
    units: number[];
    heroId?: string;
    scoutingTarget?: "resources" | "defenses";
    catapultTargets?: string[];
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/send", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  previewTroops: (payload: {
    targetX: number;
    targetY: number;
    movement: "attack" | "raid" | "reinforcement";
    units: number[];
    heroId?: string;
  }) =>
    request<MovementPreviewResponse>("/army/preview", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  recallTroops: (payload: { villageId: number; armyId: string; units: number[] }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/recall", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  releaseReinforcements: (payload: {
    villageId: number;
    armyId: string;
    units: number[];
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/release", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  releaseTrappedTroops: (payload: {
    villageId: number;
    armyId: string;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/trapped/release", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  disbandTrappedTroops: (payload: {
    villageId: number;
    armyId: string;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/trapped/disband", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  buildTraps: (payload: { villageId: number; quantity: number }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/army/traps/build", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  cancelTroopMovement: (payload: { movementId: string }, villageId = currentVillageId) =>
    request<{ success: boolean }>(`/army/movements/${payload.movementId}`, {
      headers: villageHeaders(villageId),
      method: "DELETE",
    }),
  foundVillage: (payload: {
    targetX: number;
    targetY: number;
  }, villageId = currentVillageId) =>
    request<{ success: boolean }>("/map/found-village", {
      headers: villageHeaders(villageId),
      method: "POST",
      body: JSON.stringify(payload),
    }),
  previewFoundVillage: (payload: {
    targetX: number;
    targetY: number;
  }) =>
    request<MovementPreviewResponse>("/map/found-village/preview", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
};
