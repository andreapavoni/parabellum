import { buildingDescriptionParagraphs, buildingLabel } from "@/lib/labels";
import { BuildingSprite } from "@/components/BuildingSprite";
import { ExpansionBuilding } from "@/components/buildings/ExpansionBuilding";
import { TrainingBuilding } from "@/components/buildings/TrainingBuilding";
import { AcademyBuilding } from "@/components/buildings/AcademyBuilding";
import { SmithyBuilding } from "@/components/buildings/SmithyBuilding";
import { MarketplaceBuilding } from "@/components/buildings/MarketplaceBuilding";
import { RallyPointBuilding } from "@/components/buildings/RallyPointBuilding";
import { MainBuilding } from "@/components/buildings/MainBuilding";
import { EmptySlotBuilding, QueuedConstructionUpgrade, UpgradeBuilding } from "@/components/buildings/buildingCards";
import type { BuildingPageResponse } from "@/types/api";

const RESERVED_BUILDING_SLOTS = new Set([19, 39, 40]);

function reservedSlotBuildingName(detail: BuildingPageResponse["detail"]): string | null {
  if (detail.buildingType !== "empty" || !RESERVED_BUILDING_SLOTS.has(detail.slotId)) return null;
  return (
    detail.emptySlot?.queuedBuildingName ??
    detail.emptySlot?.buildableBuildings[0]?.buildingName ??
    detail.emptySlot?.lockedBuildings[0]?.buildingName ??
    null
  );
}

export function BuildingPage({
  data,
  serverTimeObservedAtMs,
  onMutate,
}: {
  data: BuildingPageResponse;
  serverTimeObservedAtMs: number;
  onMutate: () => Promise<void>;
}) {
  const detail = data.detail;
  const expansion = detail.expansion;
  const query = new URLSearchParams(window.location.search);
  const initialTargetX = Number(query.get("x") ?? "0") || 0;
  const initialTargetY = Number(query.get("y") ?? "0") || 0;
  const queuedConstruction = detail.emptySlot?.hasQueueForSlot && detail.emptySlot.queuedBuildingName
    ? detail.emptySlot.queuedBuildingName
    : null;
  const showAsUnderConstruction = detail.buildingType === "empty" && queuedConstruction;
  const reservedBuildingName = reservedSlotBuildingName(detail);
  const showReservedSlot = detail.buildingType === "empty" && !showAsUnderConstruction && reservedBuildingName;
  const displayedBuildingName = showAsUnderConstruction && queuedConstruction
    ? queuedConstruction
    : showReservedSlot && reservedBuildingName
      ? reservedBuildingName
      : detail.buildingName;
  const descriptionParagraphs = displayedBuildingName
    ? buildingDescriptionParagraphs(displayedBuildingName)
    : [];

  return (
    <div class="container mx-auto p-4 max-w-6xl">
      <h1 class="text-2xl font-bold mb-4">
        {detail.buildingType === "empty" && !showAsUnderConstruction && !showReservedSlot ? (
          "Empty slot"
        ) : (
          <span class="inline-flex items-center gap-3">
            <BuildingSprite
              buildingName={displayedBuildingName}
              size={64}
              label={buildingLabel(displayedBuildingName)}
            />
            {buildingLabel(displayedBuildingName)}
            {showAsUnderConstruction ? " (Under construction)" : ` (Level ${showReservedSlot ? 0 : detail.currentLevel})`}
          </span>
        )}
      </h1>
      {descriptionParagraphs.length > 0 ? (
        <div class="mb-5 max-w-3xl space-y-2 text-sm leading-6 text-gray-600">
          {descriptionParagraphs.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
        </div>
      ) : null}

      <div class="space-y-6">
        {detail.buildingType === "empty" && !showAsUnderConstruction && !showReservedSlot ? (
          <div class="text-sm text-gray-600">Select a building to start construction.</div>
        ) : null}

        {detail.buildingType === "empty" ? <EmptySlotBuilding detail={detail} onMutate={onMutate} /> : null}

        {detail.buildingType === "expansion" && expansion ? <ExpansionBuilding expansion={expansion} /> : null}
        <TrainingBuilding detail={detail} onMutate={onMutate} />
        <AcademyBuilding detail={detail} onMutate={onMutate} />
        <SmithyBuilding detail={detail} onMutate={onMutate} />

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
        {detail.mainBuilding ? (
          <MainBuilding
            detail={detail.mainBuilding}
            serverTime={data.serverTime}
            serverTimeObservedAtMs={serverTimeObservedAtMs}
            onMutate={onMutate}
          />
        ) : null}
        {showAsUnderConstruction ? <QueuedConstructionUpgrade detail={detail} onMutate={onMutate} /> : null}
        {detail.buildingType !== "empty" ? <UpgradeBuilding data={detail} onMutate={onMutate} /> : null}
      </div>
    </div>
  );
}
