import type { VillageResourcesResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { QueueList } from "@/components/QueueList";
import { ResourceFieldsMap } from "@/components/ResourceFieldsMap";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { Link } from "@/components/Link";
import { unitLabel } from "@/lib/labels";

export function ResourcesPage({
  data,
  onQueueElapsed,
}: {
  data: VillageResourcesResponse;
  onQueueElapsed?: () => void;
}) {
  const production = data.village.productionPerHour;

  return (
    <div class="container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12">
      <div class="flex flex-col items-center w-full md:w-auto">
        <h1 class="text-xl font-bold mb-4 w-full text-left">
          {data.village.name} ({data.village.x}|{data.village.y})
          {data.village.isCapital ? <CapitalBadge /> : null}
        </h1>
        <ResourceFieldsMap slots={data.resourceSlots} />
        <QueueList queue={data.buildingQueue} onQueueElapsed={onQueueElapsed} />
      </div>
      <div class="w-full max-w-[360px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none">
        <h3 class="font-bold mb-3 text-sm">Production</h3>
        <div class="text-xs space-y-3">
          <ProductionRow label="🌲 Lumber" value={production.lumber} />
          <ProductionRow label="🧱 Clay" value={production.clay} />
          <ProductionRow label="⛏️ Iron" value={production.iron} />
          <ProductionRow label="🌾 Crop" value={production.crop} />
        </div>
        <h3 class="font-bold mt-6 mb-3 text-sm">Current Troops</h3>
        {data.currentTroops.length === 0 ? (
          <div class="text-xs text-gray-500 border-b border-gray-100 pb-2">No troops stationed.</div>
        ) : (
          <div class="text-xs space-y-2">
            {data.currentTroops.map((troop) => (
              <div class="flex justify-between border-b border-gray-100 pb-2" key={troop.unitName}>
                <span class="inline-flex items-center gap-2">
                  <UnitSpriteByName unitName={troop.unitName} label={unitLabel(troop.unitName)} />
                  <span>{unitLabel(troop.unitName)}</span>
                </span>
                <span class="font-bold text-gray-900">{troop.count}</span>
              </div>
            ))}
          </div>
        )}
        <h3 class="font-bold mt-6 mb-3 text-sm">Troop Movements</h3>
        <div class="text-xs space-y-2">
          <MovementRow
            label="Incoming attacks/raids"
            count={data.troopMovementSummary.incomingAttacksRaids}
            href="/app/build/39#incoming"
          />
          <MovementRow
            label="Incoming returns/reinforcements"
            count={data.troopMovementSummary.incomingReturnsReinforcements}
            href="/app/build/39#incoming"
          />
          <MovementRow
            label="Outgoing attacks/raids"
            count={data.troopMovementSummary.outgoingAttacksRaids}
            href="/app/build/39#outgoing"
          />
          <MovementRow
            label="Outgoing reinforcements"
            count={data.troopMovementSummary.outgoingReinforcements}
            href="/app/build/39#outgoing"
          />
        </div>
      </div>
    </div>
  );
}

function ProductionRow({ label, value }: { label: string; value: number }) {
  return (
    <div class="flex justify-between border-b border-gray-100 pb-2">
      <span>{label}</span>
      <span class="font-bold text-gray-900">{value}/hour</span>
    </div>
  );
}

function MovementRow({ label, count, href }: { label: string; count: number; href: string }) {
  return (
    <div class="flex justify-between border-b border-gray-100 pb-2">
      <span>{label}</span>
      <Link to={href} class="font-bold text-green-700 hover:underline">
        {count}
      </Link>
    </div>
  );
}
