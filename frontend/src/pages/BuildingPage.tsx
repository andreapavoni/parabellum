import { useState } from "preact/hooks";
import { api } from "@/lib/api";
import { buildingLabel, unitLabel } from "@/lib/labels";
import { formatDurationHms } from "@/lib/time";
import { BuildingSprite } from "@/components/BuildingSprite";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { ExpansionBuilding } from "@/components/buildings/ExpansionBuilding";
import { TrainingBuilding } from "@/components/buildings/TrainingBuilding";
import { AcademyBuilding } from "@/components/buildings/AcademyBuilding";
import { SmithyBuilding } from "@/components/buildings/SmithyBuilding";
import { MarketplaceBuilding } from "@/components/buildings/MarketplaceBuilding";
import { RallyPointBuilding } from "@/components/buildings/RallyPointBuilding";
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

function UpgradeBuilding({
  data,
  onMutate,
}: {
  data: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const affordable = canAfford(data.storedResources, data.cost);
  const canUpgrade = affordable && !data.queueFull && !data.atMaxLevel && !submitting;

  return (
    <div class="border rounded-lg p-4 bg-white shadow-sm">
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

          {data.nextValue ? (
            <div class="text-sm mb-3 p-2 bg-blue-50 border border-blue-200 rounded">
              <span class="text-gray-600">Next value: </span>
              <span class="font-semibold text-blue-700">{data.nextValue}</span>
            </div>
          ) : null}

          {data.queueFull ? <p class="text-sm text-yellow-600 mb-2">⚠️ Queue is full</p> : null}
          {!affordable ? <p class="text-sm text-red-600 mb-2">❌ Insufficient resources</p> : null}
          {error ? <p class="text-sm text-red-600 mb-2">{error}</p> : null}

          <button
            type="button"
            class="w-full text-white font-semibold py-2 px-4 rounded"
            style={
              canUpgrade
                ? "background-color: #16a34a;"
                : "background-color: #9ca3af; cursor: not-allowed; opacity: 0.7;"
            }
            disabled={!canUpgrade}
            onClick={async () => {
              setSubmitting(true);
              setError(null);
              try {
                await api.upgradeBuilding({ slotId: data.slotId });
                await onMutate();
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(false);
              }
            }}
          >
            Upgrade
          </button>
        </>
      ) : (
        <p class="text-sm text-gray-600">{buildingLabel(data.buildingName)} is at maximum level ({data.currentLevel}).</p>
      )}
    </div>
  );
}

function EmptySlotBuilding({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  if (!detail.emptySlot) return null;

  const renderOption = (option: EmptySlotBuildOption, locked: boolean) => {
    const affordable = canAfford(detail.storedResources, option.cost);
    const canBuild = !locked && !detail.queueFull && affordable;
    const isSubmitting = submitting === option.buildingName;
    return (
      <div key={option.buildingName} class="border rounded-md p-4 bg-white space-y-3">
        <div class="flex items-start justify-between gap-2">
          <div class="inline-flex items-center gap-2 text-lg font-semibold text-gray-900">
            <BuildingSprite buildingName={option.buildingName} size={28} label={buildingLabel(option.buildingName)} />
            {buildingLabel(option.buildingName)}
          </div>
          {locked ? <span class="text-xs text-amber-700 font-semibold uppercase">Locked</span> : null}
        </div>
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
          <button
            type="button"
            class={
              canBuild
                ? "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
                : "bg-emerald-600 text-white font-semibold px-4 py-2 rounded opacity-60 cursor-not-allowed"
            }
            disabled={!canBuild || isSubmitting}
            onClick={async () => {
              setSubmitting(option.buildingName);
              setError(null);
              try {
                await api.addBuilding({
                  slotId: detail.slotId,
                  buildingName: option.buildingName,
                });
                await onMutate();
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(null);
              }
            }}
          >
            Build
          </button>
        ) : null}
      </div>
    );
  };

  return (
    <div class="space-y-4">
      {detail.emptySlot.buildableBuildings.length > 0 ? (
        <div class="space-y-3">
          <div class="text-sm text-gray-500 uppercase">Available now</div>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {detail.emptySlot.buildableBuildings.map((option) => renderOption(option, false))}
          </div>
        </div>
      ) : null}

      {detail.emptySlot.lockedBuildings.length > 0 ? (
        <div class="space-y-3">
          <div class="text-sm text-gray-500 uppercase">Locked</div>
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

function TrainingUnitCard({
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
  return (
    <div class="border rounded-md p-4 bg-white space-y-3">
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
        <div>
          <div class="text-lg font-semibold text-gray-900">{unitLabel(option.name)}</div>
        </div>
        <div class="flex items-end gap-2">
          <label class="text-sm text-gray-600 min-w-0">
            <input
              type="number"
              min="1"
              value={quantity}
              onInput={(event) => setQuantity(Math.max(1, Number((event.target as HTMLInputElement).value || "1")))}
              class="w-20 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          <button
            type="button"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-3 py-1.5 rounded"
            disabled={submitting}
            onClick={async () => {
              setSubmitting(true);
              setError(null);
              try {
                await api.trainUnits({
                  slotId: detail.slotId,
                  unitIdx: option.unitIdx,
                  quantity,
                  buildingName: detail.buildingName,
                });
                await onMutate();
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setSubmitting(false);
              }
            }}
          >
            Train
          </button>
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
    </div>
  );
}

function AcademyOptionCard({
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
  const affordable = canAfford(detail.storedResources, option.cost);
  const queueFull = detail.academy?.queueFull ?? false;
  const blockedByRequirements = option.missingRequirements.length > 0;
  const canResearch = affordable && !queueFull && !submitting && !blockedByRequirements;
  return (
    <div class="border rounded-md p-4 bg-white space-y-3">
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
        <button
          type="button"
          class={canResearch ? "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-3 py-1.5 rounded" : "bg-emerald-600 text-white font-semibold px-3 py-1.5 rounded opacity-60 cursor-not-allowed"}
          disabled={!canResearch}
          onClick={async () => {
            setSubmitting(true);
            setError(null);
            try {
              await api.researchAcademy({
                slotId: detail.slotId,
                unitName: option.unitName,
              });
              await onMutate();
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setSubmitting(false);
            }
          }}
        >
          Research
        </button>
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
    </div>
  );
}

function SmithyOptionCard({
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
  const affordable = canAfford(detail.storedResources, option.cost);
  const queueFull = detail.smithy?.queueFull ?? false;
  const canUpgrade = option.canUpgrade && affordable && !queueFull && !submitting;
  const nextLevel = Math.min(option.currentLevel + 1, option.maxLevel);
  const reachedMaxLevel = option.currentLevel >= option.maxLevel;
  const alreadyQueued = detail.smithy?.queue.some((job) => job.unitName === option.unitName) ?? false;
  return (
    <div class="border rounded-md p-4 bg-white space-y-3">
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
        <button
          type="button"
          class={canUpgrade ? "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-3 py-1.5 rounded" : "bg-blue-600 text-white font-semibold px-3 py-1.5 rounded opacity-60 cursor-not-allowed"}
          disabled={!canUpgrade}
          onClick={async () => {
            setSubmitting(true);
            setError(null);
            try {
              await api.researchSmithy({
                slotId: detail.slotId,
                unitName: option.unitName,
              });
              await onMutate();
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setSubmitting(false);
            }
          }}
        >
          Upgrade
        </button>
      </div>
      {!affordable ? <div class="text-xs text-red-600">Not enough resources</div> : null}
      {queueFull ? <div class="text-xs text-amber-700">Upgrade queue is full</div> : null}
      {!queueFull && alreadyQueued ? <div class="text-xs text-amber-700">Already in upgrade queue</div> : null}
      {reachedMaxLevel ? <div class="text-xs text-gray-500">Max level reached</div> : null}
      {error ? <div class="text-xs text-red-600">{error}</div> : null}
    </div>
  );
}

export function BuildingPage({
  data,
  onMutate,
}: {
  data: BuildingPageResponse;
  onMutate: () => Promise<void>;
}) {
  const detail = data.detail;
  const expansion = detail.expansion;
  const query = new URLSearchParams(window.location.search);
  const initialTargetX = Number(query.get("target_x") ?? "0") || 0;
  const initialTargetY = Number(query.get("target_y") ?? "0") || 0;

  return (
    <div class="container mx-auto p-4 max-w-6xl">
      <h1 class="text-2xl font-bold mb-4">
        {detail.buildingType === "empty" ? (
          `Empty slot #${detail.slotId}`
        ) : (
          <span class="inline-flex items-center gap-3">
            <BuildingSprite buildingName={detail.buildingName} size={64} label={buildingLabel(detail.buildingName)} />
            {buildingLabel(detail.buildingName)} (Level {detail.currentLevel})
          </span>
        )}
      </h1>

      <div class="space-y-6">
        {detail.buildingType !== "empty" ? (
          <>
            {detail.descriptionParagraphs.length > 0 ? (
              <div class="text-gray-700 text-sm space-y-2">
                {detail.descriptionParagraphs.map((paragraph, idx) => (
                  <p key={idx}>{paragraph}</p>
                ))}
              </div>
            ) : null}
          </>
        ) : (
          <div class="text-sm text-gray-600">Select a building to start construction.</div>
        )}

        {detail.buildingType === "empty" ? <EmptySlotBuilding detail={detail} onMutate={onMutate} /> : null}

        {detail.buildingType === "expansion" && expansion ? <ExpansionBuilding expansion={expansion} /> : null}
        <TrainingBuilding detail={detail} onMutate={onMutate} TrainingUnitCard={TrainingUnitCard} />
        <AcademyBuilding detail={detail} onMutate={onMutate} AcademyOptionCard={AcademyOptionCard} />
        <SmithyBuilding detail={detail} onMutate={onMutate} SmithyOptionCard={SmithyOptionCard} />

        {detail.buildingType === "marketplace" ? (
          <MarketplaceBuilding
            detail={detail}
            initialTargetX={initialTargetX}
            initialTargetY={initialTargetY}
            onMutate={onMutate}
          />
        ) : null}

        {detail.buildingType === "rally_point" ? (
          <RallyPointBuilding detail={detail} onMutate={onMutate} />
        ) : null}
        {detail.buildingType !== "empty" ? <UpgradeBuilding data={detail} onMutate={onMutate} /> : null}
      </div>
    </div>
  );
}

