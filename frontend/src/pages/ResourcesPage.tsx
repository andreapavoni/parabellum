import { useEffect, useRef, useState } from "preact/hooks";
import type { VillageResourcesResponse } from "@/types/api";
import { CapitalBadge } from "@/components/CapitalBadge";
import { QueueList } from "@/components/QueueList";
import { ResourceFieldsMap } from "@/components/ResourceFieldsMap";
import { ResourceSprite, type ResourceSpriteKind } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { Link } from "@/components/Link";
import { VillageRenameInline } from "@/components/VillageRenameInline";
import { unitLabel } from "@/lib/labels";
import { clockSkewMsFromServerTime, formatDurationHms, secondsUntilIso } from "@/lib/time";

export function ResourcesPage({
  data,
  onQueueElapsed,
  onVillageRenamed,
}: {
  data: VillageResourcesResponse;
  onQueueElapsed?: () => void;
  onVillageRenamed?: () => Promise<void> | void;
}) {
  const production = data.village.productionPerHour;
  const clockSkewMs = clockSkewMsFromServerTime(data.serverTime);
  const lastMovementRefreshAtRef = useRef(0);
  const onMovementElapsed = () => {
    if (!onQueueElapsed) return;
    const now = Date.now();
    if (now - lastMovementRefreshAtRef.current < 1000) return;
    lastMovementRefreshAtRef.current = now;
    onQueueElapsed();
  };

  return (
    <div class="mt-4 md:mt-6 px-2 md:px-0 flex flex-col md:flex-row items-start gap-8 pb-12">
      <div class="flex flex-col items-start w-full md:flex-1">
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
        <ResourceFieldsMap slots={data.resourceSlots} />
        <QueueList queue={data.buildingQueue} onQueueElapsed={onQueueElapsed} />
      </div>
      <div class="w-full md:w-64 md:shrink-0 pt-4 md:pt-0 border-t md:border-t-0 border-gray-200 md:border-none">
        <h3 class="font-bold mb-3 text-sm">Production</h3>
        <div class="text-xs space-y-3">
          <ProductionRow kind="lumber" label="Lumber" value={production.lumber} />
          <ProductionRow kind="clay" label="Clay" value={production.clay} />
          <ProductionRow kind="iron" label="Iron" value={production.iron} />
          <ProductionRow kind="crop" label="Crop" value={production.crop} />
        </div>
        <h3 class="font-bold mt-6 mb-3 text-sm">Current Troops</h3>
        {data.currentTroops.length === 0 ? (
          <div class="text-xs text-gray-500 border-b border-gray-100 pb-2">No troops stationed.</div>
        ) : (
          <div class="text-xs space-y-2">
            {data.currentTroops.map((troop) => (
              <div class="flex justify-between border-b border-gray-100 pb-2" key={troop.unitName}>
                <span class="inline-flex items-center gap-2">
                  <UnitSpriteByName unitName={troop.unitName} label={unitLabel(troop.unitName)} />
                  <span>{unitLabel(troop.unitName)}</span>
                </span>
                <span class="font-bold text-gray-900">{troop.count}</span>
              </div>
            ))}
          </div>
        )}
        <h3 class="font-bold mt-6 mb-3 text-sm">Troop Movements</h3>
        <div class="text-xs space-y-2">
          <MovementRow
            label="Incoming attacks/raids"
            count={data.troopMovementSummary.incomingAttacksRaids}
            nextAt={data.troopMovementSummary.incomingAttacksRaidsNextAt}
            href="/app/build/39#incoming"
            onElapsed={onMovementElapsed}
            clockSkewMs={clockSkewMs}
          />
          <MovementRow
            label="Incoming returns/reinforcements"
            count={data.troopMovementSummary.incomingReturnsReinforcements}
            nextAt={data.troopMovementSummary.incomingReturnsReinforcementsNextAt}
            href="/app/build/39#incoming"
            onElapsed={onMovementElapsed}
            clockSkewMs={clockSkewMs}
          />
          <MovementRow
            label="Outgoing attacks/raids"
            count={data.troopMovementSummary.outgoingAttacksRaids}
            nextAt={data.troopMovementSummary.outgoingAttacksRaidsNextAt}
            href="/app/build/39#outgoing"
            onElapsed={onMovementElapsed}
            clockSkewMs={clockSkewMs}
          />
          <MovementRow
            label="Outgoing reinforcements"
            count={data.troopMovementSummary.outgoingReinforcements}
            nextAt={data.troopMovementSummary.outgoingReinforcementsNextAt}
            href="/app/build/39#outgoing"
            onElapsed={onMovementElapsed}
            clockSkewMs={clockSkewMs}
          />
        </div>
      </div>
    </div>
  );
}

function ProductionRow({
  kind,
  label,
  value,
}: {
  kind: ResourceSpriteKind;
  label: string;
  value: number;
}) {
  return (
    <div class="flex justify-between border-b border-gray-100 pb-2">
      <span class="inline-flex items-center gap-1.5">
        <ResourceSprite kind={kind} size={16} label={label} />
        <span>{label}</span>
      </span>
      <span class="font-bold text-gray-900">{value}/hour</span>
    </div>
  );
}

function MovementRow({
  label,
  count,
  href,
  nextAt,
  onElapsed,
  clockSkewMs,
}: {
  label: string;
  count: number;
  href: string;
  nextAt?: string;
  onElapsed?: () => void;
  clockSkewMs: number;
}) {
  return (
    <div class="flex justify-between border-b border-gray-100 pb-2">
      <span class="inline-flex items-center gap-2">
        <span>{label}</span>
        {count > 0 && nextAt ? (
          <MovementCountdown nextAt={nextAt} onElapsed={onElapsed} clockSkewMs={clockSkewMs} />
        ) : null}
      </span>
      <Link to={href} class="font-bold text-green-700 hover:underline">
        {count}
      </Link>
    </div>
  );
}

function MovementCountdown({
  nextAt,
  onElapsed,
  clockSkewMs,
}: {
  nextAt: string;
  onElapsed?: () => void;
  clockSkewMs: number;
}) {
  const ZERO_RETRY_MAX = 5;
  const ZERO_RETRY_DELAY_MS = 1200;
  const [remaining, setRemaining] = useState(secondsUntilIso(nextAt, { clockSkewMs }));
  const startedFromPositiveRef = useRef(secondsUntilIso(nextAt, { clockSkewMs }) > 0);
  const notifiedRef = useRef(false);
  const zeroRetryCountRef = useRef(0);

  useEffect(() => {
    const secs = secondsUntilIso(nextAt, { clockSkewMs });
    setRemaining(secs);
    startedFromPositiveRef.current = secs > 0;
    notifiedRef.current = false;
    if (secs > 0) {
      zeroRetryCountRef.current = 0;
    }
  }, [nextAt, clockSkewMs]);

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

  useEffect(() => {
    if (
      !onElapsed ||
      remaining > 0 ||
      startedFromPositiveRef.current ||
      zeroRetryCountRef.current >= ZERO_RETRY_MAX
    ) {
      return;
    }

    const retryTimer = window.setTimeout(() => {
      zeroRetryCountRef.current += 1;
      onElapsed();
    }, ZERO_RETRY_DELAY_MS);

    return () => window.clearTimeout(retryTimer);
  }, [remaining, onElapsed]);

  const urgencyClass = remaining <= 60 ? "text-amber-700" : "text-gray-500";
  return (
    <span class={`font-mono text-[11px] ${urgencyClass}`}>
      Next: {formatDurationHms(remaining)}
    </span>
  );
}
