import type { ReportDetailResponse, ReportsResponse } from "@/types/api";
import { Link } from "@/components/Link";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSprite } from "@/components/UnitSprite";
import { useAppStore } from "@/state/appStore";
import { useMemo, useState } from "preact/hooks";

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  return value as Record<string, unknown>;
}

function readString(record: Record<string, unknown>, key: string, fallback = "") {
  const value = record[key];
  return typeof value === "string" ? value : fallback;
}

function readNumber(record: Record<string, unknown>, key: string, fallback = 0) {
  const value = Number(record[key]);
  return Number.isFinite(value) ? value : fallback;
}

function parseResourceGroup(resources: unknown) {
  if (Array.isArray(resources)) {
    return {
      lumber: Number(resources[0] ?? 0) || 0,
      clay: Number(resources[1] ?? 0) || 0,
      iron: Number(resources[2] ?? 0) || 0,
      crop: Number(resources[3] ?? 0) || 0,
    };
  }

  const record = asRecord(resources) ?? {};
  return {
    lumber: readNumber(record, "lumber"),
    clay: readNumber(record, "clay"),
    iron: readNumber(record, "iron"),
    crop: readNumber(record, "crop"),
  };
}

function formatResourceSummary(resources: unknown) {
  const { lumber, clay, iron, crop } = parseResourceGroup(resources);
  return `lumber ${lumber}, clay ${clay}, iron ${iron}, crop ${crop}`;
}

function ResourceSummaryInline({ resources }: { resources: unknown }) {
  const { lumber, clay, iron, crop } = parseResourceGroup(resources);
  return (
    <span class="inline-flex items-center gap-2">
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="lumber" size={12} label="Lumber" />{lumber}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="clay" size={12} label="Clay" />{clay}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="iron" size={12} label="Iron" />{iron}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="crop" size={12} label="Crop" />{crop}</span>
    </span>
  );
}

function normalizeScoutingTarget(target: unknown): "resources" | "defenses" | "unknown" {
  const value = String(target ?? "").toLowerCase();
  if (value === "resources") return "resources";
  if (value === "defenses") return "defenses";
  return "unknown";
}

function parseScoutingResources(targetReport: unknown): unknown {
  const record = asRecord(targetReport);
  if (record && Array.isArray(record.Resources)) {
    return record.Resources;
  }
  return targetReport;
}

function parseScoutingDefenses(targetReport: unknown): Record<string, unknown> {
  const record = asRecord(targetReport);
  if (record) {
    const nested = asRecord(record.Defenses);
    if (nested) return nested;
    return record;
  }
  return {};
}

function sumTroops(units: unknown): number {
  if (Array.isArray(units)) {
    return units.reduce<number>((acc, value) => acc + Number(value || 0), 0);
  }
  if (units && typeof units === "object") {
    return Object.values(units as Record<string, unknown>).reduce<number>(
      (acc, value) => acc + Number(value || 0),
      0,
    );
  }
  return 0;
}

function troopArray(units: unknown): number[] {
  if (Array.isArray(units)) {
    return units.map((value) => Number(value || 0));
  }
  if (units && typeof units === "object") {
    return Object.values(units as Record<string, unknown>).map((value) => Number(value || 0));
  }
  return [];
}

function positionLabel(position: unknown) {
  const pos = asRecord(position);
  if (!pos) return "(?|?)";
  const x = Number(pos.x);
  const y = Number(pos.y);
  const safeX = Number.isFinite(x) ? String(x) : "?";
  const safeY = Number.isFinite(y) ? String(y) : "?";
  return `(${safeX}|${safeY})`;
}

function mapFieldIdFromPosition(x: number, y: number, worldSize: number) {
  return (worldSize - y) * (worldSize * 2 + 1) + (worldSize + x + 1);
}

function villageFieldLink(villageName: string, position: unknown, worldSize: number) {
  const pos = asRecord(position);
  if (!pos) {
    return <span>{villageName} (?|?)</span>;
  }
  const x = Number(pos.x);
  const y = Number(pos.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    return <span>{villageName} (?|?)</span>;
  }
  const fieldId = mapFieldIdFromPosition(x, y, worldSize);
  return (
    <span>
      <Link to={`/map/field/${fieldId}`} class="text-green-700 hover:underline">
        {villageName}
      </Link>{" "}
      <Link to={`/map/field/${fieldId}`} class="text-green-700 hover:underline">
        ({x}|{y})
      </Link>
    </span>
  );
}

function reportPayloadVariant(payload: unknown): { kind: string; value: Record<string, unknown> } | null {
  if (!payload || typeof payload !== "object") return null;
  const entries = Object.entries(payload as Record<string, unknown>);
  if (entries.length !== 1) return null;
  const entry = entries[0];
  if (!entry) return null;
  const [kind, value] = entry;
  if (!value || typeof value !== "object") return null;
  return { kind, value: value as Record<string, unknown> };
}

function reportText(report: ReportsResponse["reports"][number]) {
  const variant = reportPayloadVariant(report.payload);
  if (!variant) {
    return {
      title: report.reportType,
      summary: "Report payload unavailable",
    };
  }

  if (variant.kind === "Battle") {
    const payload = variant.value;
    const scout = payload.scouting != null;
    const attackType = String(payload.attack_type ?? "");
    const verb = scout ? "scouted" : attackType === "Raid" ? "raided" : "attacked";
    const attackerVillage = String(payload.attacker_village ?? "Unknown");
    const defenderVillage = String(payload.defender_village ?? "Unknown");
    const attackerPos = payload.attacker_position as Record<string, unknown> | undefined;
    const defenderPos = payload.defender_position as Record<string, unknown> | undefined;
    const success = Boolean(payload.success);
    const bounty = parseResourceGroup(payload.bounty);
    const bountyTotal = bounty.lumber + bounty.clay + bounty.iron + bounty.crop;
    const attacker = payload.attacker as Record<string, unknown> | undefined;
    const losses = attacker?.losses;
    const totalLosses = sumTroops(losses);
    const outcome = bountyTotal > 0
      ? `Bounty: ${formatResourceSummary(bounty)}`
      : totalLosses > 0
        ? `Lost ${totalLosses} units`
        : "No losses";

    return {
      title: `${attackerVillage} ${verb} ${defenderVillage}`,
      summary: `${attackerVillage} (${attackerPos?.x ?? "?"}|${attackerPos?.y ?? "?"}) ${verb} ${defenderVillage} (${defenderPos?.x ?? "?"}|${defenderPos?.y ?? "?"}) - ${success ? "Victory" : "Defeat"} - ${outcome}`,
    };
  }

  if (variant.kind === "Reinforcement") {
    const payload = variant.value;
    const senderVillage = String(payload.sender_village ?? "Unknown");
    const receiverVillage = String(payload.receiver_village ?? "Unknown");
    const senderPos = payload.sender_position as Record<string, unknown> | undefined;
    const receiverPos = payload.receiver_position as Record<string, unknown> | undefined;
    const units = sumTroops(payload.units);

    return {
      title: `${senderVillage} reinforced ${receiverVillage}`,
      summary: `${senderVillage} (${senderPos?.x ?? "?"}|${senderPos?.y ?? "?"}) reinforced ${receiverVillage} (${receiverPos?.x ?? "?"}|${receiverPos?.y ?? "?"}) - ${units} troops sent`,
    };
  }

  if (variant.kind === "MarketplaceDelivery") {
    const payload = variant.value;
    const senderVillage = String(payload.sender_village ?? "Unknown");
    const receiverVillage = String(payload.receiver_village ?? "Unknown");
    const senderPos = payload.sender_position as Record<string, unknown> | undefined;
    const receiverPos = payload.receiver_position as Record<string, unknown> | undefined;
    const resources = (payload.resources as Record<string, unknown> | undefined) ?? {};

    return {
      title: `${senderVillage} delivered resources to ${receiverVillage}`,
      summary: `${senderVillage} (${senderPos?.x ?? "?"}|${senderPos?.y ?? "?"}) delivered ${formatResourceSummary(resources)} to ${receiverVillage} (${receiverPos?.x ?? "?"}|${receiverPos?.y ?? "?"})`,
    };
  }

  return {
    title: report.reportType,
    summary: "Unknown report payload",
  };
}

function ArmyTable({
  title,
  before,
  losses,
  tribe,
}: {
  title: string;
  before: number[];
  losses?: number[];
  tribe?: string;
}) {
  const length = Math.max(before.length, losses?.length ?? 0);
  if (length === 0) {
    return null;
  }

  return (
    <div class="border rounded-md p-4">
      <p class="text-xs uppercase text-gray-500 font-semibold mb-3">{title}</p>
      <div class="overflow-x-auto">
        <table class="w-full border-collapse">
          <thead>
            <tr>
              {Array.from({ length }, (_, idx) => (
                <th key={`u-${idx}`} class="text-center p-1 text-xs text-gray-500 border-b">
                  <UnitSprite tribe={tribe} unitIndex={idx} label={`U${idx + 1}`} />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              {Array.from({ length }, (_, idx) => {
                const value = before[idx] ?? 0;
                return (
                  <td
                    key={`before-${idx}`}
                    class={`text-center p-2 border-r last:border-r-0 ${value === 0 ? "bg-gray-50 opacity-40" : "bg-gray-100"}`}
                  >
                    <div class={value === 0 ? "text-gray-400 text-sm" : "text-gray-900 font-semibold"}>
                      {value}
                    </div>
                  </td>
                );
              })}
            </tr>
            {losses ? (
              <tr>
                {Array.from({ length }, (_, idx) => {
                  const loss = losses[idx] ?? 0;
                  return (
                    <td key={`loss-${idx}`} class="text-center p-2 border-r last:border-r-0 bg-red-50">
                      <div class={loss > 0 ? "text-red-600 font-semibold text-sm" : "text-gray-300 text-xs"}>
                        {loss > 0 ? `↓${loss}` : "-"}
                      </div>
                    </td>
                  );
                })}
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function BattleReportDetail({
  data,
  payload,
  worldSize,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
}) {
  const success = Boolean(payload.success);
  const resultClass = success ? "border-green-200 bg-green-50 text-green-700" : "border-red-200 bg-red-50 text-red-700";

  const bounty = parseResourceGroup(payload.bounty);
  const attacker = asRecord(payload.attacker);
  const defender = asRecord(payload.defender);
  const reinforcements = Array.isArray(payload.reinforcements) ? payload.reinforcements : [];
  const scouting = asRecord(payload.scouting);
  const wallDamage = asRecord(payload.wall_damage);
  const catapultDamage = Array.isArray(payload.catapult_damage) ? payload.catapult_damage : [];

  const attackerBefore = troopArray(attacker?.army_before);
  const attackerLosses = troopArray(attacker?.losses);
  const defenderBefore = troopArray(defender?.army_before);
  const defenderLosses = troopArray(defender?.losses);
  const attackerTribe = readString(attacker ?? {}, "tribe");
  const defenderTribe = readString(defender ?? {}, "tribe");
  const scoutingTargetReport = scouting?.target_report;
  const scoutingTarget = normalizeScoutingTarget(scouting?.target);

  return (
    <div class="space-y-4">
      <div class={`p-4 rounded-md border ${resultClass}`}>
        <div class="text-xl font-bold">{success ? "Victory" : "Defeat"}</div>
        <div class="text-sm mt-1">
          {readString(payload, "attacker_village", "Unknown")} {readString(payload, "attack_type", "").toLowerCase()}{" "}
          {readString(payload, "defender_village", "Unknown")}
        </div>
      </div>

      <div class="grid gap-4 md:grid-cols-2">
        <div class="border rounded-md p-4 bg-red-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">
            Attacker - {readString(payload, "attacker_player", "Unknown")}
          </p>
          <p class="text-sm text-gray-600 mb-3">
            {villageFieldLink(
              readString(payload, "attacker_village", "Unknown"),
              payload.attacker_position,
              worldSize,
            )}
          </p>
        </div>
        <div class="border rounded-md p-4 bg-blue-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">
            Defender - {readString(payload, "defender_player", "Unknown")}
          </p>
          <p class="text-sm text-gray-600 mb-3">
            {villageFieldLink(
              readString(payload, "defender_village", "Unknown"),
              payload.defender_position,
              worldSize,
            )}
          </p>
        </div>
      </div>

      <ArmyTable title="Attacker Army" before={attackerBefore} losses={attackerLosses} tribe={attackerTribe} />
      {defender ? <ArmyTable title="Defender Army" before={defenderBefore} losses={defenderLosses} tribe={defenderTribe} /> : null}

      {reinforcements.length > 0 ? (
        <div class="border rounded-md p-4">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-3">Reinforcements</p>
          <div class="space-y-3">
            {reinforcements.map((entry, idx) => {
              const reinf = asRecord(entry) ?? {};
              return (
                <ArmyTable
                  key={`reinf-${idx}`}
                  title={`Reinforcement #${idx + 1}`}
                  before={troopArray(reinf.army_before)}
                  losses={troopArray(reinf.losses)}
                  tribe={readString(reinf, "tribe")}
                />
              );
            })}
          </div>
        </div>
      ) : null}

      {scouting ? (
        <div class="border rounded-md p-4 bg-blue-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">Scouting</p>
          <p class="text-sm text-gray-700 mb-2">
            {Boolean(scouting.was_detected) ? "Scouts were detected" : "Scouts were not detected"}
          </p>
          {scoutingTarget === "resources" ? (
            <div class="rounded bg-white p-3 text-sm text-gray-700">
              <p class="text-xs uppercase text-gray-500 font-semibold mb-1">Revealed resources</p>
              <p class="font-mono"><ResourceSummaryInline resources={parseScoutingResources(scoutingTargetReport)} /></p>
            </div>
          ) : null}
          {scoutingTarget === "defenses" ? (
            <div class="rounded bg-white p-3 text-sm text-gray-700 space-y-1">
              <p class="text-xs uppercase text-gray-500 font-semibold mb-1">Revealed defenses</p>
              <p>Wall: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "wall", 0)}</p>
              <p>Palace: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "palace", 0)}</p>
              <p>Residence: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "residence", 0)}</p>
            </div>
          ) : null}
          {scoutingTarget === "unknown" ? (
            <pre class="overflow-auto rounded bg-white p-3 text-xs text-gray-700">
              {JSON.stringify(scouting.target_report, null, 2)}
            </pre>
          ) : null}
        </div>
      ) : null}

      {wallDamage ? (
        <div class="border rounded-md p-4 bg-orange-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">Ram Damage</p>
          <p class="text-sm text-gray-700">
            {readString(wallDamage, "name", "Wall")}: Level {readNumber(wallDamage, "level_before")} → Level{" "}
            {readNumber(wallDamage, "level_after")}
          </p>
        </div>
      ) : null}

      {catapultDamage.length > 0 ? (
        <div class="border rounded-md p-4 bg-red-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">Catapult Damage</p>
          <div class="space-y-1">
            {catapultDamage.map((entry, idx) => {
              const damage = asRecord(entry) ?? {};
              return (
                <p key={`catapult-${idx}`} class="text-sm text-gray-700">
                  {readString(damage, "name", "Building")}: Level {readNumber(damage, "level_before")} → Level{" "}
                  {readNumber(damage, "level_after")}
                </p>
              );
            })}
          </div>
        </div>
      ) : null}

      {(payload.loyalty_before != null || payload.loyalty_after != null) ? (
        <div class="border rounded-md p-4 bg-purple-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">Loyalty</p>
          <p class="text-sm text-gray-700">
            {Number(payload.loyalty_before ?? 0)} → {Number(payload.loyalty_after ?? 0)}
            {payload.conquered === true ? " (Conquered)" : ""}
          </p>
        </div>
      ) : null}

      <div class="border rounded-md p-4">
        <p class="text-xs uppercase text-gray-500 font-semibold mb-1">Bounty</p>
        <p class="font-mono text-gray-800"><ResourceSummaryInline resources={bounty} /></p>
      </div>

      <div class="text-xs text-gray-500">Created at {formatTimestamp(data.createdAt)} • {data.id}</div>
    </div>
  );
}

function ReinforcementReportDetail({
  data,
  payload,
  worldSize,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
}) {
  return (
    <div class="space-y-4">
      <div class="grid gap-4 md:grid-cols-2">
        <div class="border rounded-md p-4 bg-blue-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">From</p>
          <p class="text-sm text-gray-700 font-semibold">{readString(payload, "sender_player", "Unknown")}</p>
          <p class="text-sm text-gray-600">
            {villageFieldLink(
              readString(payload, "sender_village", "Unknown"),
              payload.sender_position,
              worldSize,
            )}
          </p>
        </div>
        <div class="border rounded-md p-4 bg-green-50">
          <p class="text-xs uppercase text-gray-500 font-semibold mb-2">To</p>
          <p class="text-sm text-gray-700 font-semibold">{readString(payload, "receiver_player", "Unknown")}</p>
          <p class="text-sm text-gray-600">
            {villageFieldLink(
              readString(payload, "receiver_village", "Unknown"),
              payload.receiver_position,
              worldSize,
            )}
          </p>
        </div>
      </div>
      <ArmyTable title="Troops Sent" before={troopArray(payload.units)} tribe={readString(payload, "tribe")} />
      <div class="text-xs text-gray-500">Created at {formatTimestamp(data.createdAt)} • {data.id}</div>
    </div>
  );
}

function MarketplaceDeliveryReportDetail({
  data,
  payload,
  worldSize,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
}) {
  const resources = asRecord(payload.resources) ?? {};
  return (
    <div class="space-y-4">
      <div class="border rounded-md p-4 bg-white">
        <p class="text-sm text-gray-700">
          {villageFieldLink(readString(payload, "sender_village", "Unknown"), payload.sender_position, worldSize)}{" "}
          delivered <ResourceSummaryInline resources={resources} /> to{" "}
          {villageFieldLink(readString(payload, "receiver_village", "Unknown"), payload.receiver_position, worldSize)}.
        </p>
      </div>
      <div class="text-xs text-gray-500">Created at {formatTimestamp(data.createdAt)} • {data.id}</div>
    </div>
  );
}

export function ReportsPage({ data }: { data: ReportsResponse }) {
  const [filter, setFilter] = useState<"all" | "attacks" | "reinforcements" | "merchants">("all");
  const currentPage = data.pagination?.page ?? 1;
  const perPage = data.pagination?.perPage ?? 25;
  const hasMore = !!data.pagination?.hasMore;
  const prevHref = `/reports?page=${Math.max(1, currentPage - 1)}&per_page=${perPage}`;
  const nextHref = `/reports?page=${currentPage + 1}&per_page=${perPage}`;
  const filtered = useMemo(() => {
    switch (filter) {
      case "attacks":
        return data.reports.filter((report) => report.reportType === "battle");
      case "reinforcements":
        return data.reports.filter((report) => report.reportType === "reinforcement");
      case "merchants":
        return data.reports.filter((report) => report.reportType === "marketplace_delivery");
      default:
        return data.reports;
    }
  }, [data.reports, filter]);

  return (
    <div class="mx-auto max-w-4xl px-4 py-6 space-y-3">
      <div class="flex items-center justify-between gap-3">
        <h1 class="text-2xl font-semibold text-gray-800">Reports</h1>
        <div class="text-xs text-gray-500">Page {currentPage}</div>
      </div>
      <div class="flex items-center gap-2 text-sm">
        {(["all", "attacks", "reinforcements", "merchants"] as const).map((key) => (
          <button
            key={key}
            type="button"
            onClick={() => setFilter(key)}
            class={`px-3 py-1 rounded border ${filter === key ? "bg-green-600 text-white border-green-600" : "bg-white text-gray-700 border-gray-300 hover:bg-gray-50"}`}
          >
            {key === "all" ? "All" : key === "attacks" ? "Attacks" : key === "reinforcements" ? "Reinforcements" : "Merchants"}
          </button>
        ))}
      </div>
      {filtered.length === 0 ? (
        <div class="rounded border bg-white p-6 text-center text-sm text-gray-500">No reports available.</div>
      ) : null}
      {filtered.map((report) => {
        const text = reportText(report);
        return (
          <Link
            key={report.id}
            to={`/reports/${report.id}`}
            class={`block rounded border px-4 py-3 shadow-sm ${report.isRead ? "bg-white" : "bg-amber-50 border-amber-200"}`}
          >
            <div class="flex items-center justify-between gap-4">
              <div>
                <div class="font-semibold text-gray-800">{text.title}</div>
                <div class="text-sm text-gray-600">{text.summary}</div>
              </div>
              <div class="text-xs text-gray-500">{formatTimestamp(report.createdAt)}</div>
            </div>
          </Link>
        );
      })}
      <div class="flex items-center justify-between pt-2">
        {currentPage > 1 ? (
          <Link to={prevHref} class="text-sm text-green-700 hover:underline">
            Previous
          </Link>
        ) : (
          <span class="text-sm text-gray-400">Previous</span>
        )}
        {hasMore ? (
          <Link to={nextHref} class="text-sm text-green-700 hover:underline">
            Next
          </Link>
        ) : (
          <span class="text-sm text-gray-400">Next</span>
        )}
      </div>
    </div>
  );
}

export function ReportDetailPage({ data }: { data: ReportDetailResponse }) {
  const { meContext } = useAppStore();
  const worldSize = meContext?.worldSize ?? 100;
  const variant = reportPayloadVariant(data.payload);

  return (
    <div class="mx-auto max-w-4xl px-4 py-6 space-y-4">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-semibold text-gray-800">Report</h1>
        <Link to="/reports" class="text-sm text-green-700 hover:underline">
          Back to reports
        </Link>
      </div>

      <div class="rounded border bg-white p-4 shadow-sm space-y-4">
        {variant?.kind === "Battle" ? (
          <BattleReportDetail data={data} payload={variant.value} worldSize={worldSize} />
        ) : null}
        {variant?.kind === "Reinforcement" ? (
          <ReinforcementReportDetail data={data} payload={variant.value} worldSize={worldSize} />
        ) : null}
        {variant?.kind === "MarketplaceDelivery" ? (
          <MarketplaceDeliveryReportDetail data={data} payload={variant.value} worldSize={worldSize} />
        ) : null}
        {!variant || !["Battle", "Reinforcement", "MarketplaceDelivery"].includes(variant.kind) ? (
          <>
            <div class="text-sm text-gray-500">{formatTimestamp(data.createdAt)}</div>
            <div class="mt-2 text-sm font-semibold text-gray-700">Type: {data.reportType}</div>
            <pre class="mt-4 overflow-auto rounded bg-stone-950/95 p-4 text-xs text-stone-100">
              {JSON.stringify(data.payload, null, 2)}
            </pre>
          </>
        ) : null}
      </div>
    </div>
  );
}
