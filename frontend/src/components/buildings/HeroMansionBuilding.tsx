import { useState } from "preact/hooks";
import { useCurrentHeroQuery } from "@/query/hooks";
import {
  useAssignHeroPointsMutation,
  useResetHeroPointsMutation,
  useReviveHeroMutation,
  useSetHeroResourceFocusMutation,
} from "@/query/mutations";
import { useServerDeadlineCountdown } from "@/live/useCountdown";
import { formatDurationHms } from "@/lib/time";
import type { BuildingPageResponse, Hero } from "@/types/api";

const FOCUSES: Hero["resourceFocus"][] = ["Balanced", "Wood", "Clay", "Iron", "Crop"];
const HERO_IMAGES: Record<string, string> = {
  Roman: "/static/heroes/hero_roman.png",
  Teuton: "/static/heroes/hero_teuton.png",
  Gaul: "/static/heroes/hero_gaul.png",
};
const BASE_REGENERATION_PERCENT = 10;

type PointDraft = {
  strength: number;
  offBonus: number;
  defBonus: number;
  regeneration: number;
  resources: number;
};

function emptyDraft(): PointDraft {
  return { strength: 0, offBonus: 0, defBonus: 0, regeneration: 0, resources: 0 };
}

function formatNumber(value: number) {
  return Math.round(value).toLocaleString();
}

function formatPercent(value: number) {
  const rounded = Math.round(value * 10) / 10;
  return `${Number.isInteger(rounded) ? rounded.toFixed(0) : rounded.toFixed(1)}%`;
}

function resourceProduction(points: number, focus: Hero["resourceFocus"]) {
  if (focus === "Balanced") {
    const each = points * 3;
    return { lumber: each, clay: each, iron: each, crop: each };
  }
  return {
    lumber: focus === "Wood" ? points * 10 : 0,
    clay: focus === "Clay" ? points * 10 : 0,
    iron: focus === "Iron" ? points * 10 : 0,
    crop: focus === "Crop" ? points * 10 : 0,
  };
}

function formatResourceProduction(resources: Hero["resourceProduction"]) {
  if (
    resources.lumber === resources.clay &&
    resources.lumber === resources.iron &&
    resources.lumber === resources.crop
  ) {
    return `+${formatNumber(resources.lumber)} each / hour`;
  }
  const parts = [
    ["wood", resources.lumber],
    ["clay", resources.clay],
    ["iron", resources.iron],
    ["crop", resources.crop],
  ].filter(([, value]) => Number(value) > 0);
  if (parts.length === 0) return "+0 / hour";
  return parts.map(([label, value]) => `+${formatNumber(Number(value))} ${label}`).join(", ") + " / hour";
}

function formatResourceCost(resources: Hero["resourceProduction"]) {
  const parts = [
    ["wood", resources.lumber],
    ["clay", resources.clay],
    ["iron", resources.iron],
    ["crop", resources.crop],
  ].filter(([, value]) => Number(value) > 0);
  if (parts.length === 0) return "0";
  return parts.map(([label, value]) => `${formatNumber(Number(value))} ${label}`).join(", ");
}

function PointInput({
  label,
  value,
  max,
  onInput,
}: {
  label: string;
  value: number;
  max: number;
  onInput: (value: number) => void;
}) {
  return (
    <label class="block text-sm">
      <span class="text-gray-600">{label}</span>
      <input
        class="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
        type="number"
        min={0}
        max={max}
        value={value}
        onInput={(event) => onInput(Math.max(0, Number((event.currentTarget as HTMLInputElement).value) || 0))}
      />
    </label>
  );
}

export function HeroMansionBuilding({
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
  if (detail.buildingName !== "HeroMansion") return null;

  const heroQuery = useCurrentHeroQuery();
  const assignPoints = useAssignHeroPointsMutation();
  const resetPoints = useResetHeroPointsMutation();
  const reviveHero = useReviveHeroMutation();
  const setFocus = useSetHeroResourceFocusMutation();
  const [draft, setDraft] = useState<PointDraft>(emptyDraft());

  const hero = heroQuery.data;
  const totalDraft = draft.strength + draft.offBonus + draft.defBonus + draft.regeneration + draft.resources;
  const canAssign = !!hero && totalDraft > 0 && totalDraft <= hero.unassignedPoints && !assignPoints.isPending;

  if (heroQuery.isLoading) {
    return <div class="border rounded-md p-4 bg-gray-50 text-sm text-gray-600">Loading hero...</div>;
  }

  if (!hero) {
    return <div class="border rounded-md p-4 bg-gray-50 text-sm text-gray-600">No hero available.</div>;
  }

  const updateDraft = (key: keyof PointDraft, value: number) => {
    if (!hero) return;
    const currentPoints = {
      strength: hero.strengthPoints,
      offBonus: hero.offBonusPoints,
      defBonus: hero.defBonusPoints,
      regeneration: hero.regenerationPoints,
      resources: hero.resourcesPoints,
    }[key];
    const max = Math.min(hero.unassignedPoints, Math.max(0, 100 - currentPoints));
    setDraft((current) => ({ ...current, [key]: Math.min(max, value) }));
  };

  const pointCap = (key: keyof PointDraft) => {
    const currentPoints = {
      strength: hero.strengthPoints,
      offBonus: hero.offBonusPoints,
      defBonus: hero.defBonusPoints,
      regeneration: hero.regenerationPoints,
      resources: hero.resourcesPoints,
    }[key];
    return Math.min(hero.unassignedPoints, Math.max(0, 100 - currentPoints));
  };
  const projected = {
    strengthPoints: hero.strengthPoints + draft.strength,
    offBonusPoints: hero.offBonusPoints + draft.offBonus,
    defBonusPoints: hero.defBonusPoints + draft.defBonus,
    regenerationPoints: hero.regenerationPoints + draft.regeneration,
    resourcesPoints: hero.resourcesPoints + draft.resources,
  };
  const projectedStrength = projected.strengthPoints * hero.strengthPerPoint;
  const projectedOffBonus = projected.offBonusPoints * hero.offBonusPercentPerPoint;
  const projectedDefBonus = projected.defBonusPoints * hero.defBonusPercentPerPoint;
  const projectedRegeneration =
    BASE_REGENERATION_PERCENT + projected.regenerationPoints * hero.regenerationPercentPerPoint;
  const projectedResources = resourceProduction(projected.resourcesPoints, hero.resourceFocus);
  const portrait = HERO_IMAGES[hero.tribe] ?? HERO_IMAGES.Roman;
  const isDead = hero.health === 0;
  const revivalRemaining = useServerDeadlineCountdown(
    hero.revivalFinishesAt ?? "1970-01-01T00:00:00Z",
    serverTime,
    serverTimeObservedAtMs,
    hero.revivalFinishesAt ? onMutate : undefined,
  );
  const revivalPending = !!hero.revivalFinishesAt && revivalRemaining > 0;
  const attributeRows = [
    {
      label: "Strength",
      points: hero.strengthPoints,
      draft: draft.strength,
      current: formatNumber(hero.strengthValue),
      projected: formatNumber(projectedStrength),
      note: `+${formatNumber(hero.strengthPerPoint)} attack and defense per point`,
    },
    {
      label: "Off bonus",
      points: hero.offBonusPoints,
      draft: draft.offBonus,
      current: formatPercent(hero.offBonusPercent),
      projected: formatPercent(projectedOffBonus),
      note: `+${formatPercent(hero.offBonusPercentPerPoint)} attack per point`,
    },
    {
      label: "Def bonus",
      points: hero.defBonusPoints,
      draft: draft.defBonus,
      current: formatPercent(hero.defBonusPercent),
      projected: formatPercent(projectedDefBonus),
      note: `+${formatPercent(hero.defBonusPercentPerPoint)} defense per point`,
    },
    {
      label: "Regeneration",
      points: hero.regenerationPoints,
      draft: draft.regeneration,
      current: `${formatPercent(hero.regenerationPercentPerDay)} / day`,
      projected: `${formatPercent(projectedRegeneration)} / day`,
      note: `+${formatPercent(hero.regenerationPercentPerPoint)} / day per point`,
    },
    {
      label: "Resources",
      points: hero.resourcesPoints,
      draft: draft.resources,
      current: formatResourceProduction(hero.resourceProduction),
      projected: formatResourceProduction(projectedResources),
      note: hero.resourceFocus === "Balanced" ? "+3 each per point" : "+10 focused resource per point",
    },
  ];

  return (
    <div class="space-y-4">
      <div class="grid gap-4 rounded-md border border-stone-200 bg-white p-4 md:grid-cols-[160px_1fr]">
        <div class="mx-auto w-full max-w-[160px]">
          <img
            src={portrait}
            alt={`${hero.tribe} hero`}
            width={600}
            height={910}
            class="h-auto w-full"
          />
        </div>
        <div class="space-y-4">
          <div>
            <div class="text-xs font-semibold uppercase text-stone-500">{hero.tribe} hero</div>
            <div class="mt-1 text-2xl font-semibold text-stone-900">Hero Mansion</div>
          </div>
          <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3">
              <div class="text-xs uppercase text-stone-500">Level</div>
              <div class="mt-1 text-2xl font-semibold text-stone-900">{hero.level}</div>
            </div>
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3">
              <div class="text-xs uppercase text-stone-500">Health</div>
              <div class="mt-1 text-2xl font-semibold text-stone-900">{hero.health}%</div>
            </div>
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3">
              <div class="text-xs uppercase text-stone-500">Experience</div>
              <div class="mt-1 text-2xl font-semibold text-stone-900">
                {formatNumber(hero.experience)}/{formatNumber(hero.xpForNextLevel)}
              </div>
            </div>
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3">
              <div class="text-xs uppercase text-stone-500">Speed</div>
              <div class="mt-1 text-2xl font-semibold text-stone-900">{hero.speed}</div>
            </div>
          </div>
          <div class="grid gap-3 sm:grid-cols-3">
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3 text-sm">
              <div class="text-xs uppercase text-stone-500">Strength</div>
              <div class="mt-1 font-semibold text-stone-900">{formatNumber(hero.strengthValue)}</div>
            </div>
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3 text-sm">
              <div class="text-xs uppercase text-stone-500">Unit bonuses</div>
              <div class="mt-1 font-semibold text-stone-900">
                {formatPercent(hero.offBonusPercent)} off / {formatPercent(hero.defBonusPercent)} def
              </div>
            </div>
            <div class="rounded-md border border-stone-200 bg-stone-50 p-3 text-sm">
              <div class="text-xs uppercase text-stone-500">Resources</div>
              <div class="mt-1 font-semibold text-stone-900">{formatResourceProduction(hero.resourceProduction)}</div>
            </div>
          </div>
        </div>
      </div>

      {isDead ? (
        <div class="rounded-md border border-red-200 bg-red-50 p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-red-900">Hero died</div>
              <div class="mt-1 text-sm text-red-800">
                {revivalPending ? (
                  <>Revival in progress: {formatDurationHms(revivalRemaining)} remaining.</>
                ) : (
                  <>
                    Revival cost: {formatResourceCost(hero.resurrectionCost)}. Time:{" "}
                    {formatDurationHms(hero.resurrectionTimeSecs)}.
                  </>
                )}
              </div>
            </div>
            {revivalPending ? (
              <div class="rounded-md border border-red-200 bg-white px-3 py-2 text-sm font-semibold text-red-900">
                Revival scheduled
              </div>
            ) : (
              <button
                class="rounded-md bg-red-800 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
                type="button"
                disabled={reviveHero.isPending}
                onClick={async () => {
                  await reviveHero.mutateAsync({ heroId: hero.id, villageId: detail.villageId, reset: false });
                  await onMutate();
                }}
              >
                {reviveHero.isPending ? "Scheduling..." : "Revive hero"}
              </button>
            )}
          </div>
        </div>
      ) : null}

      <div class="rounded-md border border-stone-200 bg-stone-50 p-4">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-semibold text-stone-900">Attributes</div>
            <div class="text-sm text-stone-600">Unassigned points: {hero.unassignedPoints}</div>
          </div>
          <button
            class="rounded-md border border-stone-300 bg-white px-3 py-2 text-sm disabled:opacity-50"
            type="button"
            disabled={hero.level > 0 || resetPoints.isPending}
            onClick={async () => {
              await resetPoints.mutateAsync({ heroId: hero.id, villageId: hero.villageId });
              setDraft(emptyDraft());
              await onMutate();
            }}
          >
            Reset points
          </button>
        </div>

        <div class="mt-4 grid gap-3 lg:grid-cols-5">
          {attributeRows.map((row) => (
            <div key={row.label} class="rounded-md border border-stone-200 bg-white p-3 text-sm">
              <div class="text-xs font-semibold uppercase text-stone-500">{row.label}</div>
              <div class="mt-1 text-lg font-semibold text-stone-900">{row.current}</div>
              <div class="mt-1 text-stone-600">{row.points} points</div>
              {row.draft > 0 ? (
                <div class="mt-2 rounded border border-green-200 bg-green-50 px-2 py-1 text-xs font-semibold text-green-800">
                  {row.points + row.draft} points: {row.projected}
                </div>
              ) : null}
              <div class="mt-2 text-xs text-stone-500">{row.note}</div>
            </div>
          ))}
        </div>

        <div class="mt-4 grid gap-3 md:grid-cols-5">
          <PointInput label="Strength" value={draft.strength} max={pointCap("strength")} onInput={(value) => updateDraft("strength", value)} />
          <PointInput label="Off bonus" value={draft.offBonus} max={pointCap("offBonus")} onInput={(value) => updateDraft("offBonus", value)} />
          <PointInput label="Def bonus" value={draft.defBonus} max={pointCap("defBonus")} onInput={(value) => updateDraft("defBonus", value)} />
          <PointInput label="Regeneration" value={draft.regeneration} max={pointCap("regeneration")} onInput={(value) => updateDraft("regeneration", value)} />
          <PointInput label="Resources" value={draft.resources} max={pointCap("resources")} onInput={(value) => updateDraft("resources", value)} />
        </div>

        <button
          class="mt-4 rounded-md bg-gray-900 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
          type="button"
          disabled={!canAssign}
          onClick={async () => {
            await assignPoints.mutateAsync({ heroId: hero.id, villageId: hero.villageId, ...draft });
            setDraft(emptyDraft());
            await onMutate();
          }}
        >
          Assign points
        </button>
      </div>

      <div class="rounded-md border border-stone-200 bg-white p-4">
        <label class="block text-sm">
          <span class="text-stone-600">Resource focus</span>
          <select
            class="mt-1 w-full max-w-xs rounded-md border border-stone-300 px-3 py-2 text-sm"
            value={hero.resourceFocus}
            disabled={setFocus.isPending}
            onChange={async (event) => {
              await setFocus.mutateAsync({
                heroId: hero.id,
                villageId: hero.villageId,
                focus: (event.currentTarget as HTMLSelectElement).value as Hero["resourceFocus"],
              });
              await onMutate();
            }}
          >
            {FOCUSES.map((focus) => (
              <option key={focus} value={focus}>{focus}</option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
}
