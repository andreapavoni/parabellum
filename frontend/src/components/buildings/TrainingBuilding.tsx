import { ResourceSprite } from "@/components/ResourceSprite";
import { LiveCountdown } from "@/components/buildings/buildingShared";
import { TrainingUnitCard } from "@/components/buildings/buildingCards";
import { formatDurationHms, secondsUntilIso } from "@/lib/time";
import { unitLabel } from "@/lib/labels";
import type { BuildingPageResponse } from "@/types/api";

export function TrainingBuilding({
  detail,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  onMutate: () => Promise<void>;
}) {
  if ((detail.buildingType !== "training" && detail.buildingType !== "expansion") || !detail.training) return null;
  return (
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
                  Training time {formatDurationHms(job.timePerUnit)}
                </span>
              </div>
              <div class="flex items-center justify-between text-xs text-gray-600">
                <span class="inline-flex items-center gap-1">
                  <ResourceSprite kind="clock" size={12} label="Time remaining" />
                  Remaining
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
    </>
  );
}
