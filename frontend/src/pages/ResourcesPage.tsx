import { useRef } from "preact/hooks";
import type {
  BuildingQueueItem,
  CurrentTroop,
  ResourceSlot,
  TroopMovementSummary,
  VillageListItem,
  VillageSummary,
} from "@/types/api";
import { QueueList } from "@/components/QueueList";
import { ResourceFieldsMap } from "@/components/ResourceFieldsMap";
import { ResourceSprite, type ResourceSpriteKind } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { Link } from "@/components/Link";
import { VillageHeading, VillageSelector } from "@/components/VillageHeader";
import { Panel, SectionHeader } from "@/components/ui";
import { unitLabel } from "@/lib/labels";
import { clockSkewMsFromServerTime, formatDurationHms, secondsUntilIso } from "@/lib/time";
import { useCountdown } from "@/live/useCountdown";

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
  onSwitchVillage,
}: {
  data: {
    serverTime: number;
    village: VillageSummary;
    resourceSlots: ResourceSlot[];
    buildingQueue: BuildingQueueItem[];
    currentTroops: CurrentTroop[];
    troopMovementSummary: TroopMovementSummary;
    villages: VillageListItem[];
  };
  onQueueElapsed?: () => void;
  onVillageRenamed?: () => Promise<void> | void;
  onSwitchVillage: (villageId: number) => void;
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
      label: "incoming attacks",
      count: data.troopMovementSummary.incomingAttacks,
      nextAt: data.troopMovementSummary.incomingAttacksNextAt,
      href: "/app/build/39#incoming",
      tone: "danger" as const,
    },
    {
      label: "incoming raids",
      count: data.troopMovementSummary.incomingRaids,
      nextAt: data.troopMovementSummary.incomingRaidsNextAt,
      href: "/app/build/39#incoming",
      tone: "warning" as const,
    },
    {
      label: "incoming returns/reinforcements",
      count: data.troopMovementSummary.incomingReturnsReinforcements,
      nextAt: data.troopMovementSummary.incomingReturnsReinforcementsNextAt,
      href: "/app/build/39#incoming",
      tone: "info" as const,
    },
    {
      label: "outgoing attacks",
      count: data.troopMovementSummary.outgoingAttacks,
      nextAt: data.troopMovementSummary.outgoingAttacksNextAt,
      href: "/app/build/39#outgoing",
      tone: "success" as const,
    },
    {
      label: "outgoing raids",
      count: data.troopMovementSummary.outgoingRaids,
      nextAt: data.troopMovementSummary.outgoingRaidsNextAt,
      href: "/app/build/39#outgoing",
      tone: "success" as const,
    },
    {
      label: "outgoing reinforcements",
      count: data.troopMovementSummary.outgoingReinforcements,
      nextAt: data.troopMovementSummary.outgoingReinforcementsNextAt,
      href: "/app/build/39#outgoing",
      tone: "violet" as const,
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
    <div class="mx-auto mt-3 md:mt-4 w-full max-w-5xl px-2 md:px-3 pb-10">
      <VillageHeading village={data.village} onVillageRenamed={onVillageRenamed} />
      <div class="mt-3 flex w-full flex-col items-start justify-center gap-4 md:flex-row">
        <div class="flex flex-col items-start w-full md:max-w-[440px] md:flex-none">
          <ResourceFieldsMap slots={data.resourceSlots} />
          <QueueList queue={data.buildingQueue} onQueueElapsed={onQueueElapsed} />
        </div>
        <Panel class="w-full md:w-56 md:shrink-0 space-y-5">
          <VillageSelector villages={data.villages} onSwitchVillage={onSwitchVillage} />
          <div>
            <SectionHeader title="Production" />
            <div class="text-xs space-y-2">
              <ProductionRow kind="lumber" value={production.lumber} />
              <ProductionRow kind="clay" value={production.clay} />
              <ProductionRow kind="iron" value={production.iron} />
              <ProductionRow kind="crop" value={production.crop} />
            </div>
          </div>
          <div>
            <SectionHeader title="Current Troops" />
            {sortedCurrentTroops.length === 0 ? (
              <div class="text-xs text-stone-500 border-b border-stone-100 pb-2">No troops stationed.</div>
            ) : (
              <div class="text-xs space-y-1.5">
                {sortedCurrentTroops.map((troop) => (
                  <Link
                    to="/app/build/39"
                    class="flex justify-between border-b border-stone-100 pb-1.5 hover:underline"
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
          </div>
          {movementRows.length > 0 ? (
            <div>
              <SectionHeader title="Troop Movements" />
              <div class="text-xs space-y-2">
                {movementRows.map((row) => (
                  <MovementRow
                    key={row.label}
                    label={row.label}
                    count={row.count}
                    nextAt={row.nextAt}
                    href={row.href}
                    tone={row.tone}
                    onElapsed={onMovementElapsed}
                    clockSkewMs={clockSkewMs}
                  />
                ))}
              </div>
            </div>
          ) : null}
        </Panel>
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
    <div class="flex justify-between border-b border-stone-100 pb-1.5">
      <span class="inline-flex items-center">
        <ResourceSprite kind={kind} size={22} />
      </span>
      <span class="font-bold text-stone-900">{value}/hour</span>
    </div>
  );
}

function MovementRow({
  label,
  count,
  href,
  nextAt,
  tone,
  onElapsed,
  clockSkewMs,
}: {
  label: string;
  count: number;
  href: string;
  nextAt?: string;
  tone: "danger" | "warning" | "success" | "info" | "violet";
  onElapsed?: () => void;
  clockSkewMs: number;
}) {
  const toneClasses = {
    danger: "border-red-200 bg-red-50 text-red-800",
    warning: "border-amber-200 bg-amber-50 text-amber-800",
    success: "border-green-200 bg-green-50 text-green-800",
    info: "border-blue-200 bg-blue-50 text-blue-800",
    violet: "border-violet-200 bg-violet-50 text-violet-800",
  }[tone];
  return (
    <div class="border-b border-stone-100 pb-1.5">
      <Link
        to={href}
        class={`flex items-center justify-between gap-2 rounded-md border px-2 py-1.5 font-semibold hover:underline ${toneClasses}`}
      >
        <span title={label} aria-label={label}>
          {count}
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
  const remaining = useCountdown(secondsUntilIso(nextAt, { clockSkewMs }), onElapsed);
  const urgencyClass = remaining <= 60 ? "text-amber-700" : "text-gray-500";
  return (
    <span class={`font-mono text-[11px] ${urgencyClass}`}>
      {formatDurationHms(remaining)}
    </span>
  );
}
