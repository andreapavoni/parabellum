import type { ResourceSlot } from "@/types/api";
import { buildingLabel } from "@/lib/labels";

const hexPositions = [
  [1, 279, 190],
  [2, 400, 190],
  [3, 521, 190],
  [4, 218, 295],
  [5, 339, 295],
  [6, 460, 295],
  [7, 581, 295],
  [8, 157, 400],
  [9, 278, 400],
  [10, 521, 400],
  [11, 642, 400],
  [12, 218, 505],
  [13, 339, 505],
  [14, 460, 505],
  [15, 581, 505],
  [16, 279, 610],
  [17, 400, 610],
  [18, 521, 610],
] as const;

function hexColor(buildingName: string) {
  switch (buildingName) {
    case "Woodcutter":
      return "#6c9a35";
    case "ClayPit":
      return "#d98536";
    case "IronMine":
      return "#999999";
    case "Cropland":
      return "#f2d649";
    default:
      return "#6c9a35";
  }
}

export function ResourceFieldsMap({ slots }: { slots: ResourceSlot[] }) {
  return (
    <div class="resource-fields-svg-container">
      <svg viewBox="0 200 800 600" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <polygon
            id="hex-shape"
            points="0,-70 60.62,-35 60.62,35 0,70 -60.62,35 -60.62,-35"
            stroke="white"
            stroke-width="3"
          />
        </defs>
        {hexPositions.map(([slotId, tx, ty]) => {
          const slot = slots.find((item) => item.slotId === slotId);
          if (!slot) {
            return null;
          }
          const hasConstruction = slot.inQueue !== undefined;
          const isProcessing = Boolean(slot.inQueue);
          return (
            <a href={`/app/build/${slotId}`} key={slotId}>
              <g
                class={
                  hasConstruction
                    ? isProcessing
                      ? "resource-hex-group construction-active"
                      : "resource-hex-group construction-pending"
                    : "resource-hex-group"
                }
                transform={`translate(${tx}, ${ty})`}
              >
                <use href="#hex-shape" fill={hexColor(slot.buildingName)} stroke="none" />
                <text x="0" y="5" text-anchor="middle">
                  {slot.level}
                </text>
                <title>
                  {buildingLabel(slot.buildingName)} (Level {slot.level})
                </title>
              </g>
            </a>
          );
        })}
        <a href="/village">
          <g class="resource-city-center" transform="translate(400, 400)">
            <circle cx="0" cy="0" r="68" fill="white" />
            <circle class="resource-main-circle" cx="0" cy="0" r="62" fill="#5c192d" />
            <title>Village Center</title>
          </g>
        </a>
      </svg>
    </div>
  );
}
