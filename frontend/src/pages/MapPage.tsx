import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { api } from "@/lib/api";
import { navigate } from "@/lib/router";
import { ResourceSprite } from "@/components/ResourceSprite";
import type { MapTile, MapRegionResponse } from "@/types/api";

type MapPageProps = {
  worldSize: number;
  initialCenterX?: number;
  initialCenterY?: number;
  homeVillageId?: number;
  homeX?: number;
  homeY?: number;
};

type HoveredTile = {
  tile: MapTile | null;
  x: number;
  y: number;
  title: string;
  bonuses?: Array<{ kind: "lumber" | "clay" | "iron" | "crop"; percent: number }>;
  left: number;
  top: number;
};

function wrapCoordinate(value: number, worldSize: number) {
  const span = worldSize * 2 + 1;
  const normalized = ((value + worldSize) % span + span) % span;
  return normalized - worldSize;
}

function tileVisual(tile: MapTile | null, isHome: boolean) {
  if (!tile) {
    return {
      typeClass: "",
      title: "Unknown",
      oasisBg: "",
    };
  }

  if (tile.villageId) {
    return {
      typeClass: isHome ? "is-own-village" : "is-village",
      title: `${tile.villageName ?? "Village"}${tile.isCapital ? " (Capital)" : ""}`,
      oasisBg: "",
    };
  }

  if (tile.tileType === "oasis") {
    const oasis = (tile.oasis ?? "oasis").toLowerCase();
    const oasisBg = oasis.includes("lumber")
      ? "#c5e1a5"
      : oasis.includes("clay")
        ? "#ffe0b2"
        : oasis.includes("iron")
          ? "#e0e0e0"
          : "#fff9c4";
    return {
      typeClass: `oasis-${oasis.replace(/[^a-z0-9]/g, "-")}`,
      title: "Oasis",
      oasisBg,
    };
  }

  const valley = tile.valley ? `${tile.valley.lumber}-${tile.valley.clay}-${tile.valley.iron}-${tile.valley.crop}` : "";
  return {
    typeClass: "",
    title: valley ? `Valley ${valley}` : "Valley",
    oasisBg: "",
  };
}

function oasisIconUrls(tile: MapTile | null): string[] {
  if (!tile || tile.tileType !== "oasis") return [];
  const oasis = (tile.oasis ?? "").toLowerCase();
  if (oasis === "crop50" || (oasis.includes("crop") && oasis.includes("50") && !oasis.includes("lumber") && !oasis.includes("clay") && !oasis.includes("iron"))) {
    return [
      "/static/misc/buildings/cropland.png",
      "/static/misc/buildings/cropland.png",
    ];
  }
  const icons: string[] = [];
  if (oasis.includes("lumber")) icons.push("/static/misc/buildings/woodcutter.png");
  if (oasis.includes("clay")) icons.push("/static/misc/buildings/clay_pit.png");
  if (oasis.includes("iron")) icons.push("/static/misc/buildings/iron_mine.png");
  if (oasis.includes("crop")) icons.push("/static/misc/buildings/cropland.png");
  return icons.length > 0 ? icons.slice(0, 2) : ["/static/misc/buildings/cropland.png"];
}

function oasisBonuses(tile: MapTile | null): Array<{ kind: "lumber" | "clay" | "iron" | "crop"; percent: number }> {
  if (!tile || tile.tileType !== "oasis") return [];
  const oasis = (tile.oasis ?? "").toLowerCase();
  const isFifty = oasis.includes("50");
  const percent = isFifty ? 50 : 25;
  const result: Array<{ kind: "lumber" | "clay" | "iron" | "crop"; percent: number }> = [];
  if (oasis.includes("lumber")) result.push({ kind: "lumber", percent });
  if (oasis.includes("clay")) result.push({ kind: "clay", percent });
  if (oasis.includes("iron")) result.push({ kind: "iron", percent });
  if (oasis.includes("crop")) result.push({ kind: "crop", percent });
  return result;
}

function detailsPosition(tileEl: SVGGElement, containerEl: HTMLDivElement) {
  const containerRect = containerEl.getBoundingClientRect();
  const tileRect = tileEl.getBoundingClientRect();
  const panelWidth = 240;
  const panelHeight = 220;
  const offset = 12;
  const maxLeft = Math.max(offset, containerRect.width - panelWidth - offset);
  const maxTop = Math.max(offset, containerRect.height - panelHeight - offset);

  let left = tileRect.left - containerRect.left + tileRect.width + offset;
  let top = tileRect.top - containerRect.top;

  if (left > maxLeft) {
    left = tileRect.left - containerRect.left - panelWidth - offset;
  }

  return {
    left: Math.max(offset, Math.min(left, maxLeft)),
    top: Math.max(offset, Math.min(top, maxTop)),
  };
}

export function MapPage({
  worldSize,
  initialCenterX,
  initialCenterY,
  homeVillageId,
  homeX,
  homeY,
}: MapPageProps) {
  const [region, setRegion] = useState<MapRegionResponse | null>(null);
  const [hovered, setHovered] = useState<HoveredTile | null>(null);
  const [xInput, setXInput] = useState("");
  const [yInput, setYInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [moving, setMoving] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;

    async function bootstrap() {
      setLoading(true);
      setError(null);
      try {
        const initialRegion = Number.isFinite(initialCenterX) && Number.isFinite(initialCenterY)
          ? await api.mapRegion({
              x: wrapCoordinate(initialCenterX as number, worldSize),
              y: wrapCoordinate(initialCenterY as number, worldSize),
            })
          : await api.mapRegion();
        if (!alive) return;
        setRegion(initialRegion);
        setXInput(String(initialRegion.center.x));
        setYInput(String(initialRegion.center.y));
      } catch (err) {
        if (alive) setError((err as Error).message);
      } finally {
        if (alive) setLoading(false);
      }
    }

    bootstrap();
    return () => {
      alive = false;
    };
  }, [initialCenterX, initialCenterY, worldSize]);

  async function loadRegion(params: { x?: number; y?: number }) {
    setMoving(true);
    setError(null);
    setHovered(null);
    try {
      const next = await api.mapRegion(params);
      setRegion(next);
      setXInput(String(next.center.x));
      setYInput(String(next.center.y));
      window.history.replaceState(null, "", `/map?x=${next.center.x}&y=${next.center.y}`);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setMoving(false);
    }
  }

  const data = useMemo(() => {
    if (!region) return null;
    const gridSize = region.radius * 2 + 1;
    const lookup = new Map(region.tiles.map((tile) => [`${tile.x}:${tile.y}`, tile]));
    const rows: Array<Array<MapTile | null>> = [];
    for (let row = 0; row < gridSize; row += 1) {
      const y = region.center.y + region.radius - row;
      const currentRow = [];
      for (let col = 0; col < gridSize; col += 1) {
        const x = region.center.x - region.radius + col;
        const wrappedX = wrapCoordinate(x, worldSize);
        const wrappedY = wrapCoordinate(y, worldSize);
        currentRow.push(lookup.get(`${wrappedX}:${wrappedY}`) ?? null);
      }
      rows.push(currentRow);
    }

    const xAxis = Array.from({ length: gridSize }, (_, idx) => {
      const x = region.center.x - region.radius + idx;
      return wrapCoordinate(x, worldSize);
    });
    const yAxis = Array.from({ length: gridSize }, (_, idx) => {
      const y = region.center.y + region.radius - idx;
      return wrapCoordinate(y, worldSize);
    });

    return { rows, xAxis, yAxis, gridSize };
  }, [region, worldSize]);

  if (loading) {
    return <div class="mx-auto max-w-5xl px-4 py-6 text-sm text-gray-500">Loading map...</div>;
  }

  if (error || !region || !data) {
    return <div class="mx-auto max-w-5xl px-4 py-6 text-sm text-red-700">{error ?? "Unable to load the map."}</div>;
  }

  const mapSize = 1500;
  const cellSize = mapSize / data.gridSize;
  return (
    <div class="container mx-auto mt-4 md:mt-6 px-2 md:px-4 pb-12">
      <div class="map-container-main relative w-full mx-auto" ref={containerRef}>
        <div class="flex flex-col md:flex-row justify-between items-center w-full max-w-[840px] mb-4 px-2 md:pl-4 mx-auto">
          <h1 class="text-xl font-bold text-left w-full md:w-auto">
            Map <span id="header-coords" class="text-gray-700">({region.center.x}|{region.center.y})</span>
          </h1>
        </div>

        <div class="map-layout">
          <div class="axis-y">
            {data.yAxis.map((y) => (
              <div key={`y-${y}`} class={`y-label ${y === region.center.y ? "highlight-axis" : ""}`}>
                {y}
              </div>
            ))}
          </div>

          <div class="map-center">
            <button
              class="nav-overlay nav-n"
              title="North"
              onClick={() => loadRegion({ x: region.center.x, y: wrapCoordinate(region.center.y + 1, worldSize) })}
              disabled={moving}
            />
            <button
              class="nav-overlay nav-s"
              title="South"
              onClick={() => loadRegion({ x: region.center.x, y: wrapCoordinate(region.center.y - 1, worldSize) })}
              disabled={moving}
            />
            <button
              class="nav-overlay nav-w"
              title="West"
              onClick={() => loadRegion({ x: wrapCoordinate(region.center.x - 1, worldSize), y: region.center.y })}
              disabled={moving}
            />
            <button
              class="nav-overlay nav-e"
              title="East"
              onClick={() => loadRegion({ x: wrapCoordinate(region.center.x + 1, worldSize), y: region.center.y })}
              disabled={moving}
            />

            <svg id="map-svg" class="map-svg" viewBox={`0 0 ${mapSize} ${mapSize}`} preserveAspectRatio="none">
              <defs>
                <pattern id="gridPattern" width={cellSize} height={cellSize} patternUnits="userSpaceOnUse">
                  <rect width={cellSize} height={cellSize} fill="none" stroke="#9ACD32" strokeWidth={2} opacity={0.3} />
                </pattern>
              </defs>
              <rect width={mapSize} height={mapSize} fill="url(#gridPattern)" />

              {data.rows.map((row, rowIdx) =>
                row.map((tile, colIdx) => {
                  const x = region.center.x - region.radius + colIdx;
                  const y = region.center.y + region.radius - rowIdx;
                  const wrappedX = wrapCoordinate(x, worldSize);
                  const wrappedY = wrapCoordinate(y, worldSize);
                  const tx = colIdx * cellSize;
                  const ty = rowIdx * cellSize;
                  const isHome =
                    (homeVillageId && tile?.villageId === homeVillageId) ||
                    (homeX === wrappedX && homeY === wrappedY && tile?.villageId != null);
                  const visual = tileVisual(tile, Boolean(isHome));

                  return (
                    <g
                      key={`${wrappedX}:${wrappedY}`}
                      class="map-tile"
                      transform={`translate(${tx}, ${ty})`}
                      onMouseEnter={(event) => {
                        if (!containerRef.current) return;
                        const position = detailsPosition(event.currentTarget as SVGGElement, containerRef.current);
                        setHovered({
                          tile,
                          x: wrappedX,
                          y: wrappedY,
                          title: visual.title,
                          bonuses: oasisBonuses(tile),
                          left: position.left,
                          top: position.top,
                        });
                      }}
                      onMouseLeave={() => setHovered(null)}
                      onClick={() => {
                        if (!tile) return;
                        navigate(`/map/field/${tile.fieldId}`);
                      }}
                    >
                      <rect class="hover-bg" width={cellSize} height={cellSize} fill="transparent" />
                      {visual.oasisBg ? <rect width={cellSize} height={cellSize} fill={visual.oasisBg} /> : null}
                      {tile?.villageId ? (
                        <rect
                          x={cellSize * 0.06}
                          y={cellSize * 0.06}
                          width={cellSize * 0.88}
                          height={cellSize * 0.88}
                          fill="none"
                          stroke={isHome ? "orange" : "green"}
                          strokeWidth={6}
                        />
                      ) : null}
                      {tile?.villageId ? (
                        <foreignObject
                          x={cellSize * 0.12}
                          y={cellSize * 0.12}
                          width={cellSize * 0.76}
                          height={cellSize * 0.76}
                          pointerEvents="none"
                        >
                          <div
                            style={{
                              width: "100%",
                              height: "100%",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              fontSize: `${cellSize * 0.62}px`,
                              lineHeight: "1",
                            }}
                          >
                            <span>🏠</span>
                          </div>
                        </foreignObject>
                      ) : null}
                      {tile?.tileType === "oasis" ? (
                        (() => {
                          const icons = oasisIconUrls(tile);
                          const bonuses = oasisBonuses(tile);
                          const isOnly25Bonus =
                            bonuses.length > 0 && bonuses.every((bonus) => bonus.percent === 25);
                          if (icons.length === 1) {
                            const size = isOnly25Bonus ? cellSize * 0.66 : cellSize * 0.82;
                            return (
                              <image
                                href={icons[0]}
                                x={(cellSize - size) / 2}
                                y={(cellSize - size) / 2}
                                width={size}
                                height={size}
                                preserveAspectRatio="xMidYMid meet"
                                style={{ imageRendering: "pixelated" }}
                              />
                            );
                          }
                          const size = isOnly25Bonus ? cellSize * 0.38 : cellSize * 0.44;
                          const gap = cellSize * 0.015;
                          const total = size * 2 + gap;
                          const startX = (cellSize - total) / 2;
                          const y = (cellSize - size) / 2;
                          return (
                            <>
                              <image
                                href={icons[0]}
                                x={startX}
                                y={y}
                                width={size}
                                height={size}
                                preserveAspectRatio="xMidYMid meet"
                                style={{ imageRendering: "pixelated" }}
                              />
                              <image
                                href={icons[1]}
                                x={startX + size + gap}
                                y={y}
                                width={size}
                                height={size}
                                preserveAspectRatio="xMidYMid meet"
                                style={{ imageRendering: "pixelated" }}
                              />
                            </>
                          );
                        })()
                      ) : null}
                      {tile?.villageId && tile.isCapital ? (
                        <text
                          x={cellSize * 0.82}
                          y={cellSize * 0.22}
                          textAnchor="middle"
                          dominantBaseline="central"
                          fontSize={cellSize * 0.24}
                          pointerEvents="none"
                        >
                          👑
                        </text>
                      ) : null}
                    </g>
                  );
                }),
              )}
            </svg>
          </div>

          <div class="axis-x">
            {data.xAxis.map((x) => (
              <div key={`x-${x}`} class={`x-label ${x === region.center.x ? "highlight-axis" : ""}`}>
                {x}
              </div>
            ))}
          </div>
        </div>

        <div class="coords-input-container z-20">
          <span class="font-bold text-sm text-gray-700">x</span>
          <input
            type="text"
            id="input-x"
            value={xInput}
            class="w-12 p-1.5 border border-gray-300 rounded text-center text-sm outline-none focus:border-green-500 font-semibold"
            onInput={(event) => setXInput((event.target as HTMLInputElement).value)}
          />
          <span class="font-bold text-sm text-gray-700">y</span>
          <input
            type="text"
            id="input-y"
            value={yInput}
            class="w-12 p-1.5 border border-gray-300 rounded text-center text-sm outline-none focus:border-green-500 font-semibold"
            onInput={(event) => setYInput((event.target as HTMLInputElement).value)}
          />
          <button
            class="bg-gray-100 hover:bg-gray-200 border border-gray-300 px-4 py-1.5 rounded text-xs font-bold text-green-700 ml-3 cursor-pointer shadow-sm transition-colors disabled:opacity-60"
            onClick={() => {
              const parsedX = Number.parseInt(xInput, 10);
              const parsedY = Number.parseInt(yInput, 10);
              if (!Number.isFinite(parsedX) || !Number.isFinite(parsedY)) return;
              loadRegion({
                x: wrapCoordinate(parsedX, worldSize),
                y: wrapCoordinate(parsedY, worldSize),
              });
            }}
            disabled={moving}
          >
            OK
          </button>
        </div>

        {hovered ? (
          <div class="details-panel" style={{ left: `${hovered.left}px`, top: `${hovered.top}px` }}>
            <div class="text-center mb-4">
              <div class="text-xs font-semibold uppercase tracking-wide text-gray-500 mb-1">
                {hovered.tile?.tileType ?? "field"}
              </div>
              <div class="font-bold text-sm text-gray-800">{hovered.title}</div>
              {hovered.bonuses && hovered.bonuses.length > 0 ? (
                <div class="text-xs text-gray-700 mt-1 flex flex-wrap items-center justify-center gap-2">
                  {hovered.bonuses.map((bonus, idx) => (
                    <span key={`${bonus.kind}-${idx}`} class="inline-flex items-center justify-center gap-1 min-w-[56px]">
                      <ResourceSprite kind={bonus.kind} size={14} />
                      <span>+{bonus.percent}%</span>
                    </span>
                  ))}
                </div>
              ) : null}
              <div class="text-xs text-gray-500 mt-1">
                <span class="font-mono font-bold text-black">
                  {hovered.x}|{hovered.y}
                </span>
              </div>
            </div>
            <table class="w-full text-xs">
              <tbody>
                <tr class="border-b border-gray-200">
                  <td class="py-2 text-gray-600">Player</td>
                  <td class="py-2 text-right font-bold text-black">{hovered.tile?.playerName ?? "-"}</td>
                </tr>
                <tr class="border-b border-gray-200">
                  <td class="py-2 text-gray-600">Population</td>
                  <td class="py-2 text-right font-bold text-black">{hovered.tile?.villagePopulation ?? "-"}</td>
                </tr>
                <tr class="border-b border-gray-200">
                  <td class="py-2 text-gray-600">Capital</td>
                  <td class="py-2 text-right font-bold text-black">{hovered.tile?.isCapital ? "Yes" : "-"}</td>
                </tr>
                <tr>
                  <td class="py-2 text-gray-600">Tribe</td>
                  <td class="py-2 text-right font-bold text-black">{hovered.tile?.tribe ?? "-"}</td>
                </tr>
              </tbody>
            </table>
          </div>
        ) : null}
      </div>
    </div>
  );
}
