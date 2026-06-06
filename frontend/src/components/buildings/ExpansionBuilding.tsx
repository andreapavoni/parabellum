import type { BuildingPageResponse } from "@/types/api";

export function ExpansionBuilding({
  expansion,
}: {
  expansion: NonNullable<BuildingPageResponse["detail"]["expansion"]>;
}) {
  return (
    <div class="space-y-4">
      <div class="bg-white border border-gray-300 rounded-lg p-6 shadow-sm">
        <h2 class="text-xl font-semibold mb-4 text-gray-800">Culture Points</h2>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-blue-50 p-4 rounded-lg">
            <div class="text-sm text-gray-600 mb-1">Village Production</div>
            <div class="text-2xl font-bold text-blue-700">
              {expansion.villageCulturePointsProduction}
              <span class="text-sm font-normal text-gray-600 ml-1">/ day</span>
            </div>
          </div>
          <div class="bg-green-50 p-4 rounded-lg">
            <div class="text-sm text-gray-600 mb-1">Total Production</div>
            <div class="text-2xl font-bold text-green-700">
              {expansion.accountCulturePointsProduction}
              <span class="text-sm font-normal text-gray-600 ml-1">/ day</span>
            </div>
          </div>
          <div class="bg-purple-50 p-4 rounded-lg">
            <div class="text-sm text-gray-600 mb-1">Total Culture Points</div>
            <div class="text-2xl font-bold text-purple-700">{expansion.accountCulturePoints}</div>
          </div>
        </div>
        <div class="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
          <div class="text-sm font-medium text-gray-700">
            Next village requires:{" "}
            <span class="font-bold text-yellow-700">{expansion.nextCpRequired}</span>{" "}
            Culture Points
          </div>
        </div>
        <div class="mt-3">
          <span class="text-sm text-gray-600">Loyalty: </span>
          <span
            class={
              expansion.loyalty < 100
                ? "inline-flex items-center rounded px-2 py-0.5 text-xs font-semibold bg-amber-100 text-amber-800"
                : "text-sm font-semibold text-gray-800"
            }
          >
            {expansion.loyalty}%
          </span>
        </div>
      </div>

      <div class="bg-white border border-gray-300 rounded-lg p-6 shadow-sm">
        <h2 class="text-xl font-semibold mb-4 text-gray-800">Expansion</h2>
        <h3 class="text-lg font-medium mb-2 text-gray-700">Foundation Slots</h3>
        <div class="flex items-center gap-2 flex-wrap">
          {Array.from({ length: expansion.maxFoundationSlots }).map((_, i) =>
            i < expansion.childVillagesCount ? (
              <div
                key={i}
                class="w-14 h-14 bg-red-500 border-2 border-red-700 rounded flex items-center justify-center"
              >
                <span class="text-white font-bold">✓</span>
              </div>
            ) : (
              <div
                key={i}
                class="w-14 h-14 bg-green-500 border-2 border-green-700 rounded flex items-center justify-center"
              >
                <span class="text-white font-bold">○</span>
              </div>
            ),
          )}
        </div>
        <div class="text-sm text-gray-600 mt-2">
          {expansion.childVillagesCount} / {expansion.maxFoundationSlots} slots
          used
        </div>
      </div>
    </div>
  );
}

