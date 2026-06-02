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

const CANONICAL_UNIT_ORDER: string[] = [
  "Legionnaire", "Praetorian", "Imperian", "EquitesLegati", "EquitesImperatoris", "EquitesCaesaris", "BatteringRam", "FireCatapult", "Senator", "Settler",
  "Maceman", "Spearman", "Axeman", "Scout", "Paladin", "TeutonicKnight", "Ram", "Catapult", "Chief",
  "Phalanx", "Swordsman", "Pathfinder", "TheutatesThunder", "Druidrider", "Haeduan", "Trebuchet", "Chieftain",
  "Rat", "Spider", "Serpent", "Bat", "WildBoar", "Wolf", "Bear", "Crocodile", "Tiger", "Elephant",
  "Pikeman", "ThornedWarrior", "Guardsman", "BirdsOfPrey", "Axerider", "NatarianKnight", "Warelephant", "Ballista", "NatarianEmperor",
];

const UNIT_ORDER_INDEX = new Map<string, number>(
  CANONICAL_UNIT_ORDER.map((name, idx) => [name, idx]),
);

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
  const sortedCurrentTroops = [...data.currentTroops].sort((a, b) => {
    const ai = UNIT_ORDER_INDEX.get(a.unitName) ?? Number.MAX_SAFE_INTEGER;
    const bi = UNIT_ORDER_INDEX.get(b.unitName) ?? Number.MAX_SAFE_INTEGER;
    if (ai !== bi) return ai - bi;
    return a.unitName.localeCompare(b.unitName);
  });
  const clockSkewMs = clockSkewMsFromServerTime(data.serverTime);
  const movementRows = [
    {
      label: "incoming attacks/raids",
      count: data.troopMovementSummary.incomingAttacksRaids,
      nextAt: data.troopMovementSummary.incomingAttacksRaidsNextAt,
      href: "/app/build/39#incoming",
    },
    {
      label: "incoming returns/reinforcements",
      count: data.troopMovementSummary.incomingReturnsReinforcements,
      nextAt: data.troopMovementSummary.incomingReturnsReinforcementsNextAt,
      href: "/app/build/39#incoming",
    },
    {
      label: "outgoing attacks/raids",
      count: data.troopMovementSummary.outgoingAttacksRaids,
      nextAt: data.troopMovementSummary.outgoingAttacksRaidsNextAt,
      href: "/app/build/39#outgoing",
    },
    {
      label: "outgoing reinforcements",
      count: data.troopMovementSummary.outgoingReinforcements,
      nextAt: data.troopMovementSummary.outgoingReinforcementsNextAt,
      href: "/app/build/39#outgoing",
    },
  ].filter((row) => row.count > 0);
  const lastMovementRefreshAtRef = useRef(0);
  const onMovementElapsed = () => {
    if (!onQueueElapsed) return;
    const now = Date.now();
    if (now - lastMovementRefreshAtRef.current < 1000) return;
    lastMovementRefreshAtRef.current = now;
    onQueueElapsed();
  };

  return (
    <div class="mx-auto mt-4 md:mt-6 w-full max-w-4xl px-2 md:px-3 flex flex-col md:flex-row items-start gap-5 pb-12">
      <div class="flex flex-col items-start w-full md:max-w-[440px] md:flex-none">
        <div class="mb-4 flex w-full flex-wrap items-center gap-x-3 gap-y-1 text-left">
          <h1 class="text-xl font-bold">
            {data.village.name} ({data.village.x}|{data.village.y})
          </h1>
          {data.village.isCapital ? <CapitalBadge /> : null}
          <VillageRenameInline
            villageId={data.village.id}
            currentName={data.village.name}
            onRenamed={onVillageRenamed}
            label="rename"
            className="contents"
            linkClassName="p-0 text-xs text-green-700 underline hover:text-green-800 bg-transparent border-0"
          />
        </div>
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
      <div class="w-full md:w-52 md:shrink-0 pt-4 md:pt-0 border-t md:border-t-0 border-gray-200 md:border-none">
        <h3 class="font-bold mb-3 text-sm">Production</h3>
        <div class="text-xs space-y-2">
          <ProductionRow kind="lumber" value={production.lumber} />
          <ProductionRow kind="clay" value={production.clay} />
          <ProductionRow kind="iron" value={production.iron} />
          <ProductionRow kind="crop" value={production.crop} />
        </div>
        <h3 class="font-bold mt-6 mb-3 text-sm">Current Troops</h3>
        {sortedCurrentTroops.length === 0 ? (
          <div class="text-xs text-gray-500 border-b border-gray-100 pb-2">No troops stationed.</div>
        ) : (
          <div class="text-xs space-y-1.5">
            {sortedCurrentTroops.map((troop) => (
              <Link
                to="/app/build/39"
                class="flex justify-between border-b border-gray-100 pb-1.5 hover:underline"
                key={troop.unitName}
              >
                <span class="inline-flex items-center gap-2">
                  <UnitSpriteByName unitName={troop.unitName} label={unitLabel(troop.unitName)} />
                  <span>{unitLabel(troop.unitName)}</span>
                </span>
                <span class="font-bold text-gray-900">{troop.count}</span>
              </Link>
            ))}
          </div>
        )}
        {movementRows.length > 0 ? (
          <>
            <h3 class="font-bold mt-6 mb-3 text-sm">Troop Movements</h3>
            <div class="text-xs space-y-2">
              {movementRows.map((row) => (
                <MovementRow
                  key={row.label}
                  label={row.label}
                  count={row.count}
                  nextAt={row.nextAt}
                  href={row.href}
                  onElapsed={onMovementElapsed}
                  clockSkewMs={clockSkewMs}
                />
              ))}
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}

function ProductionRow({
  kind,
  value,
}: {
  kind: ResourceSpriteKind;
  value: number;
}) {
  return (
    <div class="flex justify-between border-b border-gray-100 pb-1.5">
      <span class="inline-flex items-center">
        <ResourceSprite kind={kind} size={22} />
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
    <div class="border-b border-gray-100 pb-2">
      <Link to={href} class="flex items-center justify-between gap-2 hover:underline">
        <span class="text-gray-800">
          <span class="font-semibold">{count}</span> {label}
        </span>
        {count > 0 && nextAt ? (
          <MovementCountdown nextAt={nextAt} onElapsed={onElapsed} clockSkewMs={clockSkewMs} />
        ) : null}
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
      {formatDurationHms(remaining)}
    </span>
  );
}
