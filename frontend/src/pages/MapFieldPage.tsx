import type { MapFieldDetailResponse } from "@/types/api";
import { Link } from "@/components/Link";

export function MapFieldPage({ data }: { data: MapFieldDetailResponse }) {
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
          {!data.villageId && data.tileType === "valley" ? (
            <p class="text-xs text-gray-500">Use movement type "Found village" with settlers.</p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
