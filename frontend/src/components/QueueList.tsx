import type { BuildingQueueItem } from "@/types/api";
import { Link } from "./Link";
import { buildingLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { useCountdown } from "@/live/useCountdown";

function QueueTimer({ seconds, onElapsed }: { seconds: number; onElapsed?: () => void }) {
  const remaining = useCountdown(seconds, onElapsed);
  return <span class="font-semibold text-gray-800">{formatDurationHms(remaining)}</span>;
}

export function QueueList({
  queue,
  onQueueElapsed,
}: {
  queue: BuildingQueueItem[];
  onQueueElapsed?: () => void;
}) {
  return (
    <div class="w-full mt-4 flex flex-col text-[11px] text-gray-600 px-4 max-w-[400px] gap-1">
      <div class="font-bold text-gray-800 border-b border-gray-300 pb-1 mb-1">Building queue</div>
      {queue.length === 0 ? (
        <div class="text-xs text-gray-500">The queue is currently empty.</div>
      ) : (
        queue.map((item) => (
          <div class="flex justify-between w-full items-center" key={`${item.slotId}-${item.targetLevel}`}>
            <Link
              to={`/app/build/${item.slotId}`}
              class="flex items-center gap-2 text-gray-800 hover:text-gray-900 hover:underline"
            >
              <span class={item.isProcessing ? "text-green-600" : "text-yellow-600"}>⏳</span>
              {buildingLabel(item.buildingName)} (Lv {item.targetLevel})
            </Link>
            <QueueTimer seconds={item.timeSeconds} onElapsed={onQueueElapsed} />
          </div>
        ))
      )}
    </div>
  );
}
