import type { ResourcesPageResponse } from "@/types/api";
import { QueueList } from "@/components/QueueList";
import { ResourceFieldsMap } from "@/components/ResourceFieldsMap";

export function ResourcesPage({ data }: { data: ResourcesPageResponse }) {
  const production = data.village.productionPerHour;

  return (
    <div class="container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12">
      <div class="flex flex-col items-center w-full md:w-auto">
        <h1 class="text-xl font-bold mb-4 w-full text-left">
          {data.village.name} ({data.village.x}|{data.village.y})
        </h1>
        <ResourceFieldsMap slots={data.resourceSlots} />
        <QueueList queue={data.buildingQueue} />
      </div>
      <div class="w-full max-w-[360px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none">
        <h3 class="font-bold mb-3 text-sm">Production</h3>
        <div class="text-xs space-y-3">
          <ProductionRow label="🌲 Lumber" value={production.lumber} />
          <ProductionRow label="🧱 Clay" value={production.clay} />
          <ProductionRow label="⛏️ Iron" value={production.iron} />
          <ProductionRow label="🌾 Crop" value={production.crop} />
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
