const RES_ICON_SIZE = 32;
const RES_STRIDE_Y = 40;
const RES_SPRITE_WIDTH = 36;
const RES_SPRITE_HEIGHT = 280;

export type ResourceSpriteKind =
  | "lumber"
  | "clay"
  | "iron"
  | "crop"
  | "upkeep"
  | "clock";

function indexForKind(kind: ResourceSpriteKind): number {
  switch (kind) {
    case "lumber":
      return 0;
    case "clay":
      return 1;
    case "iron":
      return 2;
    case "crop":
      return 3;
    case "upkeep":
      return 4;
    case "clock":
      return 6;
  }
}

type ResourceSpriteProps = {
  kind: ResourceSpriteKind;
  size?: number;
  label?: string;
  className?: string;
};

export function ResourceSprite({
  kind,
  size = RES_ICON_SIZE,
  label,
  className,
}: ResourceSpriteProps) {
  const index = indexForKind(kind);
  const scale = size / RES_ICON_SIZE;

  return (
    <span
      title={label ?? kind}
      class={className ?? "inline-block align-middle"}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        backgroundImage: "url(/static/misc/res.png)",
        backgroundRepeat: "no-repeat",
        backgroundPosition: `0px -${index * RES_STRIDE_Y * scale}px`,
        backgroundSize: `${RES_SPRITE_WIDTH * scale}px ${RES_SPRITE_HEIGHT * scale}px`,
        imageRendering: "pixelated",
      }}
    />
  );
}
