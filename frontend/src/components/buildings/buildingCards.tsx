import { useState } from "preact/hooks";
import { buildingDescriptionParagraphs, buildingLabel, unitLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { BuildingSprite } from "@/components/BuildingSprite";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { Badge, Button, Panel, SectionHeader } from "@/components/ui";
import {
  useAddBuildingMutation,
  useResearchAcademyMutation,
  useResearchSmithyMutation,
  useTrainUnitsMutation,
  useUpgradeBuildingMutation,
} from "@/query/mutations";
import type {
  AcademyResearchOption,
  BuildingPageResponse,
  EmptySlotBuildOption,
  ResourceAmounts,
  SmithyUpgradeOption,
  TrainingUnitOption,
} from "@/types/api";

function canAfford(stored: ResourceAmounts, cost: ResourceAmounts) {
  return (
    stored.lumber >= cost.lumber &&
    stored.clay >= cost.clay &&
    stored.iron >= cost.iron &&
    stored.crop >= cost.crop
  );
}

function ResourceCost({ cost }: { cost: ResourceAmounts }) {
  return (
    <div class="flex gap-3 text-sm">
      <span class="text-gray-700 inline-flex items-center gap-1"><ResourceSprite kind="lumber" size={14} label="Lumber" />{cost.lumber}</span>
      <span class="text-gray-700 inline-flex items-center gap-1"><ResourceSprite kind="clay" size={14} label="Clay" />{cost.clay}</span>
      <span class="text-gray-700 inline-flex items-center gap-1"><ResourceSprite kind="iron" size={14} label="Iron" />{cost.iron}</span>
      <span class="text-gray-700 inline-flex items-center gap-1"><ResourceSprite kind="crop" size={14} label="Crop" />{cost.crop}</span>
    </div>
  );
}

type BuildingUpgradeValue = {
  label: string;
  value: string;
};

const RESOURCE_PRODUCTION_BUILDINGS = new Set(["Woodcutter", "ClayPit", "IronMine", "Cropland"]);
const RESOURCE_BONUS_BUILDINGS = new Set(["Sawmill", "Brickyard", "IronFoundry", "GrainMill", "Bakery"]);
const STORAGE_BUILDINGS = new Set(["Warehouse", "Granary", "GreatWarehouse", "GreatGranary"]);
const TRAINING_SPEED_BUILDINGS = new Set([
  "Barracks",
  "GreatBarracks",
  "Stable",
  "GreatStable",
  "Workshop",
  "GreatWorkshop",
]);
const WALL_BUILDINGS = new Set(["CityWall", "EarthWall", "Palisade"]);
const RESERVED_BUILDING_SLOTS = new Set([19, 39, 40]);

function decimalPercent(value: number) {
  const percent = value / 10;
  return `${Number.isInteger(percent) ? percent.toFixed(0) : percent.toFixed(1)}%`;
}

function buildingUpgradeValue(buildingName: string, value?: number): BuildingUpgradeValue | null {
  if (value == null || value === 0) return null;

  if (RESOURCE_PRODUCTION_BUILDINGS.has(buildingName)) {
    return { label: "Production", value: `${value.toLocaleString()}/hour` };
  }
  if (RESOURCE_BONUS_BUILDINGS.has(buildingName)) {
    return { label: "Production bonus", value: `+${value}%` };
  }
  if (STORAGE_BUILDINGS.has(buildingName)) {
    return { label: "Storage capacity", value: value.toLocaleString() };
  }
  if (buildingName === "MainBuilding") {
    return { label: "Construction time", value: decimalPercent(value) };
  }
  if (TRAINING_SPEED_BUILDINGS.has(buildingName)) {
    return { label: "Training time", value: decimalPercent(value) };
  }
  if (buildingName === "Marketplace") {
    return { label: "Merchants", value: value.toLocaleString() };
  }
  if (buildingName === "Cranny") {
    return { label: "Hidden resources", value: value.toLocaleString() };
  }
  if (buildingName === "Smithy") {
    return { label: "Unit upgrade cap", value: `level ${Math.max(0, value - 100)}` };
  }
  if (buildingName === "Embassy") {
    return { label: "Alliance capacity", value: `${value.toLocaleString()} members` };
  }
  if (buildingName === "TownHall") {
    return { label: "Culture points", value: `${value.toLocaleString()}/day` };
  }
  if (buildingName === "TradeOffice") {
    return { label: "Merchant capacity bonus", value: `+${value}%` };
  }
  if (WALL_BUILDINGS.has(buildingName)) {
    return { label: "Defense bonus", value: `+${value}%` };
  }
  if (buildingName === "StonemansionLodge") {
    return { label: "Building stability", value: `${decimalPercent(value)} durability` };
  }
  if (buildingName === "TournamentSquare") {
    return { label: "Long-distance troop speed", value: `+${value}%` };
  }
  if (buildingName === "Treasury") {
    return { label: "Artifact capacity", value: value.toLocaleString() };
  }
  if (buildingName === "Brewery") {
    return { label: "Attack bonus", value: `+${value}%` };
  }
  if (buildingName === "Trapper") {
    return { label: "Trap capacity", value: value.toLocaleString() };
  }
  if (buildingName === "HorseDrinkingTrough") {
    return { label: "Cavalry training time", value: `-${value}%` };
  }

  return null;
}

export function UpgradeBuilding({
  data,
  onMutate,
}: {
  data: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upgradeBuilding = useUpgradeBuildingMutation();
  const affordable = canAfford(data.storedResources, data.cost);
  const canUpgrade = affordable && !data.queueFull && !data.atMaxLevel && !submitting;
  const currentUpgradeValue = buildingUpgradeValue(data.buildingName, data.currentValue);
  const nextUpgradeValue = buildingUpgradeValue(data.buildingName, data.nextValue);

  return (
    <Panel>
      <h3 class="text-lg font-bold text-gray-800 mb-3">
        {data.atMaxLevel ? "Max level reached" : `Upgrade to level ${data.nextLevel}`}
      </h3>

      {!data.atMaxLevel ? (
        <>
          <div class="mb-3 flex flex-wrap items-center gap-3 text-sm text-gray-700">
            <ResourceCost cost={data.cost} />
            <span class="inline-flex items-center gap-1">
              <ResourceSprite kind="clock" size={14} label="Duration" />
              <span class="font-semibold">{formatDurationHms(data.timeSecs)}</span>
            </span>
            <span class="inline-flex items-center gap-1">
              <ResourceSprite kind="upkeep" size={14} label="Upkeep" />
              <span class="font-semibold">{data.nextUpkeep}</span>
            </span>
          </div>

          {nextUpgradeValue ? (
            <div class="text-sm mb-3 p-2 bg-blue-50 border border-blue-200 rounded space-y-1">
              {currentUpgradeValue ? (
                <div>
                  <span class="text-gray-600">Current {currentUpgradeValue.label.toLowerCase()}: </span>
                  <span class="font-semibold text-blue-700">{currentUpgradeValue.value}</span>
                </div>
              ) : null}
              <div>
                <span class="text-gray-600">Next {nextUpgradeValue.label.toLowerCase()}: </span>
                <span class="font-semibold text-blue-700">{nextUpgradeValue.value}</span>
              </div>
            </div>
          ) : null}

          {data.queueFull ? <p class="text-sm text-yellow-600 mb-2">Queue is full</p> : null}
          {!affordable ? <p class="text-sm text-red-600 mb-2">Insufficient resources</p> : null}
          {error ? <p class="text-sm text-red-600 mb-2">{error}</p> : null}

          <Button
            type="button"
            class="w-full"
            disabled={!canUpgrade}
            onClick={async () => {
              setSubmitting(true);
              setError(null);
              try {
                await upgradeBuilding.mutateAsync({ slotId: data.slotId });
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(false);
              }
            }}
          >
            Upgrade
          </Button>
        </>
      ) : (
        <div class="space-y-3">
          <p class="text-sm text-gray-600">{buildingLabel(data.buildingName)} is at maximum level ({data.currentLevel}).</p>
          {currentUpgradeValue ? (
            <div class="text-sm p-2 bg-blue-50 border border-blue-200 rounded">
              <span class="text-gray-600">{currentUpgradeValue.label}: </span>
              <span class="font-semibold text-blue-700">{currentUpgradeValue.value}</span>
            </div>
          ) : null}
        </div>
      )}
    </Panel>
  );
}

export function EmptySlotBuilding({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const addBuilding = useAddBuildingMutation();
  if (!detail.emptySlot) return null;
  if (detail.emptySlot.hasQueueForSlot && detail.emptySlot.queuedBuildingName) return null;
  const reservedSlotOption = RESERVED_BUILDING_SLOTS.has(detail.slotId)
    ? detail.emptySlot.buildableBuildings[0] ?? detail.emptySlot.lockedBuildings[0] ?? null
    : null;

  const renderOption = (option: EmptySlotBuildOption, locked: boolean) => {
    const affordable = canAfford(detail.storedResources, option.cost);
    const canBuild = !locked && !detail.queueFull && affordable;
    const isSubmitting = submitting === option.buildingName;
    const descriptionParagraphs = buildingDescriptionParagraphs(option.buildingName);
    return (
      <Panel key={option.buildingName} class="space-y-3">
        <div class="flex items-start justify-between gap-2">
          <div class="inline-flex items-center gap-2 text-lg font-semibold text-gray-900">
            <BuildingSprite buildingName={option.buildingName} size={28} label={buildingLabel(option.buildingName)} />
            {buildingLabel(option.buildingName)}
          </div>
          {locked ? <Badge variant="warning">Locked</Badge> : null}
        </div>
        {descriptionParagraphs.length > 0 ? (
          <div class="space-y-1 text-xs leading-5 text-gray-600">
            {descriptionParagraphs.slice(0, 2).map((paragraph) => (
              <p key={paragraph}>{paragraph}</p>
            ))}
          </div>
        ) : null}
        <div class="flex flex-wrap items-center gap-3 text-sm text-gray-600">
          <ResourceCost cost={option.cost} />
          <span class="inline-flex items-center gap-1">
            <ResourceSprite kind="clock" size={14} label="Build time" />
            {formatDurationHms(option.timeSecs)}
          </span>
        </div>
        {option.missingRequirements.length > 0 ? (
          <div class="text-xs text-amber-700">
            Missing:{" "}
            {option.missingRequirements.map((req) => `${buildingLabel(req.buildingName)} ${req.requiredLevel}`).join(", ")}
          </div>
        ) : null}
        {!affordable ? <div class="text-xs text-red-600">Not enough resources</div> : null}
        {detail.queueFull ? <div class="text-xs text-amber-700">Construction queue is full</div> : null}
        {!locked ? (
          <Button
            type="button"
            disabled={!canBuild || isSubmitting}
            onClick={async () => {
              setSubmitting(option.buildingName);
              setError(null);
              try {
                await addBuilding.mutateAsync({
                  slotId: detail.slotId,
                  buildingName: option.buildingName,
                });
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(null);
              }
            }}
          >
            Build
          </Button>
        ) : null}
      </Panel>
    );
  };

  if (reservedSlotOption) {
    const locked = detail.emptySlot.lockedBuildings.some(
      (option) => option.buildingName === reservedSlotOption.buildingName,
    );
    return (
      <div class="space-y-3">
        <div class="text-sm text-gray-600">
          This slot is reserved for {buildingLabel(reservedSlotOption.buildingName)}.
        </div>
        {renderOption(reservedSlotOption, locked)}
        {error ? <div class="text-sm text-red-600">{error}</div> : null}
      </div>
    );
  }

  return (
    <div class="space-y-4">
      {detail.emptySlot.buildableBuildings.length > 0 ? (
        <div class="space-y-3">
          <SectionHeader title="Available now" />
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {detail.emptySlot.buildableBuildings.map((option) => renderOption(option, false))}
          </div>
        </div>
      ) : null}

      {detail.emptySlot.lockedBuildings.length > 0 ? (
        <div class="space-y-3">
          <SectionHeader title="Locked" />
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {detail.emptySlot.lockedBuildings.map((option) => renderOption(option, true))}
          </div>
        </div>
      ) : null}

      {detail.emptySlot.buildableBuildings.length === 0 && detail.emptySlot.lockedBuildings.length === 0 ? (
        <p class="text-sm text-gray-500">No buildings available for this slot.</p>
      ) : null}
      {error ? <div class="text-sm text-red-600">{error}</div> : null}
    </div>
  );
}

export function QueuedConstructionUpgrade({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upgradeBuilding = useUpgradeBuildingMutation();
  const queuedPreview = detail.emptySlot?.queuedUpgradePreview;
  const canUpgrade = Boolean(
    detail.emptySlot?.queuedCanUpgrade && queuedPreview && !detail.queueFull && !submitting,
  );

  if (!detail.emptySlot?.hasQueueForSlot || !detail.emptySlot.queuedBuildingName || !queuedPreview) {
    return null;
  }

  const affordable = canAfford(detail.storedResources, queuedPreview.cost);
  const currentUpgradeValue = buildingUpgradeValue(queuedPreview.buildingName, queuedPreview.currentValue);
  const nextUpgradeValue = buildingUpgradeValue(queuedPreview.buildingName, queuedPreview.nextValue);

  return (
    <Panel>
      <h3 class="text-lg font-bold text-gray-800 mb-3">
        {queuedPreview.atMaxLevel ? "Max level reached" : `Upgrade to level ${queuedPreview.nextLevel}`}
      </h3>

      {!queuedPreview.atMaxLevel ? (
        <>
          <div class="mb-3 flex flex-wrap items-center gap-3 text-sm text-gray-700">
            <ResourceCost cost={queuedPreview.cost} />
            <span class="inline-flex items-center gap-1">
              <ResourceSprite kind="clock" size={14} label="Duration" />
              <span class="font-semibold">{formatDurationHms(queuedPreview.timeSecs)}</span>
            </span>
            <span class="inline-flex items-center gap-1">
              <ResourceSprite kind="upkeep" size={14} label="Upkeep" />
              <span class="font-semibold">{queuedPreview.nextUpkeep}</span>
            </span>
          </div>

          {nextUpgradeValue ? (
            <div class="text-sm mb-3 p-2 bg-blue-50 border border-blue-200 rounded space-y-1">
              {currentUpgradeValue ? (
                <div>
                  <span class="text-gray-600">Current {currentUpgradeValue.label.toLowerCase()}: </span>
                  <span class="font-semibold text-blue-700">{currentUpgradeValue.value}</span>
                </div>
              ) : null}
              <div>
                <span class="text-gray-600">Next {nextUpgradeValue.label.toLowerCase()}: </span>
                <span class="font-semibold text-blue-700">{nextUpgradeValue.value}</span>
              </div>
            </div>
          ) : null}

          {detail.queueFull ? <p class="text-sm text-yellow-600 mb-2">Queue is full</p> : null}
          {!affordable ? <p class="text-sm text-red-600 mb-2">Insufficient resources</p> : null}
          {error ? <p class="text-sm text-red-600 mb-2">{error}</p> : null}

          <Button
            type="button"
            class="w-full"
            disabled={!canUpgrade || !affordable}
            onClick={async () => {
              setSubmitting(true);
              setError(null);
              try {
                await upgradeBuilding.mutateAsync({ slotId: detail.slotId });
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(false);
              }
            }}
          >
            Upgrade
          </Button>
        </>
      ) : (
        <div class="space-y-3">
          <p class="text-sm text-gray-600">
            {buildingLabel(detail.emptySlot.queuedBuildingName)} is at maximum level ({queuedPreview.currentLevel}).
          </p>
          {currentUpgradeValue ? (
            <div class="text-sm p-2 bg-blue-50 border border-blue-200 rounded">
              <span class="text-gray-600">{currentUpgradeValue.label}: </span>
              <span class="font-semibold text-blue-700">{currentUpgradeValue.value}</span>
            </div>
          ) : null}
        </div>
      )}
    </Panel>
  );
}

export function TrainingUnitCard({
  option,
  detail,
  onMutate,
}: {
  option: TrainingUnitOption;
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [quantity, setQuantity] = useState(1);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const trainUnits = useTrainUnitsMutation();
  const maxTrainable = option.maxTrainable;
  const clampQuantity = (value: number) => {
    const normalized = Math.max(1, Number.isFinite(value) ? value : 1);
    return maxTrainable == null ? normalized : Math.min(maxTrainable, normalized);
  };
  return (
    <Panel class="space-y-3">
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
        <div>
          <div class="text-lg font-semibold text-gray-900">{unitLabel(option.name)}</div>
        </div>
        <div class="flex items-end gap-2">
          <label class="text-sm text-gray-600 min-w-0">
            <input
              type="number"
              min="1"
              max={maxTrainable}
              value={quantity}
              onInput={(event) => setQuantity(clampQuantity(Number((event.target as HTMLInputElement).value || "1")))}
              class="w-20 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          <Button
            type="button"
            size="sm"
            disabled={submitting || (maxTrainable != null && maxTrainable < 1)}
            onClick={async () => {
              setSubmitting(true);
              setError(null);
              try {
                await trainUnits.mutateAsync({
                  slotId: detail.slotId,
                  unitIdx: option.unitIdx,
                  quantity: clampQuantity(quantity),
                  buildingName: detail.buildingName,
                });
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(false);
              }
            }}
          >
            Train
          </Button>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2 text-sm">
        <div class="inline-flex items-center gap-1"><ResourceSprite kind="lumber" size={14} label="Lumber" /><span class="font-semibold">{option.cost.lumber}</span></div>
        <div class="inline-flex items-center gap-1"><ResourceSprite kind="clay" size={14} label="Clay" /><span class="font-semibold">{option.cost.clay}</span></div>
        <div class="inline-flex items-center gap-1"><ResourceSprite kind="iron" size={14} label="Iron" /><span class="font-semibold">{option.cost.iron}</span></div>
        <div class="inline-flex items-center gap-1"><ResourceSprite kind="crop" size={14} label="Crop" /><span class="font-semibold">{option.cost.crop}</span></div>
        <span class="inline-flex items-center gap-1 text-xs text-gray-500"><ResourceSprite kind="clock" size={14} label="Training time" />{formatDurationHms(option.timeSecs)}</span>
        <span class="inline-flex items-center gap-1 text-xs text-gray-500"><ResourceSprite kind="upkeep" size={14} label="Upkeep" />{option.upkeep}</span>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-2 text-sm">
        <div class="rounded border bg-gray-50 px-2 py-1"><div class="text-[11px] uppercase text-gray-500">Attack</div><div class="font-semibold text-gray-900">{option.attack}</div></div>
        <div class="rounded border bg-gray-50 px-2 py-1"><div class="text-[11px] uppercase text-gray-500">Def. Infantry</div><div class="font-semibold text-gray-900">{option.defenseInfantry}</div></div>
        <div class="rounded border bg-gray-50 px-2 py-1"><div class="text-[11px] uppercase text-gray-500">Def. Cavalry</div><div class="font-semibold text-gray-900">{option.defenseCavalry}</div></div>
        <div class="rounded border bg-gray-50 px-2 py-1"><div class="text-[11px] uppercase text-gray-500">Speed</div><div class="font-semibold text-gray-900">{option.speed}</div></div>
        <div class="rounded border bg-gray-50 px-2 py-1"><div class="text-[11px] uppercase text-gray-500">Capacity</div><div class="font-semibold text-gray-900">{option.capacity}</div></div>
      </div>
      {error ? <div class="text-xs text-red-600">{error}</div> : null}
    </Panel>
  );
}

export function AcademyOptionCard({
  option,
  detail,
  onMutate,
}: {
  option: AcademyResearchOption;
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const researchAcademy = useResearchAcademyMutation();
  const affordable = canAfford(detail.storedResources, option.cost);
  const queueFull = detail.academy?.queueFull ?? false;
  const blockedByRequirements = option.missingRequirements.length > 0;
  const canResearch = affordable && !queueFull && !submitting && !blockedByRequirements;
  return (
    <Panel class="space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div class="inline-flex items-center gap-2 text-lg font-semibold text-gray-900">
          <UnitSpriteByName unitName={option.unitName} label={unitLabel(option.unitName)} />
          {unitLabel(option.unitName)}
        </div>
      </div>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="inline-flex items-center gap-3 text-sm text-gray-600">
          <ResourceCost cost={option.cost} />
          <span class="inline-flex items-center gap-1"><ResourceSprite kind="clock" size={14} label="Research time" />{formatDurationHms(option.timeSecs)}</span>
        </div>
        <Button
          type="button"
          size="sm"
          disabled={!canResearch}
          onClick={async () => {
            setSubmitting(true);
            setError(null);
            try {
              await researchAcademy.mutateAsync({
                slotId: detail.slotId,
                unitName: option.unitName,
              });
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setSubmitting(false);
            }
          }}
        >
          Research
        </Button>
      </div>
      {!affordable ? <div class="text-xs text-red-600">Not enough resources</div> : null}
      {queueFull ? <div class="text-xs text-amber-700">Research queue is full</div> : null}
      {blockedByRequirements ? (
        <div class="text-xs text-amber-700">
          Requires:{" "}
          {option.missingRequirements
            .map((req) => `${buildingLabel(req.buildingName)} ${req.requiredLevel}`)
            .join(", ")}
        </div>
      ) : null}
      {error ? <div class="text-xs text-red-600">{error}</div> : null}
    </Panel>
  );
}

export function SmithyOptionCard({
  option,
  detail,
  onMutate,
}: {
  option: SmithyUpgradeOption;
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const researchSmithy = useResearchSmithyMutation();
  const affordable = canAfford(detail.storedResources, option.cost);
  const queueFull = detail.smithy?.queueFull ?? false;
  const canUpgrade = option.canUpgrade && affordable && !queueFull && !submitting;
  const nextLevel = Math.min(option.nextLevel, option.maxLevel);
  const reachedMaxLevel = option.currentLevel >= option.maxLevel;
  const alreadyQueued = detail.smithy?.queue.some((job) => job.unitName === option.unitName) ?? false;
  return (
    <Panel class="space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="inline-flex items-center gap-2 text-lg font-semibold text-gray-900">
            <UnitSpriteByName unitName={option.unitName} label={unitLabel(option.unitName)} />
            {unitLabel(option.unitName)}
          </div>
          <div class="text-xs text-gray-500">Level {option.currentLevel} → {nextLevel} (Max: {option.maxLevel})</div>
        </div>
      </div>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="inline-flex items-center gap-3 text-sm text-gray-600">
          <ResourceCost cost={option.cost} />
          <span class="inline-flex items-center gap-1"><ResourceSprite kind="clock" size={14} label="Upgrade time" />{formatDurationHms(option.timeSecs)}</span>
        </div>
        <Button
          type="button"
          size="sm"
          disabled={!canUpgrade}
          onClick={async () => {
            setSubmitting(true);
            setError(null);
            try {
              await researchSmithy.mutateAsync({
                slotId: detail.slotId,
                unitName: option.unitName,
              });
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setSubmitting(false);
            }
          }}
        >
          Upgrade
        </Button>
      </div>
      {!affordable ? <div class="text-xs text-red-600">Not enough resources</div> : null}
      {queueFull ? <div class="text-xs text-amber-700">Upgrade queue is full</div> : null}
      {!queueFull && alreadyQueued ? <div class="text-xs text-amber-700">Already in upgrade queue</div> : null}
      {reachedMaxLevel ? <div class="text-xs text-gray-500">Max level reached</div> : null}
      {error ? <div class="text-xs text-red-600">{error}</div> : null}
    </Panel>
  );
}
