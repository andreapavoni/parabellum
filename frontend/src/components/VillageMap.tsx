import type { BuildingSlot } from "@/types/api";
import { BuildingSprite } from "@/components/BuildingSprite";
import { buildingLabel } from "@/lib/labels";
import { navigate, shouldUseClientNavigation } from "@/lib/router";

const buildingPositions = [
  [26, 220, 260],
  [27, 340, 160],
  [28, 490, 120],
  [29, 680, 180],
  [30, 800, 300],
  [32, 860, 440],
  [33, 860, 600],
  [34, 780, 760],
  [35, 600, 860],
  [36, 350, 840],
  [37, 210, 740],
  [38, 130, 590],
  [31, 140, 420],
  [22, 380, 330],
  [20, 300, 480],
  [21, 350, 660],
  [25, 530, 260],
  [24, 640, 380],
  [23, 520, 710],
] as const;
const VILLAGE_ICON_SIZE = 128;

function title(slot?: BuildingSlot) {
  if (!slot?.buildingName) return "Empty slot";
  return `${buildingLabel(slot.buildingName)} (Level ${slot.level})`;
}

function buildingHref(slotId: number, slot?: BuildingSlot) {
  if (!slot) {
    return `/village`;
  }
  return `/app/build/${slotId}`;
}

function slotClasses(slot?: BuildingSlot, empty = false) {
  let classes = empty ? "village-node-bg village-node-empty" : "village-node-bg village-node-occupied";
  if (slot?.inQueue !== undefined) {
    classes += slot.inQueue ? " construction-active" : " construction-pending";
  }
  return classes;
}

function onMapLinkClick(event: MouseEvent, href: string) {
  if (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  ) {
    return;
  }
  if (!shouldUseClientNavigation(href)) {
    return;
  }
  event.preventDefault();
  navigate(href);
}

export function VillageMap({ slots }: { slots: BuildingSlot[] }) {
  const wallSlot = slots.find((slot) => slot.slotId === 40);
  const mainBuilding = slots.find((slot) => slot.slotId === 19);
  const rallyPoint = slots.find((slot) => slot.slotId === 39);

  return (
    <div class="village-svg-container">
      <svg viewBox="0 0 1000 1000" xmlns="http://www.w3.org/2000/svg">
        {wallSlot ? (
          <a href={buildingHref(40, wallSlot)} onClick={(event) => onMapLinkClick(event, buildingHref(40, wallSlot))}>
            <circle
              class={wallSlot.level === 0 ? "village-wall-ring village-wall-empty" : "village-wall-ring"}
              cx="500"
              cy="500"
              r="460"
              fill="none"
              stroke="#E88C30"
              stroke-width="18"
            />
            <title>{title(wallSlot)}</title>
          </a>
        ) : null}

        {rallyPoint ? (
          <a href={buildingHref(39, rallyPoint)} onClick={(event) => onMapLinkClick(event, buildingHref(39, rallyPoint))}>
            <g transform="rotate(-30, 500, 500)">
              <path
                class="village-radar-zone"
                d="M 535 778 A 280 280 0 0 0 765 605 L 588 541 A 120 120 0 0 1 512 618 Z"
                fill="rgba(74, 122, 41, 0.25)"
                stroke="#4a7a29"
                stroke-width="3"
                stroke-dasharray="10, 8"
              />
              {(() => {
                return rallyPoint.buildingName ? (
                  <foreignObject x="552" y="595" width={VILLAGE_ICON_SIZE} height={VILLAGE_ICON_SIZE}>
                    <div class="pointer-events-none">
                      <BuildingSprite
                        buildingName={rallyPoint.buildingName}
                        size={VILLAGE_ICON_SIZE}
                        label={buildingLabel(rallyPoint.buildingName!)}
                      />
                    </div>
                  </foreignObject>
                ) : null;
              })()}
            </g>
            <title>{title(rallyPoint)}</title>
          </a>
        ) : null}

        {buildingPositions.map(([slotId, cx, cy]) => {
          const slot = slots.find((item) => item.slotId === slotId);
          const isEmpty = !slot?.buildingName;
          return (
            <a
              href={buildingHref(slotId, slot)}
              key={slotId}
              onClick={(event) => onMapLinkClick(event, buildingHref(slotId, slot))}
            >
              <g class="village-node-group">
                {isEmpty ? (
                  <circle
                    class={slotClasses(slot, true)}
                    cx={cx}
                    cy={cy}
                    r="55"
                    stroke-width="2"
                    stroke-dasharray="6,4"
                    opacity="0.6"
                  />
                ) : null}
                {!isEmpty && slot?.buildingName ? (
                  <foreignObject x={cx - 64} y={cy - 84} width={VILLAGE_ICON_SIZE} height={VILLAGE_ICON_SIZE}>
                    <div class="pointer-events-none">
                      <BuildingSprite
                        buildingName={slot.buildingName}
                        size={VILLAGE_ICON_SIZE}
                        label={buildingLabel(slot!.buildingName!)}
                      />
                    </div>
                  </foreignObject>
                ) : null}
                {isEmpty ? (
                  <text
                    x={cx}
                    y={cy}
                    dy="0.35em"
                    text-anchor="middle"
                    font-weight="bold"
                    font-size="28"
                    fill="#3e2b18"
                  >
                    -
                  </text>
                ) : null}
                <title>{title(slot)}</title>
              </g>
            </a>
          );
        })}

        {mainBuilding ? (
          <a href={buildingHref(19, mainBuilding)} onClick={(event) => onMapLinkClick(event, buildingHref(19, mainBuilding))}>
            <g id="village-main-node">
              <circle cx="500" cy="520" r="65" fill="none" stroke="white" stroke-width="5" opacity="0.8" />
              <circle cx="500" cy="520" r="55" fill="#EDF4E1" />
              <text
                x="500"
                y="520"
                dy="0.35em"
                text-anchor="middle"
                font-family="Arial, sans-serif"
                font-weight="900"
                font-size="32"
                fill="#1a3a10"
              >
              </text>
              {(() => {
                return mainBuilding.buildingName ? (
                  <foreignObject x="436" y="420" width={VILLAGE_ICON_SIZE} height={VILLAGE_ICON_SIZE}>
                    <div class="pointer-events-none">
                      <BuildingSprite
                        buildingName={mainBuilding.buildingName}
                        size={VILLAGE_ICON_SIZE}
                        label={buildingLabel(mainBuilding.buildingName!)}
                      />
                    </div>
                  </foreignObject>
                ) : null;
              })()}
              <title>{title(mainBuilding)}</title>
            </g>
          </a>
        ) : null}
      </svg>
    </div>
  );
}
