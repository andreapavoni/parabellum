import { useState } from "preact/hooks";
import { api } from "@/lib/api";
import type { MapFieldDetailResponse, MovementPreviewResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { Link } from "@/components/Link";

function secondsUntil(timestamp: string) {
  const targetMs = new Date(timestamp).getTime();
  if (Number.isNaN(targetMs)) return 0;
  return Math.max(0, Math.floor((targetMs - Date.now()) / 1000));
}

export function MapFieldPage({
  data,
  onMutate,
}: {
  data: MapFieldDetailResponse;
  onMutate: () => Promise<void>;
}) {
  const [founding, setFounding] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<MovementPreviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const canFoundHere = !data.villageId && data.tileType === "valley";

  return (
    <div class="mx-auto w-full max-w-4xl px-4 py-6">
      <div class="rounded border bg-white p-4 shadow-sm">
        <div class="flex items-center justify-between gap-4">
          <h1 class="text-2xl font-semibold text-gray-800">Field Details</h1>
          <Link to={`/map?x=${data.x}&y=${data.y}`} class="text-sm text-green-700 hover:underline">
            Show on Map
          </Link>
        </div>

        <div class="mt-4 space-y-2 text-sm text-gray-700">
          <div>
            Coordinates: ({data.x}|{data.y})
          </div>
          <div>Type: {data.tileType}</div>
          {data.villageName ? <div>Village: {data.villageName}</div> : null}
          {data.isCapital ? <div>Village status: <CapitalBadge /></div> : null}
          {data.playerName ? <div>Player: {data.playerName}</div> : null}
          {data.villagePopulation ? <div>Population: {data.villagePopulation}</div> : null}
          {data.valley ? (
            <div>
              Valley: {data.valley.lumber}/{data.valley.clay}/{data.valley.iron}/{data.valley.crop}
            </div>
          ) : null}
          <div class="pt-2">
            <Link to={`/app/build/39?target_x=${data.x}&target_y=${data.y}`} class="text-green-700 hover:underline">
              Open rally point for this target
            </Link>
          </div>
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
                    Time remaining: <span class="font-semibold">{secondsUntil(preview.arrivesAt)}s</span>
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
