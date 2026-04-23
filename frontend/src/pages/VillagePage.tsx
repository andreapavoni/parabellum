import type { VillagePageResponse } from "@/types/api";
import { QueueList } from "@/components/QueueList";
import { VillageMap } from "@/components/VillageMap";

export function VillagePage({ data }: { data: VillagePageResponse }) {
  return (
    <div class="container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col items-center gap-8 pb-12">
      <div class="flex flex-col items-center w-full md:w-auto">
        <h1 class="text-xl font-bold mb-4 w-full text-left">
          {data.village.name} ({data.village.x}|{data.village.y})
        </h1>
        <VillageMap slots={data.buildingSlots} />
        <QueueList queue={data.buildingQueue} />
      </div>
    </div>
  );
}
