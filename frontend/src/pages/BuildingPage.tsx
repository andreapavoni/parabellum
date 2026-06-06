import { buildingLabel } from "@/lib/labels";
import { BuildingSprite } from "@/components/BuildingSprite";
import { ExpansionBuilding } from "@/components/buildings/ExpansionBuilding";
import { TrainingBuilding } from "@/components/buildings/TrainingBuilding";
import { AcademyBuilding } from "@/components/buildings/AcademyBuilding";
import { SmithyBuilding } from "@/components/buildings/SmithyBuilding";
import { MarketplaceBuilding } from "@/components/buildings/MarketplaceBuilding";
import { RallyPointBuilding } from "@/components/buildings/RallyPointBuilding";
import { EmptySlotBuilding, QueuedConstructionUpgrade, UpgradeBuilding } from "@/components/buildings/buildingCards";
import type { BuildingPageResponse } from "@/types/api";

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
  const initialTargetX = Number(query.get("x") ?? "0") || 0;
  const initialTargetY = Number(query.get("y") ?? "0") || 0;
  const queuedConstruction = detail.emptySlot?.hasQueueForSlot && detail.emptySlot.queuedBuildingName
    ? detail.emptySlot.queuedBuildingName
    : null;
  const showAsUnderConstruction = detail.buildingType === "empty" && queuedConstruction;

  return (
    <div class="container mx-auto p-4 max-w-6xl">
      <h1 class="text-2xl font-bold mb-4">
        {detail.buildingType === "empty" && !showAsUnderConstruction ? (
          `Empty slot #${detail.slotId}`
        ) : (
          <span class="inline-flex items-center gap-3">
            <BuildingSprite
              buildingName={showAsUnderConstruction ? queuedConstruction : detail.buildingName}
              size={64}
              label={buildingLabel(showAsUnderConstruction ? queuedConstruction : detail.buildingName)}
            />
            {buildingLabel(showAsUnderConstruction ? queuedConstruction : detail.buildingName)}
            {showAsUnderConstruction ? " (Under construction)" : ` (Level ${detail.currentLevel})`}
          </span>
        )}
      </h1>

      <div class="space-y-6">
        {detail.buildingType === "empty" && !showAsUnderConstruction ? (
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
        {showAsUnderConstruction ? <QueuedConstructionUpgrade detail={detail} onMutate={onMutate} /> : null}
        {detail.buildingType !== "empty" ? <UpgradeBuilding data={detail} onMutate={onMutate} /> : null}
      </div>
    </div>
  );
}
