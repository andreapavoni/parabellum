import type {
  BuildingPageResponse,
  GameContextResponse,
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
let refreshToken: string | null = null;
let refreshInFlight: Promise<void> | null = null;
const REFRESH_TOKEN_STORAGE_KEY = "parabellum_refresh_token";
const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL ?? "/api/v1").replace(/\/+$/, "");

if (typeof window !== "undefined") {
  refreshToken = window.localStorage.getItem(REFRESH_TOKEN_STORAGE_KEY);
}

function setTokens(tokenResponse: TokenAuthResponse) {
  accessToken = tokenResponse.accessToken;
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
  refreshToken = null;
  if (typeof window !== "undefined") {
    window.localStorage.removeItem(REFRESH_TOKEN_STORAGE_KEY);
  }
}

async function rawRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers: HeadersInit = {
    ...(init.body ? { "Content-Type": "application/json" } : {}),
    ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
    ...(init.headers ?? {}),
  };

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
  try {
    return await rawRequest<T>(path, init);
  } catch (error) {
    if (!(error instanceof ApiError) || !retry) throw error;
    if (error.status !== 401) throw error;
    if (!["token_expired", "unauthorized", "refresh_expired", "session_revoked"].includes(error.code)) {
      throw error;
    }

    await ensureRefreshed();
    return rawRequest<T>(path, init);
  }
}

export const api = {
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
  gameContext: () => request<GameContextResponse>("/game/context"),
  building: (slotId: number) => request<BuildingPageResponse>(`/buildings/${slotId}`),
  switchVillage: (payload: { villageId: number }) =>
    request<{ villageId: number; accessToken?: string; expiresIn?: number }>("/me/village/current", {
      method: "POST",
      body: JSON.stringify(payload),
    }).then((res) => {
      if (res.accessToken) updateAccessToken(res.accessToken);
      return res;
    }),
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
  addBuilding: (payload: { slotId: number; buildingName: string }) =>
    request<{ success: boolean }>("/buildings/add", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  upgradeBuilding: (payload: { slotId: number }) =>
    request<{ success: boolean }>("/buildings/upgrade", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  downgradeBuilding: (payload: { slotId: number }) =>
    request<{ success: boolean }>("/buildings/downgrade", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  renameVillage: (payload: { villageId: number; villageName: string }) =>
    request<{ success: boolean }>("/villages/rename", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  trainUnits: (payload: {
    slotId: number;
    unitIdx: number;
    quantity: number;
    buildingName: string;
  }) =>
    request<{ success: boolean }>("/army/train", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  researchAcademy: (payload: { slotId: number; unitName: string }) =>
    request<{ success: boolean }>("/academy/research", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  researchSmithy: (payload: { slotId: number; unitName: string }) =>
    request<{ success: boolean }>("/smithy/research", {
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
  }) =>
    request<{ success: boolean }>("/marketplace/send", {
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
  }) =>
    request<{ success: boolean }>("/marketplace/offers", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  acceptMarketplaceOffer: (payload: { offerId: string; slotId: number }) =>
    request<{ success: boolean }>(`/marketplace/offers/${payload.offerId}/accept`, {
      method: "POST",
      body: JSON.stringify({ slotId: payload.slotId }),
    }),
  cancelMarketplaceOffer: (payload: { offerId: string; slotId: number }) =>
    request<{ success: boolean }>(`/marketplace/offers/${payload.offerId}/cancel`, {
      method: "POST",
      body: JSON.stringify({ slotId: payload.slotId }),
    }),
  sendTroops: (payload: {
    slotId: number;
    targetX: number;
    targetY: number;
    movement: "attack" | "raid" | "reinforcement";
    units: number[];
    scoutingTarget?: "resources" | "defenses";
    catapultTargets?: string[];
  }) =>
    request<{ success: boolean }>("/army/send", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  previewTroops: (payload: {
    targetX: number;
    targetY: number;
    movement: "attack" | "raid" | "reinforcement";
    units: number[];
  }) =>
    request<MovementPreviewResponse>("/army/preview", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  recallTroops: (payload: { villageId: number; armyId: string; units: number[] }) =>
    request<{ success: boolean }>("/army/recall", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  releaseReinforcements: (payload: {
    villageId: number;
    armyId: string;
    units: number[];
  }) =>
    request<{ success: boolean }>("/army/release", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  foundVillage: (payload: {
    targetX: number;
    targetY: number;
  }) =>
    request<{ success: boolean }>("/map/found-village", {
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
