import type { StatsResponse } from "@/types/api";
import { Link } from "@/components/Link";
import { DataTable } from "@/components/ui";
import { tribeLabel } from "@/lib/labels";

export function StatsPage({ data }: { data: StatsResponse }) {
  const hasPrev = data.pagination.page > 1;
  const hasNext = data.pagination.page < data.pagination.totalPages;

  return (
    <div class="max-w-2xl mx-auto space-y-4 px-4 py-6">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl mt-3 font-semibold text-gray-800">Leaderboard</h1>
        <div class="text-sm text-gray-600">Total players: {data.pagination.totalPlayers}</div>
      </div>
      <DataTable>
          <thead class="bg-gray-100 text-left text-gray-600 uppercase text-xs tracking-wide">
            <tr>
              <th class="px-4 py-3 w-16">#</th>
              <th class="px-4 py-3">Player</th>
              <th class="px-4 py-3">Tribe</th>
              <th class="px-4 py-3 text-right">Villages</th>
              <th class="px-4 py-3 text-right">Population</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200">
            {data.entries.map((entry) => (
              <tr class="hover:bg-gray-50" key={entry.playerId}>
                <td class="px-4 py-3 font-mono text-gray-600">{entry.rank}</td>
                <td class="px-4 py-3 font-semibold text-gray-800">
                  <Link to={`/players/${entry.playerId}`} class="text-green-700 hover:underline">
                    {entry.username}
                  </Link>
                </td>
                <td class="px-4 py-3 text-gray-700">{tribeLabel(entry.tribe)}</td>
                <td class="px-4 py-3 text-right text-gray-700">{entry.villageCount}</td>
                <td class="px-4 py-3 text-right text-gray-900 font-semibold">{entry.population}</td>
              </tr>
            ))}
          </tbody>
      </DataTable>
      <div class="flex items-center justify-between text-sm text-gray-600">
        <span>
          Page {data.pagination.page} / {data.pagination.totalPages}
        </span>
        <div class="flex items-center gap-2">
          {hasPrev ? (
            <Link to={`/stats?page=${data.pagination.page - 1}`} class="px-3 py-1 rounded border border-gray-300 bg-white hover:bg-gray-50">
              Prev
            </Link>
          ) : (
            <span class="px-3 py-1 rounded border border-gray-200 text-gray-400 cursor-not-allowed bg-gray-50">Prev</span>
          )}
          {hasNext ? (
            <Link to={`/stats?page=${data.pagination.page + 1}`} class="px-3 py-1 rounded border border-gray-300 bg-white hover:bg-gray-50">
              Next
            </Link>
          ) : (
            <span class="px-3 py-1 rounded border border-gray-200 text-gray-400 cursor-not-allowed bg-gray-50">Next</span>
          )}
        </div>
      </div>
    </div>
  );
}
