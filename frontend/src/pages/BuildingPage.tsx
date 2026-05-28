import { useEffect, useRef, useState } from "preact/hooks";
import { api } from "@/lib/api";
import { buildingLabel, unitLabel } from "@/lib/labels";
import { UnitSprite } from "@/components/UnitSprite";
import type {
  AcademyResearchOption,
  BuildingPageResponse,
  EmptySlotBuildOption,
  MovementPreviewResponse,
  MarketplaceOffer,
  ResourceAmounts,
  RallyCard,
  SmithyUpgradeOption,
  TrainingUnitOption,
} from "@/types/api";

function formatDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((value) => value.toString().padStart(2, "0")).join(":");
}

function LiveCountdown({
  seconds,
  onElapsed,
}: {
  seconds: number;
  onElapsed?: () => void;
}) {
  const ZERO_RETRY_MAX = 5;
  const ZERO_RETRY_DELAY_MS = 1200;
  const [remaining, setRemaining] = useState(seconds);
  const startedFromPositiveRef = useRef(seconds > 0);
  const notifiedRef = useRef(seconds <= 0);
  const zeroRetryCountRef = useRef(0);

  useEffect(() => {
    setRemaining(seconds);
    startedFromPositiveRef.current = seconds > 0;
    notifiedRef.current = seconds <= 0;
    if (seconds > 0) {
      zeroRetryCountRef.current = 0;
    }
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

  return <span class="font-mono">{formatDuration(remaining)}</span>;
}

function secondsUntil(timestamp: string) {
  const targetMs = new Date(timestamp).getTime();
  if (Number.isNaN(targetMs)) return 0;
  return Math.max(0, Math.floor((targetMs - Date.now()) / 1000));
}

function formatRelativeTime(timestamp: number) {
  const now = Date.now();
  const diffMs = now - timestamp * 1000;
  const diffSec = Math.max(0, Math.floor(diffMs / 1000));
  if (diffSec < 60) return "just now";
  if (diffSec < 3600) {
    const mins = Math.floor(diffSec / 60);
    return `${mins} minute${mins === 1 ? "" : "s"} ago`;
  }
  if (diffSec < 86_400) {
    const hours = Math.floor(diffSec / 3600);
    return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  }
  const days = Math.floor(diffSec / 86_400);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

function canAfford(stored: ResourceAmounts, cost: ResourceAmounts) {
  return (
    stored.lumber >= cost.lumber &&
    stored.clay >= cost.clay &&
    stored.iron >= cost.iron &&
    stored.crop >= cost.crop
  );
}

function formatResourceSummary(resources: ResourceAmounts) {
  return `🌲 ${resources.lumber} 🧱 ${resources.clay} ⛏️ ${resources.iron} 🌾 ${resources.crop}`;
}

type ResourceKey = "lumber" | "clay" | "iron" | "crop";

const RESOURCE_KEYS: ResourceKey[] = ["lumber", "clay", "iron", "crop"];
const RESOURCE_LABELS: Record<ResourceKey, string> = {
  lumber: "Lumber",
  clay: "Clay",
  iron: "Iron",
  crop: "Crop",
};

function nonZeroResourceKeys(resources: ResourceAmounts): ResourceKey[] {
  return RESOURCE_KEYS.filter((key) => resources[key] > 0);
}

function formatSingleResourceSummary(resources: ResourceAmounts) {
  const keys = nonZeroResourceKeys(resources);
  if (keys.length !== 1) {
    return formatResourceSummary(resources);
  }

  const key = keys[0];
  const value = resources[key];
  const icon = key === "lumber" ? "🌲" : key === "clay" ? "🧱" : key === "iron" ? "⛏️" : "🌾";
  return `${icon} ${value}`;
}

function isValidMarketplaceOfferShape(offerResources: ResourceAmounts, seekResources: ResourceAmounts): boolean {
  const offerKeys = nonZeroResourceKeys(offerResources);
  const seekKeys = nonZeroResourceKeys(seekResources);
  if (offerKeys.length !== 1 || seekKeys.length !== 1) return false;
  if (offerKeys[0] === seekKeys[0]) return false;
  const offerTotal = offerResources.lumber + offerResources.clay + offerResources.iron + offerResources.crop;
  const seekTotal = seekResources.lumber + seekResources.clay + seekResources.iron + seekResources.crop;
  if (offerTotal <= 0 || seekTotal <= 0) return false;
  const maxSide = Math.max(offerTotal, seekTotal);
  const minSide = Math.min(offerTotal, seekTotal);
  return maxSide <= minSide * 3;
}

function ResourceCost({ cost }: { cost: ResourceAmounts }) {
  return (
    <div class="flex gap-3 text-sm">
      <span class="text-gray-700">🌲 {cost.lumber}</span>
      <span class="text-gray-700">🧱 {cost.clay}</span>
      <span class="text-gray-700">⛏️ {cost.iron}</span>
      <span class="text-gray-700">🌾 {cost.crop}</span>
    </div>
  );
}

function UpgradeBlock({
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
          <div class="mb-3">
            <p class="text-sm text-gray-600 mb-2">Cost:</p>
            <ResourceCost cost={data.cost} />
          </div>

          <div class="grid grid-cols-2 gap-2 text-sm mb-3">
            <div>
              <span class="text-gray-600">Duration: </span>
              <span class="font-semibold">{formatDuration(data.timeSecs)}</span>
            </div>
            <div>
              <span class="text-gray-600">Upkeep: </span>
              <span class="font-semibold">
                {data.currentUpkeep} → {data.nextUpkeep}
              </span>
            </div>
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

function EmptySlotSection({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  if (!detail.emptySlot) return null;

  if (detail.emptySlot.hasQueueForSlot) {
    const preview = detail.emptySlot.queuedUpgradePreview;
    const queuedUpgradeDetail = preview
      ? {
          ...detail,
          buildingName: preview.buildingName,
          buildingType: "generic" as const,
          currentLevel: preview.currentLevel,
          nextLevel: preview.nextLevel,
          currentUpkeep: preview.currentUpkeep,
          nextUpkeep: preview.nextUpkeep,
          timeSecs: preview.timeSecs,
          atMaxLevel: preview.atMaxLevel,
          nextValue: preview.nextValue,
          cost: preview.cost,
        }
      : null;

    return (
      <div class="space-y-4">
        <div class="text-sm text-amber-700 bg-amber-50 border border-amber-200 rounded px-3 py-2">
          Construction in progress
          {detail.emptySlot.queuedBuildingName && detail.emptySlot.queuedTargetLevel
            ? `: ${buildingLabel(detail.emptySlot.queuedBuildingName)} (Level ${detail.emptySlot.queuedTargetLevel})`
            : ""}
        </div>

        {queuedUpgradeDetail ? <UpgradeBlock data={queuedUpgradeDetail} onMutate={onMutate} /> : null}
        {error ? <div class="text-sm text-red-600">{error}</div> : null}
      </div>
    );
  }

  const renderOption = (option: EmptySlotBuildOption, locked: boolean) => {
    const affordable = canAfford(detail.storedResources, option.cost);
    const canBuild = !locked && !detail.queueFull && affordable;
    const isSubmitting = submitting === option.buildingName;

    return (
      <div key={option.buildingName} class="border rounded-md p-4 bg-white space-y-3">
        <div class="flex items-start justify-between gap-2">
          <div class="text-lg font-semibold text-gray-900">{buildingLabel(option.buildingName)}</div>
          {locked ? <span class="text-xs text-amber-700 font-semibold uppercase">Locked</span> : null}
        </div>
        <ResourceCost cost={option.cost} />
        <div class="text-sm text-gray-600">Build time: {formatDuration(option.timeSecs)}</div>
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
      {detail.emptySlot.hasQueueForSlot ? (
        <div class="text-sm text-amber-700 bg-amber-50 border border-amber-200 rounded px-3 py-2">
          Construction in progress
          {detail.emptySlot.queuedBuildingName && detail.emptySlot.queuedTargetLevel
            ? `: ${buildingLabel(detail.emptySlot.queuedBuildingName)} (Level ${detail.emptySlot.queuedTargetLevel})`
            : ""}
        </div>
      ) : null}

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
          <div class="text-xs text-gray-500">Training time: {formatDuration(option.timeSecs)}</div>
        </div>
        <div class="text-xs text-gray-500">Upkeep {option.upkeep}</div>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 text-sm">
        <div class="rounded border bg-gray-50 px-2 py-1">
          <div class="text-[11px] uppercase text-gray-500">Attack</div>
          <div class="font-semibold text-gray-900">{option.attack}</div>
        </div>
        <div class="rounded border bg-gray-50 px-2 py-1">
          <div class="text-[11px] uppercase text-gray-500">Def. Infantry</div>
          <div class="font-semibold text-gray-900">{option.defenseInfantry}</div>
        </div>
        <div class="rounded border bg-gray-50 px-2 py-1">
          <div class="text-[11px] uppercase text-gray-500">Def. Cavalry</div>
          <div class="font-semibold text-gray-900">{option.defenseCavalry}</div>
        </div>
        <div class="rounded border bg-gray-50 px-2 py-1">
          <div class="text-[11px] uppercase text-gray-500">Upkeep</div>
          <div class="font-semibold text-gray-900">{option.upkeep}</div>
        </div>
      </div>

      <div>
        <div class="text-xs uppercase text-gray-500">Training cost</div>
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-2 text-sm">
          <div class="flex items-center justify-between">
            <span>🌲 Lumber</span>
            <span class="font-semibold">{option.cost.lumber}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>🧱 Clay</span>
            <span class="font-semibold">{option.cost.clay}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>⚒️ Iron</span>
            <span class="font-semibold">{option.cost.iron}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>🌾 Crop</span>
            <span class="font-semibold">{option.cost.crop}</span>
          </div>
        </div>
      </div>

      {error ? <div class="text-xs text-red-600">{error}</div> : null}
      <div class="flex flex-col sm:flex-row sm:items-end gap-3">
        <label class="flex-1 text-sm text-gray-600">
          Quantity
          <input
            type="number"
            min="1"
            value={quantity}
            onInput={(event) => setQuantity(Math.max(1, Number((event.target as HTMLInputElement).value || "1")))}
            class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
          />
        </label>
        <button
          type="button"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
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
      <div class="flex items-center justify-between">
        <div>
          <div class="text-lg font-semibold text-gray-900">{unitLabel(option.unitName)}</div>
          <div class="text-xs text-gray-500">Research time: {formatDuration(option.timeSecs)}</div>
        </div>
      </div>
      <ResourceCost cost={option.cost} />
      <button
        type="button"
        class={
          canResearch
            ? "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
            : "bg-emerald-600 text-white font-semibold px-4 py-2 rounded opacity-60 cursor-not-allowed"
        }
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

  return (
    <div class="border rounded-md p-4 bg-white space-y-3">
      <div class="flex items-center justify-between">
        <div>
          <div class="text-lg font-semibold text-gray-900">{unitLabel(option.unitName)}</div>
          <div class="text-xs text-gray-500">
            Level {option.currentLevel} → {option.currentLevel + 1} (Max: {option.maxLevel})
          </div>
          <div class="text-xs text-gray-500">Upgrade time: {formatDuration(option.timeSecs)}</div>
        </div>
      </div>
      <ResourceCost cost={option.cost} />
      <button
        type="button"
        class={
          canUpgrade
            ? "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded"
            : "bg-blue-600 text-white font-semibold px-4 py-2 rounded opacity-60 cursor-not-allowed"
        }
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
      {!affordable ? <div class="text-xs text-red-600">Not enough resources</div> : null}
      {queueFull ? <div class="text-xs text-amber-700">Upgrade queue is full</div> : null}
      {!option.canUpgrade ? <div class="text-xs text-gray-500">Max level reached</div> : null}
      {error ? <div class="text-xs text-red-600">{error}</div> : null}
    </div>
  );
}

function MarketplaceSection({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const [send, setSend] = useState({
    targetX: 0,
    targetY: 0,
    lumber: 0,
    clay: 0,
    iron: 0,
    crop: 0,
  });
  const [offer, setOffer] = useState({
    offerResource: "lumber" as ResourceKey,
    offerAmount: 0,
    seekResource: "clay" as ResourceKey,
    seekAmount: 0,
  });
  const [error, setError] = useState<string | null>(null);

  if (!detail.marketplace) return null;

  return (
    <>
      <div class="border rounded-md p-4 bg-white space-y-4">
        <div>
          <div class="text-sm text-gray-500 uppercase">Send resources</div>
          <p class="text-sm text-gray-500">
            Available merchants: {detail.marketplace.availableMerchants}/{detail.marketplace.totalMerchants}
          </p>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="text-sm text-gray-600">
            Target X
            <input
              type="number"
              value={send.targetX}
              onInput={(e) => setSend((v) => ({ ...v, targetX: Number((e.target as HTMLInputElement).value || "0") }))}
              class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
            />
          </label>
          <label class="text-sm text-gray-600">
            Target Y
            <input
              type="number"
              value={send.targetY}
              onInput={(e) => setSend((v) => ({ ...v, targetY: Number((e.target as HTMLInputElement).value || "0") }))}
              class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
            />
          </label>
        </div>
        <div class="grid gap-3 sm:grid-cols-4">
          {(["lumber", "clay", "iron", "crop"] as const).map((key) => (
            <label key={key} class="text-sm text-gray-600">
              {key[0].toUpperCase() + key.slice(1)}
              <input
                type="number"
                min="0"
                value={send[key]}
                onInput={(e) =>
                  setSend((v) => ({ ...v, [key]: Math.max(0, Number((e.target as HTMLInputElement).value || "0")) }))
                }
                class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
              />
            </label>
          ))}
        </div>
        <button
          type="button"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
          onClick={async () => {
            setError(null);
            try {
              await api.sendResources({ slotId: detail.slotId, ...send });
              await onMutate();
            } catch (err) {
              setError((err as Error).message);
            }
          }}
        >
          Send resources
        </button>
      </div>

      <div class="border rounded-md p-4 bg-white space-y-4">
        <div>
          <div class="text-sm text-gray-500 uppercase">Create offer</div>
          <p class="text-sm text-gray-500">
            One resource type per side. Offer and seek resources must be different. Ratio must stay
            between 1:3 and 3:1.
          </p>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <div class="space-y-2">
            <div class="text-sm font-semibold text-gray-700">Offering</div>
            <label class="text-sm text-gray-600 block">
              Resource
              <select
                value={offer.offerResource}
                onChange={(e) =>
                  setOffer((v) => ({ ...v, offerResource: (e.target as HTMLSelectElement).value as ResourceKey }))
                }
                class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
              >
                {RESOURCE_KEYS.map((key) => (
                  <option value={key} key={key}>
                    {RESOURCE_LABELS[key]}
                  </option>
                ))}
              </select>
            </label>
            <label class="text-sm text-gray-600 block">
              Amount
              <input
                type="number"
                min="0"
                value={offer.offerAmount}
                onInput={(e) =>
                  setOffer((v) => ({
                    ...v,
                    offerAmount: Math.max(0, Number((e.target as HTMLInputElement).value || "0")),
                  }))
                }
                class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
              />
            </label>
          </div>
          <div class="space-y-2">
            <div class="text-sm font-semibold text-gray-700">Seeking</div>
            <label class="text-sm text-gray-600 block">
              Resource
              <select
                value={offer.seekResource}
                onChange={(e) =>
                  setOffer((v) => ({ ...v, seekResource: (e.target as HTMLSelectElement).value as ResourceKey }))
                }
                class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
              >
                {RESOURCE_KEYS.map((key) => (
                  <option value={key} key={key}>
                    {RESOURCE_LABELS[key]}
                  </option>
                ))}
              </select>
            </label>
            <label class="text-sm text-gray-600 block">
              Amount
              <input
                type="number"
                min="0"
                value={offer.seekAmount}
                onInput={(e) =>
                  setOffer((v) => ({
                    ...v,
                    seekAmount: Math.max(0, Number((e.target as HTMLInputElement).value || "0")),
                  }))
                }
                class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
              />
            </label>
          </div>
        </div>
        <button
          type="button"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
          onClick={async () => {
            setError(null);
            try {
              if (offer.offerAmount <= 0 || offer.seekAmount <= 0) {
                throw new Error("Offer and seek amounts must be greater than zero.");
              }
              if (offer.offerResource === offer.seekResource) {
                throw new Error("Offer and seek resources must be different.");
              }
              const maxSide = Math.max(offer.offerAmount, offer.seekAmount);
              const minSide = Math.min(offer.offerAmount, offer.seekAmount);
              if (maxSide > minSide * 3) {
                throw new Error("Offer ratio must stay between 1:3 and 3:1.");
              }

              await api.createMarketplaceOffer({
                slotId: detail.slotId,
                offerLumber: offer.offerResource === "lumber" ? offer.offerAmount : 0,
                offerClay: offer.offerResource === "clay" ? offer.offerAmount : 0,
                offerIron: offer.offerResource === "iron" ? offer.offerAmount : 0,
                offerCrop: offer.offerResource === "crop" ? offer.offerAmount : 0,
                seekLumber: offer.seekResource === "lumber" ? offer.seekAmount : 0,
                seekClay: offer.seekResource === "clay" ? offer.seekAmount : 0,
                seekIron: offer.seekResource === "iron" ? offer.seekAmount : 0,
                seekCrop: offer.seekResource === "crop" ? offer.seekAmount : 0,
              });
              await onMutate();
            } catch (err) {
              setError((err as Error).message);
            }
          }}
        >
          Create offer
        </button>
      </div>

      <OffersTable title="Your offers" offers={detail.marketplace.ownOffers} actionLabel="Cancel" onAction={async (offer) => {
        await api.cancelMarketplaceOffer({ offerId: offer.offerId, slotId: detail.slotId });
        await onMutate();
      }} />
      <OffersTable title="Global marketplace" offers={detail.marketplace.globalOffers} actionLabel="Accept" enforceTradeRules onAction={async (offer) => {
        await api.acceptMarketplaceOffer({ offerId: offer.offerId, slotId: detail.slotId });
        await onMutate();
      }} />

      <div class="border rounded-md p-4 bg-white space-y-3">
        <div class="text-sm text-gray-500 uppercase">Merchant movements</div>
        {detail.marketplace.merchantMovements.length === 0 ? (
          <p class="text-sm text-gray-500">No merchant movements to display.</p>
        ) : (
          <div class="overflow-x-auto">
            <table class="min-w-full text-sm">
              <thead class="text-left text-xs uppercase text-gray-500 border-b">
                <tr>
                  <th class="py-2 pr-4">Direction</th>
                  <th class="py-2 pr-4">Route</th>
                  <th class="py-2 pr-4">Resources</th>
                  <th class="py-2 pr-4">Merchants</th>
                  <th class="py-2">Arrives</th>
                </tr>
              </thead>
              <tbody>
                {detail.marketplace.merchantMovements.map((movement) => {
                  const origin = movement.originPosition
                    ? `${movement.originName} (${movement.originPosition.x}|${movement.originPosition.y})`
                    : movement.originName;
                  const destination = movement.destinationPosition
                    ? `${movement.destinationName} (${movement.destinationPosition.x}|${movement.destinationPosition.y})`
                    : movement.destinationName;
                  return (
                    <tr key={movement.jobId} class="border-b last:border-b-0">
                      <td class="py-2 pr-4 text-gray-700">
                        {movement.direction} ({movement.kind})
                      </td>
                      <td class="py-2 pr-4">{origin} → {destination}</td>
                      <td class="py-2 pr-4">{formatResourceSummary(movement.resources)}</td>
                      <td class="py-2 pr-4">{movement.merchantsUsed}</td>
                      <td class="py-2 font-mono text-gray-600">
                        <LiveCountdown
                          seconds={secondsUntil(movement.arrivesAt)}
                          onElapsed={() => {
                            void onMutate();
                          }}
                        />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {error ? <div class="text-sm text-red-600">{error}</div> : null}
    </>
  );
}

function OffersTable({
  title,
  offers,
  actionLabel,
  enforceTradeRules,
  onAction,
}: {
  title: string;
  offers: MarketplaceOffer[];
  actionLabel: string;
  enforceTradeRules?: boolean;
  onAction: (offer: MarketplaceOffer) => Promise<void>;
}) {
  return (
    <div class="border rounded-md p-4 bg-white space-y-3">
      <div class="text-sm text-gray-500 uppercase">{title}</div>
      {offers.length === 0 ? (
        <p class="text-sm text-gray-500">No offers available.</p>
      ) : (
        <div class="overflow-x-auto">
          <table class="min-w-full text-sm">
            <thead class="text-left text-xs uppercase text-gray-500 border-b">
              <tr>
                <th class="py-2 pr-4">Village</th>
                <th class="py-2 pr-4">Offering</th>
                <th class="py-2 pr-4">Seeking</th>
                <th class="py-2 pr-4">Merchants</th>
                <th class="py-2 pr-4">Created</th>
                <th class="py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {offers.map((offer) => (
                <tr key={offer.offerId} class="border-b last:border-b-0">
                  <td class="py-2 pr-4">{offer.villageName} ({offer.position.x}|{offer.position.y})</td>
                  <td class="py-2 pr-4">{formatSingleResourceSummary(offer.offerResources)}</td>
                  <td class="py-2 pr-4">{formatSingleResourceSummary(offer.seekResources)}</td>
                  <td class="py-2 pr-4">{offer.merchantsRequired}</td>
                  <td class="py-2 pr-4 text-gray-600">{formatRelativeTime(offer.createdAt)}</td>
                  <td class="py-2">
                    {(() => {
                      const invalidTrade =
                        Boolean(enforceTradeRules) &&
                        !isValidMarketplaceOfferShape(offer.offerResources, offer.seekResources);
                      return (
                    <button
                      type="button"
                      class={
                        invalidTrade
                          ? "bg-gray-400 text-white text-xs font-semibold px-3 py-1.5 rounded cursor-not-allowed"
                          : actionLabel === "Cancel"
                          ? "bg-red-600 hover:bg-red-700 text-white text-xs font-semibold px-3 py-1.5 rounded"
                          : "bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-semibold px-3 py-1.5 rounded"
                      }
                      disabled={invalidTrade}
                      title={invalidTrade ? "Invalid offer rules (single-resource and ratio constraints)." : undefined}
                      onClick={() => onAction(offer)}
                    >
                      {actionLabel}
                    </button>
                      );
                    })()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function RallyPointSection({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const query = new URLSearchParams(window.location.search);
  const initialTargetX = Number(query.get("target_x") ?? "0") || 0;
  const initialTargetY = Number(query.get("target_y") ?? "0") || 0;
  const [targetX, setTargetX] = useState(initialTargetX);
  const [targetY, setTargetY] = useState(initialTargetY);
  const [movement, setMovement] = useState<"attack" | "raid" | "reinforcement">("attack");
  const [scoutingTarget, setScoutingTarget] = useState<"resources" | "defenses">("resources");
  const [catapultTarget1, setCatapultTarget1] = useState("MainBuilding");
  const [catapultTarget2, setCatapultTarget2] = useState("Warehouse");
  const [units, setUnits] = useState<Record<number, number>>({});
  const [preview, setPreview] = useState<MovementPreviewResponse | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!detail.rallyPoint) return null;

  const toUnitsArray = () => {
    const arr = Array.from({ length: 10 }, (_, idx) => units[idx] ?? 0);
    return arr;
  };

  const isScoutUnitName = (name: string) =>
    name === "Scout" || name === "Pathfinder" || name === "EquitesLegati";
  const isCatapultUnitName = (name: string) =>
    name === "Catapult" || name === "FireCatapult" || name === "Trebuchet" || name === "Ballista";

  const selectedScoutUnits = detail.rallyPoint.sendableUnits.filter((unit) => {
    const selected = units[unit.unitIdx] ?? 0;
    return selected > 0 && isScoutUnitName(unit.name);
  });
  const selectedCatapultUnits = detail.rallyPoint.sendableUnits
    .filter((unit) => isCatapultUnitName(unit.name))
    .reduce((sum, unit) => sum + (units[unit.unitIdx] ?? 0), 0);
  const isScoutDetected = preview?.detectedKind === "scout_only";
  const showScoutingTargetChoice =
    movement !== "reinforcement" && !!preview?.supportsScoutingTargetChoice;
  const showCatapultTargets =
    movement === "attack" && !isScoutDetected && !!preview?.hasCatapultUnits;
  const catapultTargetSelectionCount = selectedCatapultUnits <= 1 ? 1 : 2;

  const fullUnitsFromCard = (card: RallyCard) => card.units.map((value) => Number(value ?? 0));

  return (
    <>
      <div class="space-y-4">
        {(["stationed", "deployed", "reinforcement", "outgoing", "incoming"] as const).map((category) => {
          const cards = detail.rallyPoint!.cards.filter((card) => card.category === category);
          if (cards.length === 0) return null;
          return (
            <div class="space-y-2" key={category}>
              <h3 class="text-sm font-semibold text-gray-700">{category}</h3>
              <div class="space-y-2">
                {cards.map((card) => (
                  <div key={`${category}-${card.villageId}-${card.actionId ?? "no-action"}`} class="border rounded-lg p-4 bg-white shadow-sm space-y-3">
                    <div class="flex justify-between items-start">
                      <div class="flex-1">
                        <div class="flex items-center gap-2">
                          <h3 class="font-semibold text-gray-900">{card.villageName ?? "Unknown Village"}</h3>
                          {card.movementKind ? (
                            <span class="text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-800">{card.movementKind}</span>
                          ) : null}
                        </div>
                        {card.position ? <p class="text-sm text-gray-600 mt-1">({card.position.x}, {card.position.y})</p> : null}
                        <p class="text-xs text-gray-500 mt-1">Upkeep: {card.upkeep}</p>
                        {card.arrivesAt ? (
                          <div class="mt-1 space-y-1 text-sm text-gray-500">
                            <p class="font-mono">
                              ⏱️{" "}
                              <LiveCountdown
                                seconds={secondsUntil(card.arrivesAt)}
                                onElapsed={() => {
                                  void onMutate();
                                }}
                              />
                            </p>
                            <p>Arrives at: <span class="font-mono">{new Date(card.arrivesAt).toLocaleString()}</span></p>
                          </div>
                        ) : null}
                      </div>
                      <span class="text-xs px-2 py-1 rounded font-medium whitespace-nowrap bg-gray-100 text-gray-800">{card.category}</span>
                    </div>

                    <div class="overflow-x-auto">
                      <table class="w-full border-collapse">
                        <thead>
                          <tr>
                            {card.units.map((_, idx) => (
                              <th key={`icon-${idx}`} class="text-center p-1 border-r last:border-r-0 bg-white">
                                <UnitSprite tribe={card.tribe} unitIndex={idx} label={unitLabel(detail.rallyPoint!.sendableUnits[idx]?.name ?? `U${idx + 1}`)} />
                              </th>
                            ))}
                          </tr>
                        </thead>
                        <tbody>
                          <tr>
                            {card.units.map((count, idx) => (
                              <td key={idx} class={count === 0 ? "text-center p-2 border-r last:border-r-0 bg-gray-50 opacity-40" : "text-center p-2 border-r last:border-r-0 bg-gray-100"}>
                                <div class={count === 0 ? "text-gray-400 text-sm" : "text-gray-900 font-semibold"}>{count}</div>
                              </td>
                            ))}
                          </tr>
                        </tbody>
                      </table>
                    </div>

                    {card.action === "recall" && card.actionId ? (
                      <button
                        type="button"
                        class="inline-block px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white text-sm rounded"
                        onClick={async () => {
                          await api.recallTroops({
                            villageId: detail.villageId,
                            armyId: card.actionId!,
                            units: fullUnitsFromCard(card),
                          });
                          await onMutate();
                        }}
                      >
                        ↩️ Recall Troops
                      </button>
                    ) : null}
                    {card.action === "release" && card.actionId ? (
                      <button
                        type="button"
                        class="inline-block px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded"
                        onClick={async () => {
                          await api.releaseReinforcements({
                            sourceVillageId: card.villageId,
                            units: fullUnitsFromCard(card),
                          });
                          await onMutate();
                        }}
                      >
                        🏠 Release Reinforcements
                      </button>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>

      <div class="border rounded-md p-4 bg-white space-y-4">
        <div>
          <div class="text-sm text-gray-500 uppercase">Send troops</div>
          <p class="text-sm text-gray-500">Select target and units.</p>
        </div>
        <div class="grid gap-3 sm:grid-cols-3">
          <label class="text-sm text-gray-600">
            Target X
            <input type="number" value={targetX} onInput={(e) => setTargetX(Number((e.target as HTMLInputElement).value || "0"))} class="mt-1 w-full border rounded px-3 py-2 text-gray-700" />
          </label>
          <label class="text-sm text-gray-600">
            Target Y
            <input type="number" value={targetY} onInput={(e) => setTargetY(Number((e.target as HTMLInputElement).value || "0"))} class="mt-1 w-full border rounded px-3 py-2 text-gray-700" />
          </label>
          <label class="text-sm text-gray-600">
            Movement type
            <select value={movement} onChange={(e) => setMovement((e.target as HTMLSelectElement).value as "attack" | "raid" | "reinforcement")} class="mt-1 w-full border rounded px-3 py-2 text-gray-700">
              <option value="attack">Attack</option>
              <option value="raid">Raid</option>
              <option value="reinforcement">Reinforcement</option>
            </select>
          </label>
        </div>
        <div class="space-y-2">
          <div class="text-sm text-gray-500 uppercase">Select units</div>
          {detail.rallyPoint.sendableUnits.map((unit) => (
            <label key={unit.unitIdx} class={unit.isResearched ? "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-sm text-gray-700 border rounded-md px-3 py-2" : "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-sm text-gray-400 border rounded-md px-3 py-2 bg-gray-50"}>
              <span class="font-semibold">{unitLabel(unit.name)}</span>
              {unit.isResearched ? (
                <>
                  <span class="text-xs text-gray-500">Available: {unit.available}</span>
                  <input
                    type="number"
                    min="0"
                    max={unit.available}
                    value={units[unit.unitIdx] ?? 0}
                    onInput={(e) =>
                      setUnits((v) => ({
                        ...v,
                        [unit.unitIdx]: Math.min(unit.available, Math.max(0, Number((e.target as HTMLInputElement).value || "0"))),
                      }))
                    }
                    class="w-full sm:w-32 border rounded px-2 py-1 text-gray-700"
                  />
                </>
              ) : (
                <span class="text-xs text-gray-500">Not researched</span>
              )}
            </label>
          ))}
        </div>

        <button
          type="button"
          class="bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded"
          disabled={previewing || sending}
          onClick={async () => {
            setError(null);
            setPreview(null);
            try {
              setPreviewing(true);
              const result = await api.previewTroops({
                targetX,
                targetY,
                movement,
                units: toUnitsArray(),
              });
              setPreview(result);
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setPreviewing(false);
            }
          }}
        >
          {previewing ? "Calculating..." : "Preview movement"}
        </button>
        {preview ? (
          <div class="rounded border border-emerald-200 bg-emerald-50 p-3 space-y-2 text-sm">
            <div>
              Arrives at: <span class="font-semibold">{new Date(preview.arrivesAt).toLocaleString()}</span>
            </div>
            <div>
              Time remaining: <span class="font-semibold">{formatDuration(secondsUntil(preview.arrivesAt))}</span>
            </div>
            <div>
              Detected movement:{" "}
              <span class="font-semibold">
                {preview.detectedKind === "scout_only"
                  ? `Scout-only (${movement === "raid" ? "Raid" : "Attack"})`
                  : preview.detectedKind === "reinforcement"
                    ? "Reinforcement"
                    : "Attack/Raid"}
              </span>
            </div>
            {showScoutingTargetChoice ? (
              <div class="grid gap-2">
                <label class="text-sm text-gray-700">
                  Scouting target
                  <select
                    value={scoutingTarget}
                    onChange={(e) =>
                      setScoutingTarget((e.target as HTMLSelectElement).value as "resources" | "defenses")
                    }
                    class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
                  >
                    <option value="resources">Resources + troops</option>
                    <option value="defenses">Residence/Palace + Walls + troops</option>
                  </select>
                </label>
              </div>
            ) : null}
            {showCatapultTargets ? (
              <div class="grid gap-2">
                <div class="text-xs text-gray-700">
                  Catapults detected: select {catapultTargetSelectionCount === 1 ? "one target building" : "up to two target buildings"}.
                </div>
                <label class="text-sm text-gray-700">
                  Catapult target #1
                  <select
                    value={catapultTarget1}
                    onChange={(e) => setCatapultTarget1((e.target as HTMLSelectElement).value)}
                    class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
                  >
                    <option value="MainBuilding">Main Building</option>
                    <option value="Warehouse">Warehouse</option>
                    <option value="Granary">Granary</option>
                    <option value="RallyPoint">Rally Point</option>
                    <option value="Barracks">Barracks</option>
                    <option value="Stable">Stable</option>
                    <option value="Workshop">Workshop</option>
                    <option value="Academy">Academy</option>
                    <option value="Residence">Residence</option>
                    <option value="Palace">Palace</option>
                    <option value="Smithy">Smithy</option>
                  </select>
                </label>
                {catapultTargetSelectionCount > 1 ? (
                  <label class="text-sm text-gray-700">
                    Catapult target #2
                    <select
                      value={catapultTarget2}
                      onChange={(e) => setCatapultTarget2((e.target as HTMLSelectElement).value)}
                      class="mt-1 w-full border rounded px-3 py-2 text-gray-700"
                    >
                      <option value="MainBuilding">Main Building</option>
                      <option value="Warehouse">Warehouse</option>
                      <option value="Granary">Granary</option>
                      <option value="RallyPoint">Rally Point</option>
                      <option value="Barracks">Barracks</option>
                      <option value="Stable">Stable</option>
                      <option value="Workshop">Workshop</option>
                      <option value="Academy">Academy</option>
                      <option value="Residence">Residence</option>
                      <option value="Palace">Palace</option>
                      <option value="Smithy">Smithy</option>
                    </select>
                  </label>
                ) : null}
              </div>
            ) : null}
            <button
              type="button"
              class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded"
              disabled={sending}
              onClick={async () => {
                setError(null);
                try {
                  setSending(true);
                  if (showScoutingTargetChoice && selectedScoutUnits.length === 0) {
                    throw new Error("Scout movement requires at least one scout unit.");
                  }
                  await api.sendTroops({
                    slotId: detail.slotId,
                    targetX,
                    targetY,
                    movement,
                    scoutingTarget: showScoutingTargetChoice ? scoutingTarget : undefined,
                    catapultTargets: showCatapultTargets
                      ? (catapultTargetSelectionCount === 1
                        ? [catapultTarget1]
                        : [catapultTarget1, catapultTarget2])
                      : undefined,
                    units: toUnitsArray(),
                  });
                  await onMutate();
                  window.location.assign(`/app/build/39?target_x=${targetX}&target_y=${targetY}`);
                } catch (err) {
                  setError((err as Error).message);
                } finally {
                  setSending(false);
                }
              }}
            >
              {sending ? "Sending..." : "Confirm and send"}
            </button>
          </div>
        ) : null}
        {error ? <div class="text-sm text-red-600">{error}</div> : null}
      </div>
    </>
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

  return (
    <div class="container mx-auto p-4 max-w-6xl">
      <h1 class="text-2xl font-bold mb-4">
        {detail.buildingType === "empty"
          ? `Empty slot #${detail.slotId}`
          : `${buildingLabel(detail.buildingName)} (Level ${detail.currentLevel})`}
      </h1>

      <div class="space-y-6">
        {detail.buildingType !== "empty" ? (
          <>
            <div>
              <div class="text-sm text-gray-500 uppercase">Existing</div>
              <div class="text-2xl font-semibold">{buildingLabel(detail.buildingName)}</div>
              {detail.descriptionParagraphs.length > 0 ? (
                <div class="mt-2 text-gray-700 text-sm space-y-2">
                  {detail.descriptionParagraphs.map((paragraph, idx) => (
                    <p key={idx}>{paragraph}</p>
                  ))}
                </div>
              ) : null}
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
              <div class="p-3 border rounded-md bg-gray-50">
                <div class="text-gray-500">Level</div>
                <div class="text-lg font-bold">{detail.currentLevel}</div>
              </div>
              <div class="p-3 border rounded-md bg-gray-50">
                <div class="text-gray-500">Population</div>
                <div class="text-lg font-bold">{detail.population}</div>
              </div>
            </div>
          </>
        ) : (
          <div class="text-sm text-gray-600">Select a building to start construction.</div>
        )}

        {detail.buildingType === "empty" ? (
          <EmptySlotSection detail={detail} onMutate={onMutate} />
        ) : (
          <UpgradeBlock data={detail} onMutate={onMutate} />
        )}

        {detail.buildingType === "expansion" && detail.expansion ? (
          <div class="space-y-4">
            <div class="bg-white border border-gray-300 rounded-lg p-6 shadow-sm">
              <h2 class="text-xl font-semibold mb-4 text-gray-800">Culture Points</h2>
              <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="bg-blue-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Village Production</div>
                  <div class="text-2xl font-bold text-blue-700">
                    {detail.expansion.villageCulturePointsProduction}
                    <span class="text-sm font-normal text-gray-600 ml-1">/ day</span>
                  </div>
                </div>
                <div class="bg-green-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Total Production</div>
                  <div class="text-2xl font-bold text-green-700">
                    {detail.expansion.accountCulturePointsProduction}
                    <span class="text-sm font-normal text-gray-600 ml-1">/ day</span>
                  </div>
                </div>
                <div class="bg-purple-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Total Culture Points</div>
                  <div class="text-2xl font-bold text-purple-700">{detail.expansion.accountCulturePoints}</div>
                </div>
              </div>
              <div class="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
                <div class="text-sm font-medium text-gray-700">
                  Next village requires:{" "}
                  <span class="font-bold text-yellow-700">{detail.expansion.nextCpRequired}</span>{" "}
                  Culture Points
                </div>
              </div>
            </div>

            <div class="bg-white border border-gray-300 rounded-lg p-6 shadow-sm">
              <h2 class="text-xl font-semibold mb-4 text-gray-800">Expansion</h2>
              <h3 class="text-lg font-medium mb-2 text-gray-700">Foundation Slots</h3>
              <div class="flex items-center gap-2 flex-wrap">
                {Array.from({ length: detail.expansion.maxFoundationSlots }).map((_, i) =>
                  i < detail.expansion.childVillagesCount ? (
                    <div
                      key={i}
                      class="w-14 h-14 bg-red-500 border-2 border-red-700 rounded flex items-center justify-center"
                    >
                      <span class="text-white font-bold">✓</span>
                    </div>
                  ) : (
                    <div
                      key={i}
                      class="w-14 h-14 bg-green-500 border-2 border-green-700 rounded flex items-center justify-center"
                    >
                      <span class="text-white font-bold">○</span>
                    </div>
                  ),
                )}
              </div>
              <div class="text-sm text-gray-600 mt-2">
                {detail.expansion.childVillagesCount} / {detail.expansion.maxFoundationSlots} slots
                used
              </div>

              <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
                <div class="bg-blue-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Settlers at Home</div>
                  <div class="text-2xl font-bold text-blue-700">{detail.expansion.settlersAtHome}</div>
                </div>
                <div class="bg-purple-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Settlers Deployed</div>
                  <div class="text-2xl font-bold text-purple-700">{detail.expansion.settlersDeployed}</div>
                </div>
                <div class="bg-amber-50 p-4 rounded-lg">
                  <div class="text-sm text-gray-600 mb-1">Trainable Settlers</div>
                  <div class="text-2xl font-bold text-amber-700">{detail.expansion.maxSettlersTrainable}</div>
                </div>
              </div>
            </div>
          </div>
        ) : null}

        {(detail.buildingType === "training" || detail.buildingType === "expansion") && detail.training ? (
          <>
            <div class="space-y-3">
              <div class="text-sm text-gray-500 uppercase">
                {detail.buildingType === "expansion" ? "Train expansion units" : "Train units"}
              </div>
              {detail.training.units.length === 0 ? (
                <p class="text-sm text-gray-500">
                  {detail.buildingType === "expansion"
                    ? "No expansion units available for training."
                    : "No units available."}
                </p>
              ) : (
                <div class="space-y-4">
                  {detail.training.units.map((option) => (
                    <TrainingUnitCard
                      key={`${detail.slotId}-${option.unitIdx}`}
                      option={option}
                      detail={detail}
                      onMutate={onMutate}
                    />
                  ))}
                </div>
              )}
            </div>
            {detail.training.queue.length > 0 ? (
              <div class="border rounded-md p-4 bg-gray-50 space-y-2">
                <div class="text-sm text-gray-500 uppercase">Training queue</div>
                {detail.training.queue.map((job, index) => (
                  <div key={`${job.unitName}-${index}`} class="p-3 bg-white border rounded-md space-y-1 text-sm">
                    <div class="flex items-center justify-between font-semibold text-gray-800">
                      <span>
                        {job.quantity} × {unitLabel(job.unitName)}
                      </span>
                      <span class="text-xs text-gray-500">
                        Training time {formatDuration(job.timePerUnit)}
                      </span>
                    </div>
                    <div class="flex items-center justify-between text-xs text-gray-600">
                      <span>Remaining</span>
                      <LiveCountdown
                        seconds={secondsUntil(job.finishesAt)}
                        onElapsed={() => {
                          void onMutate();
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            ) : null}
          </>
        ) : null}

        {detail.buildingType === "academy" && detail.academy ? (
          <>
            {detail.academy.queue.length > 0 ? (
              <div class="border rounded-md p-4 bg-gray-50 space-y-3">
                <div class="text-sm text-gray-500 uppercase">Research queue</div>
                {detail.academy.queue.map((job, index) => (
                  <div key={`${job.unitName}-${index}`} class="bg-white border rounded-md p-3 text-sm space-y-1">
                    <div class="flex items-center justify-between">
                      <span class="font-semibold text-gray-900">{unitLabel(job.unitName)}</span>
                      <span class={job.isProcessing ? "text-xs font-semibold text-emerald-600" : "text-xs font-semibold text-gray-500"}>
                        {job.isProcessing ? "In progress" : "Pending"}
                      </span>
                    </div>
                    <div class="flex items-center justify-between text-xs text-gray-600">
                      <span>Time remaining</span>
                      <LiveCountdown
                        seconds={secondsUntil(job.finishesAt)}
                        onElapsed={() => {
                          void onMutate();
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            ) : null}

            <div>
              <div class="text-sm text-gray-500 uppercase">Research available</div>
              {detail.academy.readyUnits.length === 0 && detail.academy.lockedUnits.length === 0 ? (
                <p class="text-sm text-gray-500 mt-2">No research available.</p>
              ) : (
                <div class="space-y-4 mt-3">
                  {detail.academy.readyUnits.map((option) => (
                    <AcademyOptionCard
                      key={option.unitName}
                      option={option}
                      detail={detail}
                      onMutate={onMutate}
                    />
                  ))}
                </div>
              )}
            </div>
            {detail.academy.lockedUnits.length > 0 ? (
              <div>
                <div class="text-sm text-gray-500 uppercase">Locked research</div>
                <div class="space-y-4 mt-3">
                  {detail.academy.lockedUnits.map((option) => (
                    <AcademyOptionCard
                      key={option.unitName}
                      option={option}
                      detail={detail}
                      onMutate={onMutate}
                    />
                  ))}
                </div>
              </div>
            ) : null}
          </>
        ) : null}

        {detail.buildingType === "smithy" && detail.smithy ? (
          <div>
            <div class="text-sm text-gray-500 uppercase">Smithy upgrades</div>
            {detail.smithy.units.length === 0 ? (
              <p class="text-sm text-gray-500 mt-2">No units to upgrade.</p>
            ) : (
              <div class="space-y-4 mt-3">
                {detail.smithy.units.map((option) => (
                  <SmithyOptionCard
                    key={option.unitName}
                    option={option}
                    detail={detail}
                    onMutate={onMutate}
                  />
                ))}
              </div>
            )}
          </div>
        ) : null}

        {detail.buildingType === "marketplace" ? (
          <MarketplaceSection detail={detail} onMutate={onMutate} />
        ) : null}

        {detail.buildingType === "rally_point" ? (
          <RallyPointSection detail={detail} onMutate={onMutate} />
        ) : null}
      </div>
    </div>
  );
}
