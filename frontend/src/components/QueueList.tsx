import type { BuildingQueueItem } from "@/types/api";
import { X } from "lucide-preact";
import { Link } from "./Link";
import { buildingLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { useServerDeadlineCountdown } from "@/live/useCountdown";
import { useCancelBuildingConstructionMutation } from "@/query/mutations";
import { ResourceSprite } from "@/components/ResourceSprite";

function QueueTimer({
  finishesAt,
  serverTime,
  serverTimeObservedAtMs,
  onElapsed,
}: {
  finishesAt: string;
  serverTime: number;
  serverTimeObservedAtMs: number;
  onElapsed?: () => void;
}) {
  const remaining = useServerDeadlineCountdown(finishesAt, serverTime, serverTimeObservedAtMs, onElapsed);
  return (
    <span class="inline-flex items-center gap-1 font-mono text-[11px] font-semibold text-gray-800">
      <ResourceSprite kind="clock" size={14} label="Time remaining" />
      {formatDurationHms(remaining)}
    </span>
  );
}

export function QueueList({
  queue,
  serverTime,
  serverTimeObservedAtMs,
  onQueueElapsed,
}: {
  queue: BuildingQueueItem[];
  serverTime: number;
  serverTimeObservedAtMs: number;
  onQueueElapsed?: () => void;
}) {
  const cancelBuilding = useCancelBuildingConstructionMutation();

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
              class="grid w-full grid-cols-[24px_minmax(0,1fr)_auto] items-center gap-2 py-1"
              key={item.actionId}
            >
              {!item.isProcessing ? (
                <button
                  type="button"
                  title="Cancel construction"
                  class="inline-flex h-6 w-6 shrink-0 items-center justify-center text-red-600 hover:text-red-800 disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={cancelBuilding.isPending}
                  onClick={async () => {
                    if (
                      !window.confirm(
                        "Cancel this construction and any later queued work for this slot?",
                      )
                    ) {
                      return;
                    }
                    await cancelBuilding.mutateAsync({ actionId: item.actionId });
                    onQueueElapsed?.();
                  }}
                >
                  <X size={14} aria-hidden="true" />
                </button>
              ) : (
                <span aria-hidden="true" />
              )}
              <Link
                to={`/app/build/${item.slotId}`}
                class="min-w-0 truncate font-semibold text-gray-800 hover:text-gray-900 hover:underline"
              >
                {buildingLabel(item.buildingName)} (Lv {item.targetLevel})
              </Link>
              <QueueTimer
                finishesAt={item.finishesAt}
                serverTime={serverTime}
                serverTimeObservedAtMs={serverTimeObservedAtMs}
                onElapsed={onQueueElapsed}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
