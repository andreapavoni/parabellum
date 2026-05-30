import type { PlayerProfileResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { Link } from "@/components/Link";

export function PlayerPage({ data }: { data: PlayerProfileResponse }) {
  return (
    <div class="mx-auto max-w-3xl px-4 py-6">
      <h1 class="text-2xl font-semibold text-gray-800">{data.username}</h1>
      <div class="mt-4 overflow-hidden rounded border bg-white shadow-sm">
        <table class="min-w-full text-sm">
          <thead class="bg-gray-100 text-left text-gray-600 uppercase text-xs tracking-wide">
            <tr>
              <th class="px-4 py-3">Village</th>
              <th class="px-4 py-3">Coordinates</th>
              <th class="px-4 py-3 text-right">Distance</th>
              <th class="px-4 py-3 text-right">Population</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200">
            {data.villages.map((village) => (
              <tr key={village.villageId}>
                <td class="px-4 py-3 font-semibold text-gray-800">
                  <Link to={`/map/field/${village.villageId}`} class="text-green-700 hover:underline">
                    {village.name}
                  </Link>
                  {village.isCapital ? <CapitalBadge compact /> : null}
                </td>
                <td class="px-4 py-3 text-gray-600">
                  <Link to={`/map/field/${village.villageId}`} class="text-green-700 hover:underline">
                    ({village.x}|{village.y})
                  </Link>
                </td>
                <td class="px-4 py-3 text-right text-gray-700">{village.distanceFromCurrent}</td>
                <td class="px-4 py-3 text-right text-gray-800">{village.population}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
