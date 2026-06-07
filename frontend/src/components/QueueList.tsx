import type { BuildingQueueItem } from "@/types/api";
import { Link } from "./Link";
import { buildingLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { useCountdown } from "@/live/useCountdown";
import { ResourceSprite } from "@/components/ResourceSprite";

function QueueTimer({ seconds, onElapsed }: { seconds: number; onElapsed?: () => void }) {
  const remaining = useCountdown(seconds, onElapsed);
  return (
    <span class="inline-flex items-center gap-1 font-mono text-[11px] font-semibold text-gray-800">
      <ResourceSprite kind="clock" size={14} label="Time remaining" />
      {formatDurationHms(remaining)}
    </span>
  );
}

export function QueueList({
  queue,
  onQueueElapsed,
}: {
  queue: BuildingQueueItem[];
  onQueueElapsed?: () => void;
}) {
  return (
    <div class="w-full mt-4 max-w-[400px] rounded-md border border-stone-300 bg-white px-3 py-2 text-[11px] text-gray-600 shadow-sm">
      <div class="mb-2 flex items-center justify-between border-b border-stone-200 pb-1.5">
        <div class="font-bold text-gray-900">Building queue</div>
        {queue.length > 0 ? <span class="font-mono text-stone-500">{queue.length}</span> : null}
      </div>
      {queue.length === 0 ? (
        <div class="text-xs text-gray-500">The queue is currently empty.</div>
      ) : (
        <div class="space-y-1.5">
          {queue.map((item) => (
            <div
              class={`flex w-full items-center justify-between gap-3 rounded border px-2 py-1.5 ${
                item.isProcessing
                  ? "border-green-200 bg-green-50"
                  : "border-amber-200 bg-amber-50"
              }`}
              key={`${item.slotId}-${item.targetLevel}`}
            >
              <Link
                to={`/app/build/${item.slotId}`}
                class="min-w-0 truncate font-semibold text-gray-800 hover:text-gray-900 hover:underline"
              >
                {buildingLabel(item.buildingName)} (Lv {item.targetLevel})
              </Link>
              <QueueTimer seconds={item.timeSeconds} onElapsed={onQueueElapsed} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
