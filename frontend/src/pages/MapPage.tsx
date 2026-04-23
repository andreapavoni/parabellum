import { useEffect, useMemo, useState } from "preact/hooks";
import { api } from "@/lib/api";
import type { MapFieldDetailResponse, MapRegionResponse } from "@/types/api";
import { Link } from "@/components/Link";

function wrapCoordinate(value: number, worldSize: number) {
  const span = worldSize * 2 + 1;
  const normalized = ((value + worldSize) % span + span) % span;
  return normalized - worldSize;
}

export function MapPage({
  worldSize,
  initialFieldId,
}: {
  worldSize: number;
  initialFieldId?: number;
}) {
  const [region, setRegion] = useState<MapRegionResponse | null>(null);
  const [detail, setDetail] = useState<MapFieldDetailResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    api
      .mapRegion()
      .then((result) => {
        if (!alive) return;
        setRegion(result);
        if (initialFieldId) {
          return api.mapField(initialFieldId).then((field) => alive && setDetail(field));
        }
      })
      .catch((err: Error) => alive && setError(err.message))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [initialFieldId]);

  const rows = useMemo(() => {
    if (!region) return [];
    const gridSize = region.radius * 2 + 1;
    const lookup = new Map(region.tiles.map((tile) => [`${tile.x}:${tile.y}`, tile]));
    const result = [];
    for (let row = 0; row < gridSize; row += 1) {
      const y = region.center.y + region.radius - row;
      const currentRow = [];
      for (let col = 0; col < gridSize; col += 1) {
        const x = region.center.x - region.radius + col;
        const wrappedX = wrapCoordinate(x, worldSize);
        const wrappedY = wrapCoordinate(y, worldSize);
        currentRow.push(lookup.get(`${wrappedX}:${wrappedY}`) ?? null);
      }
      result.push(currentRow);
    }
    return result;
  }, [region, worldSize]);

  if (loading) {
    return <div class="mx-auto max-w-5xl px-4 py-6 text-sm text-gray-500">Loading map...</div>;
  }

  if (error || !region) {
    return <div class="mx-auto max-w-5xl px-4 py-6 text-sm text-red-700">{error ?? "Unable to load the map."}</div>;
  }

  return (
    <div class="mx-auto max-w-6xl px-4 py-6">
      <div class="grid gap-6 lg:grid-cols-[1fr_320px]">
        <div class="overflow-auto rounded border bg-white p-4 shadow-sm">
          <div class="mb-3 text-sm text-gray-600">
            Center: ({region.center.x}|{region.center.y})
          </div>
          <div class="grid gap-1" style={{ gridTemplateColumns: `repeat(${region.radius * 2 + 1}, minmax(0, 1fr))` }}>
            {rows.flat().map((tile, index) =>
              tile ? (
                <button
                  key={tile.fieldId}
                  class={`aspect-square rounded border p-1 text-[10px] text-left ${
                    tile.tileType === "village"
                      ? "bg-emerald-50 border-emerald-200"
                      : tile.tileType === "oasis"
                        ? "bg-amber-50 border-amber-200"
                        : "bg-stone-50 border-stone-200"
                  }`}
                  onClick={async () => {
                    setDetail(await api.mapField(tile.fieldId));
                  }}
                >
                  <div class="font-semibold">
                    {tile.x}|{tile.y}
                  </div>
                  <div>{tile.villageName ?? tile.oasis ?? tile.tileType}</div>
                </button>
              ) : (
                <div key={`empty-${index}`} class="aspect-square rounded border border-stone-100 bg-stone-50" />
              ),
            )}
          </div>
        </div>
        <div class="rounded border bg-white p-4 shadow-sm">
          <h2 class="text-lg font-semibold text-gray-800">Field details</h2>
          {detail ? (
            <div class="mt-4 space-y-2 text-sm text-gray-700">
              <div>
                Coordinates: ({detail.x}|{detail.y})
              </div>
              <div>Type: {detail.tileType}</div>
              {detail.villageName ? <div>Village: {detail.villageName}</div> : null}
              {detail.playerName ? <div>Player: {detail.playerName}</div> : null}
              {detail.villagePopulation ? <div>Population: {detail.villagePopulation}</div> : null}
              {detail.valley ? (
                <div>
                  Valley: {detail.valley.lumber}/{detail.valley.clay}/{detail.valley.iron}/{detail.valley.crop}
                </div>
              ) : null}
              <Link to={`/app/build/39?target_x=${detail.x}&target_y=${detail.y}`} class="inline-block text-green-700 hover:underline">
                Open rally point for this target
              </Link>
              {!detail.villageId && detail.tileType === "valley" ? (
                <p class="text-xs text-gray-500">Use movement type "Found village" with settlers.</p>
              ) : null}
            </div>
          ) : (
            <div class="mt-4 text-sm text-gray-500">Select a field to inspect it.</div>
          )}
        </div>
      </div>
    </div>
  );
}
