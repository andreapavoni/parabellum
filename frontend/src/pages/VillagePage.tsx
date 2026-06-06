import type { BuildingQueueItem, BuildingSlot, VillageListItem, VillageSummary } from "@/types/api";
import { QueueList } from "@/components/QueueList";
import { VillageMap } from "@/components/VillageMap";
import { VillageHeading, VillageSelector } from "@/components/VillageHeader";
import { Panel } from "@/components/ui";

export function VillagePage({
  data,
  onQueueElapsed,
  onVillageRenamed,
  onSwitchVillage,
}: {
  data: {
    serverTime: number;
    village: VillageSummary;
    buildingSlots: BuildingSlot[];
    buildingQueue: BuildingQueueItem[];
    villages: VillageListItem[];
  };
  onQueueElapsed?: () => void;
  onVillageRenamed?: () => Promise<void> | void;
  onSwitchVillage: (villageId: number) => void;
}) {
  return (
    <div class="mx-auto mt-3 md:mt-4 w-full max-w-5xl px-2 md:px-3 pb-10">
      <VillageHeading village={data.village} onVillageRenamed={onVillageRenamed} />
      <div class="mt-3 flex flex-col items-start gap-4 md:flex-row">
        <div class="flex flex-col items-center w-full max-w-[400px] md:flex-none">
          <VillageMap slots={data.buildingSlots} />
          <QueueList queue={data.buildingQueue} onQueueElapsed={onQueueElapsed} />
        </div>
        <Panel class="w-full md:w-56 md:shrink-0">
          <VillageSelector villages={data.villages} onSwitchVillage={onSwitchVillage} />
        </Panel>
      </div>
    </div>
  );
}
