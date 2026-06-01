import type { VNode } from "preact";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { LiveCountdown } from "@/components/buildings/buildingShared";
import { secondsUntilIso } from "@/lib/time";
import { unitLabel } from "@/lib/labels";
import type { BuildingPageResponse, SmithyUpgradeOption } from "@/types/api";

export function SmithyBuilding({
  detail,
  onMutate,
  SmithyOptionCard,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
  SmithyOptionCard: (props: {
    option: SmithyUpgradeOption;
    detail: BuildingPageResponse["detail"];
    onMutate: () => Promise<void>;
  }) => VNode;
}) {
  if (detail.buildingType !== "smithy" || !detail.smithy) return null;
  return (
    <>
      {detail.smithy.queue.length > 0 ? (
        <div class="border rounded-md p-4 bg-gray-50 space-y-3">
          <div class="text-sm text-gray-500 uppercase">Upgrade queue</div>
          {detail.smithy.queue.map((job, index) => (
            <div key={`${job.unitName}-${job.targetLevel}-${index}`} class="bg-white border rounded-md p-3 text-sm space-y-1">
              <div class="flex items-center justify-between">
                <span class="font-semibold text-gray-900">
                  <span class="inline-flex items-center gap-2">
                    <UnitSpriteByName unitName={job.unitName} label={unitLabel(job.unitName)} />
                    {unitLabel(job.unitName)} to level {job.targetLevel}
                  </span>
                </span>
                <span class={job.isProcessing ? "text-xs font-semibold text-emerald-600" : "text-xs font-semibold text-gray-500"}>
                  {job.isProcessing ? "In progress" : "Pending"}
                </span>
              </div>
              <div class="flex items-center justify-between text-xs text-gray-600">
                <span class="inline-flex items-center gap-1">
                  <ResourceSprite kind="clock" size={12} label="Time remaining" />
                  Time remaining
                </span>
                <LiveCountdown
                  seconds={secondsUntilIso(job.finishesAt)}
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
    </>
  );
}
