import { UnitSprite } from "@/components/UnitSprite";

function normalizeUnits(units: number[]) {
  return Array.from({ length: Math.max(10, units.length) }, (_, idx) => Number(units[idx] ?? 0));
}

function HeroMarker() {
  return (
    <span
      title="Hero"
      class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-amber-300 bg-amber-50 text-[11px] font-bold text-amber-800"
    >
      H
    </span>
  );
}

export function ArmyTable({
  title,
  units,
  losses,
  tribe,
  hasHero = false,
  heroLost = false,
  showEmpty = false,
  showUpkeep,
}: {
  title?: string;
  units: number[];
  losses?: number[];
  tribe?: string;
  hasHero?: boolean;
  heroLost?: boolean;
  showEmpty?: boolean;
  showUpkeep?: number;
}) {
  const normalizedUnits = normalizeUnits(units);
  const normalizedLosses = losses ? normalizeUnits(losses) : undefined;
  const hasContent = normalizedUnits.some((value) => value > 0) || hasHero || heroLost;
  if (!hasContent && !showEmpty) return null;

  return (
    <div class="rounded-md border border-stone-200 bg-stone-50 p-2">
      {title ? <p class="mb-1 text-[11px] font-semibold uppercase text-stone-500">{title}</p> : null}
      <div class="overflow-x-auto">
        <table class="w-full table-fixed border-collapse overflow-hidden rounded-md border border-stone-200 bg-white text-xs">
          <thead>
            <tr>
              {normalizedUnits.map((_, idx) => (
                <th key={`u-${idx}`} class="border-b border-stone-200 p-0.5 text-center text-stone-500">
                  <UnitSprite tribe={tribe} unitIndex={idx} label={`U${idx + 1}`} />
                </th>
              ))}
              <th class="border-b border-stone-200 p-0.5 text-center text-stone-500">
                <HeroMarker />
              </th>
            </tr>
          </thead>
          <tbody>
            <tr>
              {normalizedUnits.map((value, idx) => (
                <td
                  key={`before-${idx}`}
                  class={`border-r border-stone-200 p-1 text-center ${value === 0 ? "bg-stone-50 opacity-50" : "bg-white"}`}
                >
                  <div class={value === 0 ? "text-stone-400" : "font-semibold text-stone-900"}>{value}</div>
                </td>
              ))}
              <td class={`p-1 text-center ${hasHero ? "bg-white" : "bg-stone-50 opacity-50"}`}>
                <div class={hasHero ? "font-semibold text-stone-900" : "text-stone-400"}>{hasHero ? 1 : 0}</div>
              </td>
            </tr>
            {normalizedLosses ? (
              <tr>
                {normalizedLosses.map((loss, idx) => (
                  <td key={`loss-${idx}`} class="border-r border-stone-200 bg-stone-100 p-1 text-center">
                    <div class={loss > 0 ? "font-semibold text-stone-700" : "text-stone-300"}>
                      {loss > 0 ? `↓${loss}` : "-"}
                    </div>
                  </td>
                ))}
                <td class="bg-stone-100 p-1 text-center">
                  <div class={heroLost ? "font-semibold text-stone-700" : "text-stone-300"}>
                    {heroLost ? "↓1" : "-"}
                  </div>
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
      {showUpkeep !== undefined ? (
        <div class="mt-2 flex justify-end text-xs text-stone-500">{showUpkeep}</div>
      ) : null}
    </div>
  );
}
