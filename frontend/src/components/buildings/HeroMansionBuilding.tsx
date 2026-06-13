import { useState } from "preact/hooks";
import { useCurrentHeroQuery } from "@/query/hooks";
import {
  useAssignHeroPointsMutation,
  useResetHeroPointsMutation,
  useSetHeroResourceFocusMutation,
} from "@/query/mutations";
import type { BuildingPageResponse, Hero } from "@/types/api";

const FOCUSES: Hero["resourceFocus"][] = ["Balanced", "Wood", "Clay", "Iron", "Crop"];

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
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  if (detail.buildingName !== "HeroMansion") return null;

  const heroQuery = useCurrentHeroQuery();
  const assignPoints = useAssignHeroPointsMutation();
  const resetPoints = useResetHeroPointsMutation();
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
    setDraft((current) => ({ ...current, [key]: value }));
  };

  return (
    <div class="space-y-4">
      <div class="grid gap-3 md:grid-cols-4">
        <div class="border rounded-md p-4 bg-white">
          <div class="text-xs uppercase text-gray-500">Level</div>
          <div class="mt-1 text-2xl font-semibold">{hero.level}</div>
        </div>
        <div class="border rounded-md p-4 bg-white">
          <div class="text-xs uppercase text-gray-500">Health</div>
          <div class="mt-1 text-2xl font-semibold">{hero.health}%</div>
        </div>
        <div class="border rounded-md p-4 bg-white">
          <div class="text-xs uppercase text-gray-500">Experience</div>
          <div class="mt-1 text-2xl font-semibold">{hero.experience}/{hero.xpForNextLevel}</div>
        </div>
        <div class="border rounded-md p-4 bg-white">
          <div class="text-xs uppercase text-gray-500">Speed</div>
          <div class="mt-1 text-2xl font-semibold">{hero.speed}</div>
        </div>
      </div>

      <div class="border rounded-md p-4 bg-gray-50">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-semibold text-gray-900">Attributes</div>
            <div class="text-sm text-gray-600">Unassigned points: {hero.unassignedPoints}</div>
          </div>
          <button
            class="rounded-md border border-gray-300 bg-white px-3 py-2 text-sm disabled:opacity-50"
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

        <div class="mt-4 grid gap-3 md:grid-cols-5">
          <div class="text-sm"><span class="text-gray-500">Strength</span><div class="font-semibold">{hero.strengthPoints}</div></div>
          <div class="text-sm"><span class="text-gray-500">Off bonus</span><div class="font-semibold">{hero.offBonusPoints}</div></div>
          <div class="text-sm"><span class="text-gray-500">Def bonus</span><div class="font-semibold">{hero.defBonusPoints}</div></div>
          <div class="text-sm"><span class="text-gray-500">Regeneration</span><div class="font-semibold">{hero.regenerationPoints}</div></div>
          <div class="text-sm"><span class="text-gray-500">Resources</span><div class="font-semibold">{hero.resourcesPoints}</div></div>
        </div>

        <div class="mt-4 grid gap-3 md:grid-cols-5">
          <PointInput label="Strength" value={draft.strength} max={hero.unassignedPoints} onInput={(value) => updateDraft("strength", value)} />
          <PointInput label="Off bonus" value={draft.offBonus} max={hero.unassignedPoints} onInput={(value) => updateDraft("offBonus", value)} />
          <PointInput label="Def bonus" value={draft.defBonus} max={hero.unassignedPoints} onInput={(value) => updateDraft("defBonus", value)} />
          <PointInput label="Regeneration" value={draft.regeneration} max={hero.unassignedPoints} onInput={(value) => updateDraft("regeneration", value)} />
          <PointInput label="Resources" value={draft.resources} max={hero.unassignedPoints} onInput={(value) => updateDraft("resources", value)} />
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

      <div class="border rounded-md p-4 bg-white">
        <label class="block text-sm">
          <span class="text-gray-600">Resource focus</span>
          <select
            class="mt-1 w-full max-w-xs rounded-md border border-gray-300 px-3 py-2 text-sm"
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
