import { useEffect, useState } from "preact/hooks";
import { ArrowDownToLine } from "lucide-preact";
import { buildingLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { useServerDeadlineCountdown } from "@/live/useCountdown";
import { useDowngradeBuildingMutation } from "@/query/mutations";
import { Button, Field, Panel, SectionHeader } from "@/components/ui";
import { ResourceSprite } from "@/components/ResourceSprite";
import type { BuildingPageResponse } from "@/types/api";

type MainBuildingDetail = NonNullable<BuildingPageResponse["detail"]["mainBuilding"]>;

function DowngradeTimer({
  finishesAt,
  serverTime,
  serverTimeObservedAtMs,
  onElapsed,
}: {
  finishesAt: string;
  serverTime: number;
  serverTimeObservedAtMs: number;
  onElapsed: () => void;
}) {
  const remaining = useServerDeadlineCountdown(finishesAt, serverTime, serverTimeObservedAtMs, onElapsed);
  return (
    <span class="inline-flex items-center gap-1 font-mono text-[11px] font-semibold text-stone-800">
      <ResourceSprite kind="clock" size={14} label="Time remaining" />
      {formatDurationHms(remaining)}
    </span>
  );
}

export function MainBuilding({
  detail,
  serverTime,
  serverTimeObservedAtMs,
  onMutate,
}: {
  detail: MainBuildingDetail;
  serverTime: number;
  serverTimeObservedAtMs: number;
  onMutate: () => Promise<void>;
}) {
  const [selectedSlotId, setSelectedSlotId] = useState(detail.options[0]?.slotId ?? 0);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const downgradeBuilding = useDowngradeBuildingMutation();
  const selectedSlotStillAvailable = detail.options.some((option) => option.slotId === selectedSlotId);
  const currentSelectedSlotId = selectedSlotStillAvailable ? selectedSlotId : (detail.options[0]?.slotId ?? 0);
  const selected = detail.options.find((option) => option.slotId === currentSelectedSlotId);
  const disabled = !detail.canDowngrade || detail.queueFull || !selected || submitting;

  useEffect(() => {
    if (selectedSlotId !== currentSelectedSlotId) {
      setSelectedSlotId(currentSelectedSlotId);
    }
  }, [currentSelectedSlotId, selectedSlotId]);

  return (
    <Panel>
      <SectionHeader
        title="Downgrade buildings"
        aside={detail.queue.length > 0 ? `${detail.queue.length}/2 queued` : undefined}
      />

      <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(260px,360px)]">
        <div class="space-y-3">
          {!detail.canDowngrade ? (
            <p class="text-sm text-stone-600">Downgrades unlock when the Main Building reaches level 10.</p>
          ) : null}

          {detail.canDowngrade ? (
            <>
              <Field label="Building">
                <select
                  class="w-full rounded-md border border-stone-300 bg-white px-3 py-2 text-sm text-stone-800 shadow-sm focus:border-green-700 focus:outline-none focus:ring-2 focus:ring-green-100"
                  disabled={detail.queueFull || detail.options.length === 0 || submitting}
                  value={currentSelectedSlotId}
                  onChange={(event) => {
                    setSelectedSlotId(Number((event.currentTarget as HTMLSelectElement).value));
                  }}
                >
                  {detail.options.length === 0 ? (
                    <option value={0}>No available building</option>
                  ) : (
                    detail.options.map((option) => (
                      <option key={option.slotId} value={option.slotId}>
                        Slot {option.slotId} - {buildingLabel(option.buildingName)} level {option.currentLevel}
                      </option>
                    ))
                  )}
                </select>
              </Field>

              {selected ? (
                <div class="flex flex-wrap items-center gap-3 rounded-md border border-stone-200 bg-stone-50 px-3 py-2 text-sm text-stone-700">
                  <span class="font-semibold text-stone-900">{buildingLabel(selected.buildingName)}</span>
                  <span>Level {selected.currentLevel}</span>
                  <ArrowDownToLine size={16} aria-hidden="true" />
                  <span>Level {selected.nextLevel}</span>
                </div>
              ) : null}

              {detail.queueFull ? <p class="text-sm text-amber-700">Downgrade queue is full.</p> : null}
              {error ? <p class="text-sm text-red-700">{error}</p> : null}

              <Button
                type="button"
                variant="warning"
                disabled={disabled}
                onClick={async () => {
                  if (!selected) return;
                  setSubmitting(true);
                  setError(null);
                  try {
                    await downgradeBuilding.mutateAsync({ slotId: selected.slotId });
                    await onMutate();
                  } catch (err) {
                    setError((err as Error).message);
                  } finally {
                    setSubmitting(false);
                  }
                }}
              >
                <ArrowDownToLine size={16} aria-hidden="true" />
                Downgrade
              </Button>
            </>
          ) : null}
        </div>

        <div class="rounded-md border border-stone-200 bg-white px-3 py-2">
          <div class="mb-2 text-xs font-semibold uppercase text-stone-500">Downgrade queue</div>
          {detail.queue.length === 0 ? (
            <p class="text-sm text-stone-500">No active downgrades.</p>
          ) : (
            <div class="space-y-2">
              {detail.queue.map((item) => (
                <div
                  key={`${item.slotId}-${item.targetLevel}`}
                  class={`flex items-center justify-between gap-3 rounded-md border px-2 py-1.5 text-sm ${
                    item.isProcessing
                      ? "border-green-200 bg-green-50"
                      : "border-amber-200 bg-amber-50"
                  }`}
                >
                  <span class="min-w-0 truncate font-semibold text-stone-800">
                    {buildingLabel(item.buildingName)} to level {item.targetLevel}
                  </span>
                  <DowngradeTimer
                    finishesAt={item.finishesAt}
                    serverTime={serverTime}
                    serverTimeObservedAtMs={serverTimeObservedAtMs}
                    onElapsed={onMutate}
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </Panel>
  );
}
