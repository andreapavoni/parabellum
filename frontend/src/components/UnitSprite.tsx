const ICON_SIZE = 16;
const ICON_STRIDE = 19;

function spritePathForTribe(tribe?: string) {
  const normalized = String(tribe ?? "").toLowerCase();
  if (normalized === "roman") return "/static/units/romans.gif";
  if (normalized === "gaul") return "/static/units/gauls.gif";
  if (normalized === "teuton") return "/static/units/teutons.gif";
  if (normalized === "nature") return "/static/units/nature.gif";
  if (normalized === "natar") return "/static/units/natars.gif";
  return null;
}

type SpriteMeta = { tribe: string; unitIndex: number };

const UNIT_SPRITE_BY_NAME: Record<string, SpriteMeta> = {
  Legionnaire: { tribe: "Roman", unitIndex: 0 },
  Praetorian: { tribe: "Roman", unitIndex: 1 },
  Imperian: { tribe: "Roman", unitIndex: 2 },
  EquitesLegati: { tribe: "Roman", unitIndex: 3 },
  EquitesImperatoris: { tribe: "Roman", unitIndex: 4 },
  EquitesCaesaris: { tribe: "Roman", unitIndex: 5 },
  BatteringRam: { tribe: "Roman", unitIndex: 6 },
  FireCatapult: { tribe: "Roman", unitIndex: 7 },
  Senator: { tribe: "Roman", unitIndex: 8 },
  Settler: { tribe: "Roman", unitIndex: 9 },
  Maceman: { tribe: "Teuton", unitIndex: 0 },
  Spearman: { tribe: "Teuton", unitIndex: 1 },
  Axeman: { tribe: "Teuton", unitIndex: 2 },
  Scout: { tribe: "Teuton", unitIndex: 3 },
  Paladin: { tribe: "Teuton", unitIndex: 4 },
  TeutonicKnight: { tribe: "Teuton", unitIndex: 5 },
  Ram: { tribe: "Teuton", unitIndex: 6 },
  Catapult: { tribe: "Teuton", unitIndex: 7 },
  Chief: { tribe: "Teuton", unitIndex: 8 },
  Phalanx: { tribe: "Gaul", unitIndex: 0 },
  Swordsman: { tribe: "Gaul", unitIndex: 1 },
  Pathfinder: { tribe: "Gaul", unitIndex: 2 },
  TheutatesThunder: { tribe: "Gaul", unitIndex: 3 },
  Druidrider: { tribe: "Gaul", unitIndex: 4 },
  Haeduan: { tribe: "Gaul", unitIndex: 5 },
  Trebuchet: { tribe: "Gaul", unitIndex: 7 },
  Chieftain: { tribe: "Gaul", unitIndex: 8 },
};

export function spriteMetaForUnitName(unitName: string): SpriteMeta | null {
  return UNIT_SPRITE_BY_NAME[unitName] ?? null;
}

export function UnitSprite({
  tribe,
  unitIndex,
  label,
}: {
  tribe?: string;
  unitIndex: number;
  label?: string;
}) {
  const src = spritePathForTribe(tribe);
  if (!src || unitIndex < 0) {
    return <span class="inline-flex h-4 w-4 items-center justify-center text-[10px] text-gray-400">?</span>;
  }

  return (
    <span
      title={label ?? `Unit ${unitIndex + 1}`}
      class="inline-block align-middle"
      style={{
        width: `${ICON_SIZE}px`,
        height: `${ICON_SIZE}px`,
        backgroundImage: `url(${src})`,
        backgroundRepeat: "no-repeat",
        backgroundPosition: `-${unitIndex * ICON_STRIDE}px 0px`,
      }}
    />
  );
}

export function UnitSpriteByName({ unitName, label }: { unitName: string; label?: string }) {
  const meta = spriteMetaForUnitName(unitName);
  if (!meta) {
    return <span class="inline-flex h-4 w-4 items-center justify-center text-[10px] text-gray-400">?</span>;
  }
  return (
    <UnitSprite
      tribe={meta.tribe}
      unitIndex={meta.unitIndex}
      label={label ?? unitName}
    />
  );
}
