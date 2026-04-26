import { useEffect, useRef, useState } from "preact/hooks";
import type { BuildingQueueItem } from "@/types/api";
import { Link } from "./Link";
import { buildingLabel } from "@/lib/labels";

function formatDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((value) => value.toString().padStart(2, "0")).join(":");
}

function QueueTimer({ seconds, onElapsed }: { seconds: number; onElapsed?: () => void }) {
  const [remaining, setRemaining] = useState(seconds);
  const notifiedRef = useRef(false);
  const startedFromPositiveRef = useRef(seconds > 0);

  useEffect(() => {
    setRemaining(seconds);
    startedFromPositiveRef.current = seconds > 0;
    notifiedRef.current = seconds <= 0;
  }, [seconds]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setRemaining((value) => Math.max(0, value - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (
      !onElapsed ||
      notifiedRef.current ||
      remaining > 0 ||
      !startedFromPositiveRef.current
    ) {
      return;
    }
    notifiedRef.current = true;
    onElapsed();
  }, [remaining, onElapsed]);

  return <span class="font-semibold text-gray-800">{formatDuration(remaining)}</span>;
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
