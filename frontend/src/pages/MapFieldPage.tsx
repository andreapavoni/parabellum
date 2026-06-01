import { useState } from "preact/hooks";
import { api } from "@/lib/api";
import type { MapFieldDetailResponse, MovementPreviewResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { Link } from "@/components/Link";
import { VillageRenameInline } from "@/components/VillageRenameInline";
import { ResourceSprite } from "@/components/ResourceSprite";
import { secondsUntilIso } from "@/lib/time";

export function MapFieldPage({
  data,
  onMutate,
  currentPlayerId,
}: {
  data: MapFieldDetailResponse;
  onMutate: () => Promise<void>;
  currentPlayerId?: string;
}) {
  const [founding, setFounding] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<MovementPreviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const canFoundHere = !data.villageId && data.tileType === "valley" && data.canPreviewFounding;
  const canRenameVillage =
    data.tileType === "village" &&
    !!data.villageId &&
    !!data.playerId &&
    !!currentPlayerId &&
    data.playerId === currentPlayerId;

  return (
    <div class="mx-auto w-full max-w-4xl px-4 py-6">
      <div class="rounded border bg-white p-4 shadow-sm">
        <div class="flex items-center justify-between gap-4">
          <h1 class="text-2xl font-semibold text-gray-800">
            {data.tileType === "valley"
              ? "Unoccupied valley"
              : data.tileType === "village"
              ? data.villageName ?? "Village"
              : "Oasis"}
          </h1>
          <Link to={`/map?x=${data.x}&y=${data.y}`} class="text-sm text-green-700 hover:underline">
            Show on Map
          </Link>
        </div>

        <div class="mt-4 space-y-2 text-sm text-gray-700">
          <div>
            Coordinates: ({data.x}|{data.y})
          </div>
          <div>Type: {data.tileType}</div>
          {data.tileType === "village" ? (
            <>
              {canRenameVillage ? (
                <div class="pt-1">
                  <VillageRenameInline
                    villageId={data.villageId!}
                    currentName={data.villageName ?? ""}
                    onRenamed={onMutate}
                    className="w-full"
                    linkClassName="p-0 text-sm text-green-700 underline hover:text-green-800 bg-transparent border-0"
                  />
                </div>
              ) : null}
              {data.playerId && data.playerName ? (
                <div>
                  Owner:{" "}
                  <Link to={`/players/${data.playerId}`} class="text-green-700 hover:underline">
                    {data.playerName}
                  </Link>
                </div>
              ) : null}
              {data.villagePopulation ? <div>Population: {data.villagePopulation}</div> : null}
              {data.isCapital ? (
                <div>
                  Village status: <CapitalBadge />
                </div>
              ) : null}
              {data.valley ? (
                <div>
                  Topology: {data.valley.lumber}/{data.valley.clay}/{data.valley.iron}/{data.valley.crop}
                </div>
              ) : null}
            </>
          ) : null}
          {data.tileType === "valley" && data.valley ? (
            <div>
              Topology: {data.valley.lumber}/{data.valley.clay}/{data.valley.iron}/{data.valley.crop}
            </div>
          ) : null}
          {data.tileType === "oasis" ? (
            <>
              {data.oasis ? <div>Oasis type: {data.oasis}</div> : null}
              {data.oasisBonus ? (
                <div class="inline-flex flex-wrap items-center gap-3">
                  <span>Bonus:</span>
                  {data.oasisBonus.lumber > 0 ? (
                    <span class="inline-flex items-center gap-1">
                      <ResourceSprite kind="lumber" size={16} label="Wood" />
                      +{data.oasisBonus.lumber}%
                    </span>
                  ) : null}
                  {data.oasisBonus.clay > 0 ? (
                    <span class="inline-flex items-center gap-1">
                      <ResourceSprite kind="clay" size={16} label="Clay" />
                      +{data.oasisBonus.clay}%
                    </span>
                  ) : null}
                  {data.oasisBonus.iron > 0 ? (
                    <span class="inline-flex items-center gap-1">
                      <ResourceSprite kind="iron" size={16} label="Iron" />
                      +{data.oasisBonus.iron}%
                    </span>
                  ) : null}
                  {data.oasisBonus.crop > 0 ? (
                    <span class="inline-flex items-center gap-1">
                      <ResourceSprite kind="crop" size={16} label="Crop" />
                      +{data.oasisBonus.crop}%
                    </span>
                  ) : null}
                </div>
              ) : null}
            </>
          ) : null}

          {data.tileType === "village" ? (
            <div class="pt-2 space-y-1">
              {data.hasMarketplace && data.marketplaceSlotId ? (
                <div>
                  <Link
                    to={`/app/build/${data.marketplaceSlotId}?target_x=${data.x}&target_y=${data.y}`}
                    class="text-green-700 hover:underline"
                  >
                    Send resources from marketplace
                  </Link>
                </div>
              ) : (
                <div class="text-gray-500">Build a Marketplace to send resources.</div>
              )}
              {data.hasRallyPoint && data.rallyPointSlotId ? (
                <div>
                  <Link
                    to={`/app/build/${data.rallyPointSlotId}?target_x=${data.x}&target_y=${data.y}`}
                    class="text-green-700 hover:underline"
                  >
                    Open rally point for this target
                  </Link>
                </div>
              ) : (
                <div class="text-gray-500">Build a Rally Point to send troops.</div>
              )}
            </div>
          ) : null}
          {data.tileType === "oasis" ? (
            <div class="pt-2 space-y-1">
              {data.hasRallyPoint && data.rallyPointSlotId ? (
                <div>
                  <Link
                    to={`/app/build/${data.rallyPointSlotId}?target_x=${data.x}&target_y=${data.y}`}
                    class="text-green-700 hover:underline"
                  >
                    Open rally point for this target
                  </Link>
                </div>
              ) : (
                <div class="text-gray-500">Build a Rally Point to send troops.</div>
              )}
            </div>
          ) : null}

          {canFoundHere ? (
            <div class="pt-1 space-y-2">
              <button
                type="button"
                class="rounded bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-60"
                disabled={founding || previewing}
                onClick={async () => {
                  setError(null);
                  setPreview(null);
                  try {
                    setPreviewing(true);
                    const result = await api.previewFoundVillage({
                      targetX: data.x,
                      targetY: data.y,
                    });
                    setPreview(result);
                  } catch (err) {
                    setError((err as Error).message);
                  } finally {
                    setPreviewing(false);
                  }
                }}
              >
                {previewing ? "Calculating..." : "Preview founding"}
              </button>
              {preview ? (
                <div class="rounded border border-emerald-200 bg-emerald-50 p-3 space-y-2 text-sm">
                  <div>
                    Arrives at: <span class="font-semibold">{new Date(preview.arrivesAt).toLocaleString()}</span>
                  </div>
                  <div>
                    Time remaining: <span class="font-semibold">{secondsUntilIso(preview.arrivesAt)}s</span>
                  </div>
                  <div>
                    Detected movement: <span class="font-semibold">Found village</span>
                  </div>
                  <button
                    type="button"
                    class="rounded bg-emerald-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-emerald-700 disabled:opacity-60"
                    disabled={founding}
                    onClick={async () => {
                      setFounding(true);
                      setError(null);
                      try {
                        await api.foundVillage({
                          targetX: data.x,
                          targetY: data.y,
                        });
                        await onMutate();
                        window.location.assign(`/app/build/39?target_x=${data.x}&target_y=${data.y}`);
                      } catch (err) {
                        setError((err as Error).message);
                      } finally {
                        setFounding(false);
                      }
                    }}
                  >
                    {founding ? "Founding..." : "Confirm and found village"}
                  </button>
                </div>
              ) : null}
              {error ? <p class="text-xs text-red-600">{error}</p> : null}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
