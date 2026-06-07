import { useEffect, useState } from "preact/hooks";
import type { ComponentChildren } from "preact";
import { api } from "@/lib/api";
import { formatDurationHms, secondsUntilIso } from "@/lib/time";
import { unitLabel } from "@/lib/labels";
import { Link } from "@/components/Link";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSprite, UnitSpriteByName } from "@/components/UnitSprite";
import { LiveCountdown } from "@/components/buildings/buildingShared";
import { Badge, Button, Panel, SectionHeader } from "@/components/ui";
import {
  useRecallTroopsMutation,
  useReleaseReinforcementsMutation,
  useSendTroopsMutation,
} from "@/query/mutations";
import type { BuildingPageResponse, MovementPreviewResponse, RallyCard } from "@/types/api";

function unitsFromCard(card: RallyCard) {
  return Array.from({ length: 10 }, (_, idx) => Number(card.units[idx] ?? 0));
}

function clampUnitAmount(value: number, max: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(max, Math.trunc(value)));
}

function previewMovementLabel(preview: MovementPreviewResponse, selectedMovement: "attack" | "raid" | "reinforcement") {
  if (preview.detectedKind === "scout_only") {
    return `Scout-only ${selectedMovement === "raid" ? "raid" : "attack"}`;
  }
  if (preview.detectedKind === "reinforcement") return "Reinforcement";
  if (preview.detectedKind === "found_village") return "Found village";
  return selectedMovement === "raid" ? "Raid" : "Attack";
}

function UnitAmountGrid({
  available,
  amounts,
  unitLabels,
  disabled,
  renderIcon,
  onChange,
  onSetAll,
}: {
  available: number[];
  amounts: number[];
  unitLabels: string[];
  disabled?: boolean;
  renderIcon: (idx: number) => ComponentChildren;
  onChange: (idx: number, value: number) => void;
  onSetAll?: (idx: number) => void;
}) {
  return (
    <div class="grid grid-cols-2 gap-2 sm:grid-cols-5">
      {available.map((max, idx) => {
        const isDisabled = disabled || max === 0;
        return (
          <label
            key={idx}
            class={isDisabled
              ? "flex items-center gap-2 rounded-md border border-stone-200 bg-white px-2 py-1.5 text-xs text-stone-400 opacity-60"
              : "flex items-center gap-2 rounded-md border border-stone-200 bg-white px-2 py-1.5 text-xs text-stone-700"}
          >
            <span class="inline-flex shrink-0 items-center justify-center">
              {renderIcon(idx)}
            </span>
            <div class="min-w-0 flex-1">
              <div class="mb-1 flex items-center justify-between gap-1">
                <span class="truncate text-[11px] font-semibold text-stone-500">{unitLabels[idx]}</span>
                {onSetAll ? (
                  <button
                    type="button"
                    class={isDisabled ? "text-[11px] text-stone-400" : "text-[11px] font-semibold text-green-800 hover:underline"}
                    disabled={isDisabled}
                    onClick={() => onSetAll(idx)}
                  >
                    {max}
                  </button>
                ) : null}
              </div>
              <input
                type="number"
                min={0}
                max={max}
                value={amounts[idx] ?? 0}
                disabled={isDisabled}
                class="w-full rounded border border-stone-300 px-1.5 py-1 text-right text-sm font-semibold text-stone-900 disabled:bg-stone-100 disabled:text-stone-400"
                onInput={(event) => {
                  const value = Number((event.currentTarget as HTMLInputElement).value || "0");
                  onChange(idx, clampUnitAmount(value, max));
                }}
              />
            </div>
          </label>
        );
      })}
    </div>
  );
}

function RallyReinforcementActionForm({
  card,
  action,
  label,
  variant,
  unitNames,
  expanded,
  onExpandedChange,
  onSubmit,
}: {
  card: RallyCard;
  action: "recall" | "release";
  label: string;
  variant: "warning" | "secondary";
  unitNames: string[];
  expanded: boolean;
  onExpandedChange: (expanded: boolean) => void;
  onSubmit: (units: number[]) => Promise<void>;
}) {
  const [amounts, setAmounts] = useState(() => unitsFromCard(card));
  const [submitting, setSubmitting] = useState(false);
  const totalSelected = amounts.reduce((sum, value) => sum + value, 0);

  useEffect(() => {
    setAmounts(unitsFromCard(card));
  }, [card.actionId, card.units.join(",")]);

  if (!expanded) {
    return (
      <Button type="button" variant={variant} size="sm" onClick={() => onExpandedChange(true)}>
        {label}
      </Button>
    );
  }

  return (
    <div class="space-y-2 rounded-md border border-stone-200 bg-stone-50 p-3">
      <div class="flex items-center justify-between gap-3">
        <div class="text-xs font-semibold uppercase text-stone-500">
          {action === "recall" ? "Recall amounts" : "Release amounts"}
        </div>
        <div class="flex items-center gap-3">
          <button
            type="button"
            class="text-xs font-semibold text-green-800 hover:underline"
            onClick={() => setAmounts(unitsFromCard(card))}
          >
            All
          </button>
          <button
            type="button"
            class="text-xs font-semibold text-stone-500 hover:text-stone-700 hover:underline"
            onClick={() => onExpandedChange(false)}
          >
            Cancel
          </button>
        </div>
      </div>

      <UnitAmountGrid
        available={card.units.map((value) => Number(value ?? 0))}
        amounts={amounts}
        unitLabels={unitNames.map((name, idx) => unitLabel(name ?? `U${idx + 1}`))}
        disabled={submitting}
        renderIcon={(idx) => (
          <UnitSprite
            tribe={card.tribe}
            unitIndex={idx}
            label={unitLabel(unitNames[idx] ?? `U${idx + 1}`)}
          />
        )}
        onChange={(idx, value) => {
          setAmounts((current) => {
            const next = [...current];
            next[idx] = value;
            return next;
          });
        }}
      />

      <Button
        type="button"
        variant={variant}
        size="sm"
        disabled={submitting || totalSelected === 0}
        onClick={async () => {
          setSubmitting(true);
          try {
            await onSubmit(amounts);
          } finally {
            setSubmitting(false);
          }
        }}
      >
        {submitting ? "Submitting..." : label}
      </Button>
    </div>
  );
}

export function RallyPointBuilding({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  const query = new URLSearchParams(window.location.search);
  const initialTargetX = Number(query.get("x") ?? "0") || 0;
  const initialTargetY = Number(query.get("y") ?? "0") || 0;
  const [targetX, setTargetX] = useState(initialTargetX);
  const [targetY, setTargetY] = useState(initialTargetY);
  const [movement, setMovement] = useState<"attack" | "raid" | "reinforcement">("attack");
  const [scoutingTarget, setScoutingTarget] = useState<"resources" | "defenses">("resources");
  const [catapultTarget1, setCatapultTarget1] = useState("MainBuilding");
  const [catapultTarget2, setCatapultTarget2] = useState("Warehouse");
  const [units, setUnits] = useState<Record<number, number>>({});
  const [preview, setPreview] = useState<MovementPreviewResponse | null>(null);
  const [previewStartedAtMs, setPreviewStartedAtMs] = useState<number | null>(null);
  const [previewTravelSeconds, setPreviewTravelSeconds] = useState(0);
  const [previewTick, setPreviewTick] = useState(0);
  const [previewing, setPreviewing] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedActionKey, setExpandedActionKey] = useState<string | null>(null);
  const sendTroops = useSendTroopsMutation();
  const recallTroops = useRecallTroopsMutation();
  const releaseReinforcements = useReleaseReinforcementsMutation();
  useEffect(() => {
    if (!preview) return;
    const timer = window.setInterval(() => setPreviewTick((v) => v + 1), 1000);
    return () => window.clearInterval(timer);
  }, [preview]);

  if (!detail.rallyPoint) return null;

  const toUnitsArray = () => {
    const arr = Array.from({ length: 10 }, (_, idx) => units[idx] ?? 0);
    return arr;
  };
  const sendableUnitAmounts = detail.rallyPoint.sendableUnits.map((unit) => units[unit.unitIdx] ?? 0);
  const sendableUnitAvailable = detail.rallyPoint.sendableUnits.map((unit) =>
    unit.isResearched ? unit.available : 0
  );
  const sendableUnitLabels = detail.rallyPoint.sendableUnits.map((unit) => unitLabel(unit.name));

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

  return (
    <>
      <Panel class="space-y-4">
        <div>
          <SectionHeader title="Send troops" class="mb-1" />
          <p class="text-sm text-stone-500">Select target and units.</p>
        </div>
        <div class="grid gap-2 rounded-md border border-stone-200 bg-stone-50 p-3 sm:grid-cols-[96px_96px_1fr]">
          <label class="text-sm text-gray-600">
            Target X
            <input type="number" value={targetX} onInput={(e) => setTargetX(Number((e.target as HTMLInputElement).value || "0"))} class="mt-1 w-full rounded border border-stone-300 bg-white px-2 py-1.5 text-gray-700" />
          </label>
          <label class="text-sm text-gray-600">
            Target Y
            <input type="number" value={targetY} onInput={(e) => setTargetY(Number((e.target as HTMLInputElement).value || "0"))} class="mt-1 w-full rounded border border-stone-300 bg-white px-2 py-1.5 text-gray-700" />
          </label>
          <label class="text-sm text-gray-600">
            Movement type
            <select value={movement} onChange={(e) => setMovement((e.target as HTMLSelectElement).value as "attack" | "raid" | "reinforcement")} class="mt-1 w-full rounded border border-stone-300 bg-white px-2 py-1.5 text-gray-700">
              <option value="attack">Attack</option>
              <option value="raid">Raid</option>
              <option value="reinforcement">Reinforcement</option>
            </select>
          </label>
        </div>
        <div class="space-y-2">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold uppercase text-stone-500">Select units</div>
            <button
              type="button"
              class="text-xs font-semibold text-stone-500 hover:text-stone-700 hover:underline"
              onClick={() => setUnits({})}
            >
              Clear
            </button>
          </div>
          <div class="rounded-md border border-stone-200 bg-stone-50 p-3">
            <UnitAmountGrid
              available={sendableUnitAvailable}
              amounts={sendableUnitAmounts}
              unitLabels={sendableUnitLabels}
              disabled={sending}
              renderIcon={(idx) => {
                const unit = detail.rallyPoint!.sendableUnits[idx];
                return (
                  <UnitSpriteByName
                    unitName={unit.name}
                    label={unitLabel(unit.name)}
                  />
                );
              }}
              onSetAll={(idx) => {
                const unit = detail.rallyPoint!.sendableUnits[idx];
                setUnits((current) => ({
                  ...current,
                  [unit.unitIdx]: unit.available,
                }));
              }}
              onChange={(idx, value) => {
                const unit = detail.rallyPoint!.sendableUnits[idx];
                setUnits((current) => ({
                  ...current,
                  [unit.unitIdx]: value,
                }));
              }}
            />
          </div>
        </div>

        <Button
          type="button"
          variant="secondary"
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
              setPreviewStartedAtMs(Date.now());
              setPreviewTravelSeconds(secondsUntilIso(result.arrivesAt));
            } catch (err) {
              setError((err as Error).message);
            } finally {
              setPreviewing(false);
            }
          }}
        >
          {previewing ? "Calculating..." : "Preview movement"}
        </Button>
        {preview ? (
          <div class="space-y-3 rounded-md border border-stone-200 bg-stone-50 p-3 text-sm">
            {(() => {
              void previewTick;
              void previewStartedAtMs;
              const dynamicArrivesAt = new Date(Date.now() + previewTravelSeconds * 1000);
              return (
                <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
                  <div class="rounded-md border border-stone-200 bg-white px-3 py-2">
                    <div class="text-[11px] font-semibold uppercase text-stone-500">Movement</div>
                    <div class="font-semibold text-stone-900">{previewMovementLabel(preview, movement)}</div>
                  </div>
                  <div class="rounded-md border border-stone-200 bg-white px-3 py-2">
                    <div class="text-[11px] font-semibold uppercase text-stone-500">Distance</div>
                    <div class="font-semibold text-stone-900">{preview.distance ?? "-"}</div>
                  </div>
                  <div class="rounded-md border border-stone-200 bg-white px-3 py-2">
                    <div class="text-[11px] font-semibold uppercase text-stone-500">Travel time</div>
                    <div class="font-semibold text-stone-900">{formatDurationHms(previewTravelSeconds)}</div>
                  </div>
                  <div class="rounded-md border border-stone-200 bg-white px-3 py-2">
                    <div class="text-[11px] font-semibold uppercase text-stone-500">Arrival</div>
                    <div class="font-semibold text-stone-900">{dynamicArrivesAt.toLocaleString()}</div>
                  </div>
                </div>
              );
            })()}
            {showScoutingTargetChoice ? (
              <div class="grid gap-2">
                <label class="text-sm text-gray-700">
                  Scouting target
                  <select
                    value={scoutingTarget}
                    onChange={(e) =>
                      setScoutingTarget((e.target as HTMLSelectElement).value as "resources" | "defenses")
                    }
                    class="mt-1 w-full rounded border border-stone-300 bg-white px-3 py-2 text-gray-700"
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
                    class="mt-1 w-full rounded border border-stone-300 bg-white px-3 py-2 text-gray-700"
                  >
                    <option value="random">Random</option>
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
                      class="mt-1 w-full rounded border border-stone-300 bg-white px-3 py-2 text-gray-700"
                    >
                      <option value="random">Random</option>
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
            <Button
              type="button"
              disabled={sending}
              onClick={async () => {
                setError(null);
                try {
                  setSending(true);
                  if (showScoutingTargetChoice && selectedScoutUnits.length === 0) {
                    throw new Error("Scout movement requires at least one scout unit.");
                  }
                  await sendTroops.mutateAsync({
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
                  window.location.assign(`/app/build/39?x=${targetX}&y=${targetY}`);
                } catch (err) {
                  setError((err as Error).message);
                } finally {
                  setSending(false);
                }
              }}
            >
              {sending ? "Sending..." : "Confirm and send"}
            </Button>
          </div>
        ) : null}
        {error ? <div class="text-sm text-red-600">{error}</div> : null}
      </Panel>

      <div class="space-y-4">
        {(["stationed", "deployed", "reinforcement", "outgoing", "incoming"] as const).map((category) => {
          const cards = detail.rallyPoint!.cards.filter((card) => card.category === category);
          if (cards.length === 0) return null;
          return (
            <div class="space-y-2" key={category}>
              <h3
                id={category === "incoming" ? "incoming" : category === "outgoing" ? "outgoing" : undefined}
                class="text-sm font-semibold text-gray-700"
              >
                {category}
              </h3>
              <div class="space-y-2">
                {cards.map((card) => {
                  const actionKey = card.action && card.actionId
                    ? `${card.action}-${card.actionId}`
                    : null;
                  const isActionEditorOpen = actionKey !== null && expandedActionKey === actionKey;

                  return (
                  <Panel key={`${category}-${card.villageId}-${card.actionId ?? "no-action"}`} class="space-y-3">
                    <div class="flex justify-between items-start">
                      <div class="flex-1">
                        <div class="flex items-center gap-2">
                          {card.villageName ? (
                            <h3 class="font-semibold text-gray-900">
                              <Link to={`/map/field/${card.villageId}`} class="text-green-700 hover:underline">
                                {card.villageName}
                              </Link>
                            </h3>
                          ) : (
                            <h3 class="font-semibold text-gray-900">Unknown Village</h3>
                          )}
                          {card.movementKind ? (
                            <Badge>{card.movementKind}</Badge>
                          ) : null}
                        </div>
                        {card.position ? (
                          <p class="text-sm text-gray-600 mt-1">
                            <Link to={`/map/field/${card.villageId}`} class="text-green-700 hover:underline">
                              ({card.position.x}, {card.position.y})
                            </Link>
                          </p>
                        ) : null}
                        <p class="text-xs text-gray-500 mt-1 inline-flex items-center gap-1">
                          <ResourceSprite kind="upkeep" size={12} label="Upkeep" />
                          {card.upkeep}
                        </p>
                        {card.arrivesAt ? (
                          <div class="mt-1 space-y-1 text-sm text-gray-500">
                            <p class="font-mono">
                              ETA{" "}
                              <LiveCountdown
                                seconds={secondsUntilIso(card.arrivesAt)}
                                onElapsed={() => {
                                  void onMutate();
                                }}
                              />
                            </p>
                            <p>Arrives at: <span class="font-mono">{new Date(card.arrivesAt).toLocaleString()}</span></p>
                          </div>
                        ) : null}
                        {card.bounty ? (
                          <p class="text-xs text-amber-700 mt-1">
                            Loot: {card.bounty.lumber}/{card.bounty.clay}/{card.bounty.iron}/{card.bounty.crop}
                          </p>
                        ) : null}
                      </div>
                      <Badge>{card.category}</Badge>
                    </div>

                    {!isActionEditorOpen ? (
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
                    ) : null}

                    {card.action === "recall" && card.actionId ? (
                      <RallyReinforcementActionForm
                        card={card}
                        action="recall"
                        label="Recall Troops"
                        variant="warning"
                        unitNames={detail.rallyPoint!.sendableUnits.map((unit) => unit.name)}
                        expanded={isActionEditorOpen}
                        onExpandedChange={(expanded) => {
                          setExpandedActionKey(expanded ? actionKey : null);
                        }}
                        onSubmit={async (selectedUnits) => {
                          setError(null);
                          try {
                            await recallTroops.mutateAsync({
                              villageId: detail.villageId,
                              armyId: card.actionId!,
                              units: selectedUnits,
                            });
                          } catch (err) {
                            const message = err instanceof Error ? err.message : "Unable to recall troops";
                            setError(message);
                          }
                        }}
                      />
                    ) : null}
                    {card.action === "release" && card.actionId ? (
                      <RallyReinforcementActionForm
                        card={card}
                        action="release"
                        label="Release Reinforcements"
                        variant="secondary"
                        unitNames={detail.rallyPoint!.sendableUnits.map((unit) => unit.name)}
                        expanded={isActionEditorOpen}
                        onExpandedChange={(expanded) => {
                          setExpandedActionKey(expanded ? actionKey : null);
                        }}
                        onSubmit={async (selectedUnits) => {
                          setError(null);
                          try {
                            await releaseReinforcements.mutateAsync({
                              villageId: card.villageId,
                              armyId: card.actionId!,
                              units: selectedUnits,
                            });
                          } catch (err) {
                            const message = err instanceof Error ? err.message : "Unable to release reinforcements";
                            setError(message);
                          }
                        }}
                      />
                    ) : null}
                  </Panel>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}
