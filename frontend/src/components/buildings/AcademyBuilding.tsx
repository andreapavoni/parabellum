import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSpriteByName } from "@/components/UnitSprite";
import { LiveCountdown } from "@/components/buildings/buildingShared";
import { AcademyOptionCard } from "@/components/buildings/buildingCards";
import { secondsUntilIso } from "@/lib/time";
import { unitLabel } from "@/lib/labels";
import type { BuildingPageResponse } from "@/types/api";

export function AcademyBuilding({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  if (detail.buildingType !== "academy" || !detail.academy) return null;
  return (
    <>
      {detail.academy.queue.length > 0 ? (
        <div class="border rounded-md p-4 bg-gray-50 space-y-3">
          <div class="text-sm text-gray-500 uppercase">Research queue</div>
          {detail.academy.queue.map((job, index) => (
            <div key={`${job.unitName}-${index}`} class="bg-white border rounded-md p-3 text-sm space-y-1">
              <div class="flex items-center justify-between">
                <span class="inline-flex items-center gap-2 font-semibold text-gray-900">
                  <UnitSpriteByName unitName={job.unitName} label={unitLabel(job.unitName)} />
                  {unitLabel(job.unitName)}
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
  );
}
