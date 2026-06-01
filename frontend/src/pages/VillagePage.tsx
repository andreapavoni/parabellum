import type { VillageOverviewResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { QueueList } from "@/components/QueueList";
import { VillageMap } from "@/components/VillageMap";
import { VillageRenameInline } from "@/components/VillageRenameInline";

export function VillagePage({
  data,
  onQueueElapsed,
  onVillageRenamed,
}: {
  data: VillageOverviewResponse;
  onQueueElapsed?: () => void;
  onVillageRenamed?: () => Promise<void> | void;
}) {
  return (
    <div class="container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col items-center gap-8 pb-12">
      <div class="flex flex-col items-center w-full md:w-auto">
        <h1 class="text-xl font-bold mb-4 w-full text-left">
          {data.village.name} ({data.village.x}|{data.village.y})
          {data.village.isCapital ? <CapitalBadge /> : null}
        </h1>
        <VillageRenameInline
          villageId={data.village.id}
          currentName={data.village.name}
          onRenamed={onVillageRenamed}
        />
        <div class="w-full mb-3">
          <span class="text-xs text-gray-600">Loyalty: </span>
          <span
            class={
              data.village.loyalty < 100
                ? "inline-flex items-center rounded px-2 py-0.5 text-xs font-semibold bg-amber-100 text-amber-800"
                : "text-xs font-semibold text-gray-800"
            }
          >
            {data.village.loyalty}%
          </span>
        </div>
        <VillageMap slots={data.buildingSlots} />
        <QueueList queue={data.buildingQueue} onQueueElapsed={onQueueElapsed} />
      </div>
    </div>
  );
}
