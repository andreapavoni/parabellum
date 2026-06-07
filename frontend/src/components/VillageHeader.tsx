import { Crown, MapPin, Pencil, Shield } from "lucide-preact";
import type { VillageListItem, VillageSummary } from "@/types/api";
import { Link } from "@/components/Link";
import { VillageRenameInline } from "@/components/VillageRenameInline";
import { Badge, SectionHeader } from "@/components/ui";

export function VillageHeading({
  village,
  onVillageRenamed,
}: {
  village: VillageSummary;
  onVillageRenamed?: () => Promise<void> | void;
}) {
  return (
    <div class="flex w-full flex-wrap items-center justify-center gap-x-3 gap-y-2 text-center text-sm">
      <h1 class="text-xl font-bold text-stone-900">{village.name}</h1>
      <Link to={`/map?x=${village.x}&y=${village.y}`} class="inline-flex items-center gap-1 text-xs text-stone-500 hover:underline">
        <MapPin size={13} aria-hidden="true" />
        {village.x}|{village.y}
      </Link>
      <span class="inline-flex items-center gap-1 text-xs text-stone-600">
        <Shield size={13} aria-hidden="true" />
        Loyalty
        <Badge variant={village.loyalty < 100 ? "warning" : "neutral"}>{village.loyalty}%</Badge>
      </span>
      {village.isCapital ? (
        <Badge variant="success" class="gap-1">
          <Crown size={12} aria-hidden="true" />
          Capital
        </Badge>
      ) : null}
      <VillageRenameInline
        villageId={village.id}
        currentName={village.name}
        onRenamed={onVillageRenamed}
        label={
          <span class="inline-flex items-center gap-1">
            <Pencil size={12} aria-hidden="true" />
            Rename
          </span>
        }
        className="contents"
        linkClassName="inline-flex items-center rounded-md border border-stone-300 bg-white px-2.5 py-1 text-xs font-semibold text-stone-700 hover:bg-stone-50"
      />
    </div>
  );
}

export function VillageSelector({
  villages,
  onSwitchVillage,
}: {
  villages: VillageListItem[];
  onSwitchVillage: (villageId: number) => void;
}) {
  const villagesByName = [...villages].sort((a, b) => {
    const byName = a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    if (byName !== 0) return byName;
    return a.id - b.id;
  });

  return (
    <div>
      <SectionHeader title="Villages" class="mb-2" />
      <ul class="space-y-1 text-xs">
        {villagesByName.map((item) => (
          <li key={item.id}>
            {item.isCurrent ? (
              <div class="grid w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-2 rounded-md border border-stone-200 bg-stone-100 px-2 py-1.5 font-semibold text-stone-900">
                <span class="truncate">{item.name}</span>
                <span class="shrink-0 text-stone-500">{item.x}|{item.y}</span>
              </div>
            ) : (
              <button
                type="button"
                class="grid w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-2 rounded-md border border-transparent bg-transparent px-2 py-1.5 text-left text-stone-700 transition-colors hover:bg-green-50"
                onClick={() => onSwitchVillage(item.id)}
              >
                <span class="truncate">{item.name}</span>
                <span class="shrink-0 text-stone-500">{item.x}|{item.y}</span>
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
