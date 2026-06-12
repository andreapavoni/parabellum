import { useEffect, useState } from "preact/hooks";
import { ResourceSprite } from "@/components/ResourceSprite";
import { Button, Panel, SectionHeader } from "@/components/ui";
import { formatDurationHms } from "@/lib/time";
import { useServerDeadlineCountdown } from "@/live/useCountdown";
import { useBuildTrapsMutation } from "@/query/mutations";
import type { BuildingPageResponse, ResourceAmounts, TrapQueueItem } from "@/types/api";

function resourceAmount(resources: ResourceAmounts, key: keyof ResourceAmounts) {
  return Number(resources[key] ?? 0);
}

function totalTrapCost(costPerTrap: ResourceAmounts, quantity: number): ResourceAmounts {
  return {
    lumber: resourceAmount(costPerTrap, "lumber") * quantity,
    clay: resourceAmount(costPerTrap, "clay") * quantity,
    iron: resourceAmount(costPerTrap, "iron") * quantity,
    crop: resourceAmount(costPerTrap, "crop") * quantity,
  };
}

function TrapResourceCost({ cost }: { cost: ResourceAmounts }) {
  return (
    <div class="flex flex-wrap items-center gap-3 text-xs text-stone-700">
      <span class="inline-flex items-center gap-1">
        <ResourceSprite kind="lumber" size={12} label="Lumber" />
        {cost.lumber}
      </span>
      <span class="inline-flex items-center gap-1">
        <ResourceSprite kind="clay" size={12} label="Clay" />
        {cost.clay}
      </span>
      <span class="inline-flex items-center gap-1">
        <ResourceSprite kind="iron" size={12} label="Iron" />
        {cost.iron}
      </span>
      <span class="inline-flex items-center gap-1">
        <ResourceSprite kind="crop" size={12} label="Crop" />
        {cost.crop}
      </span>
    </div>
  );
}

function clampQuantity(value: number, max: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(max, Math.trunc(value)));
}

function TrapQueueRow({
  job,
  serverTime,
  serverTimeObservedAtMs,
  onElapsed,
}: {
  job: TrapQueueItem;
  serverTime: number;
  serverTimeObservedAtMs: number;
  onElapsed: () => void;
}) {
  const nextTrapSeconds = useServerDeadlineCountdown(
    job.finishesAt,
    serverTime,
    serverTimeObservedAtMs,
    onElapsed,
  );
  const batchRemainingSeconds = nextTrapSeconds + Math.max(0, job.quantity - 1) * job.timePerTrap;

  return (
    <div class="rounded-md border border-stone-200 bg-white p-3 text-sm">
      <div class="flex items-center justify-between gap-3 font-semibold text-stone-800">
        <span>{job.quantity} traps</span>
        <span class="text-xs text-stone-500">
          {job.isProcessing ? "Building" : "Queued"}
        </span>
      </div>
      <div class="mt-1 flex flex-wrap items-center justify-between gap-x-3 gap-y-1 text-xs text-stone-600">
        <span class="inline-flex items-center gap-1">
          <ResourceSprite kind="clock" size={12} label="Next trap completion" />
          Next in <span class="font-mono text-stone-800">{formatDurationHms(nextTrapSeconds)}</span>
        </span>
        <span>
          Batch time <span class="font-mono text-stone-800">{formatDurationHms(batchRemainingSeconds)}</span>
        </span>
      </div>
    </div>
  );
}

export function TrapperBuilding({
  detail,
  serverTime,
  serverTimeObservedAtMs,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  serverTime: number;
  serverTimeObservedAtMs: number;
  onMutate: () => Promise<void>;
}) {
  const trapper = detail.trapper ?? detail.rallyPoint?.trapper;
  const [quantity, setQuantity] = useState(() => Math.min(1, trapper?.buildable ?? 0));
  const [error, setError] = useState<string | null>(null);
  const buildTraps = useBuildTrapsMutation();

  useEffect(() => {
    setQuantity((current) => clampQuantity(current || Math.min(1, trapper?.buildable ?? 0), trapper?.buildable ?? 0));
  }, [trapper?.buildable]);

  if (!trapper) return null;

  const totalCost = totalTrapCost(trapper.costPerTrap, quantity);
  const totalSeconds = trapper.timePerTrapSeconds * quantity;
  const canSubmit = quantity > 0 && !buildTraps.isPending;

  const stats = [
    { label: "Capacity", value: trapper.capacity },
    { label: "Active", value: trapper.active },
    { label: "Occupied", value: trapper.occupied },
    { label: "Broken", value: trapper.broken },
    { label: "Queued", value: trapper.queued },
    { label: "Buildable", value: trapper.buildable },
  ];

  return (
    <Panel class="space-y-4">
      <SectionHeader title="Traps" />
      <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
        {stats.map((stat) => (
          <div key={stat.label} class="rounded-md border border-stone-200 bg-stone-50 px-3 py-2">
            <div class="text-[11px] font-semibold uppercase text-stone-500">{stat.label}</div>
            <div class="text-lg font-semibold text-stone-900">{stat.value}</div>
          </div>
        ))}
      </div>

      <div class="grid gap-3 rounded-md border border-stone-200 bg-stone-50 p-3 md:grid-cols-[140px_1fr_auto] md:items-end">
        <label class="text-sm font-medium text-stone-600">
          Quantity
          <input
            type="number"
            min={0}
            max={trapper.buildable}
            value={quantity}
            disabled={trapper.buildable === 0 || buildTraps.isPending}
            class="mt-1 w-full rounded border border-stone-300 bg-white px-2 py-1.5 text-right text-stone-900 disabled:bg-stone-100 disabled:text-stone-400"
            onInput={(event) => {
              const value = Number((event.currentTarget as HTMLInputElement).value || "0");
              setQuantity(clampQuantity(value, trapper.buildable));
            }}
          />
        </label>
        <div class="space-y-1 text-sm text-stone-600">
          <div class="text-xs font-semibold uppercase text-stone-500">Cost and time</div>
          <TrapResourceCost cost={quantity > 0 ? totalCost : trapper.costPerTrap} />
          <div class="inline-flex items-center gap-1 text-xs text-stone-700">
            <ResourceSprite kind="clock" size={12} label="Build time" />
            {quantity > 0 ? formatDurationHms(totalSeconds) : `${formatDurationHms(trapper.timePerTrapSeconds)} each`}
          </div>
        </div>
        <Button
          type="button"
          variant="primary"
          disabled={!canSubmit}
          onClick={async () => {
            setError(null);
            try {
              await buildTraps.mutateAsync({
                villageId: detail.villageId,
                quantity,
              });
              await onMutate();
            } catch (err) {
              const message = err instanceof Error ? err.message : "Unable to queue traps";
              setError(message);
            }
          }}
        >
          {buildTraps.isPending ? "Queuing..." : trapper.broken > 0 ? "Repair traps" : "Build traps"}
        </Button>
      </div>

      {trapper.buildable === 0 ? (
        <p class="text-sm text-stone-500">No traps can be queued at the current Trapper capacity.</p>
      ) : null}
      {trapper.queue.length > 0 ? (
        <div class="space-y-2 rounded-md border border-stone-200 bg-stone-50 p-3">
          <SectionHeader title="Trap queue" class="mb-2" />
          {trapper.queue.map((job, index) => (
            <TrapQueueRow
              key={`${job.finishesAt}-${index}`}
              job={job}
              serverTime={serverTime}
              serverTimeObservedAtMs={serverTimeObservedAtMs}
              onElapsed={() => {
                void onMutate();
              }}
            />
          ))}
        </div>
      ) : null}
      {error ? <p class="text-sm text-red-600">{error}</p> : null}
    </Panel>
  );
}
