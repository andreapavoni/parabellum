import type { ReportDetailResponse, ReportListItem, ReportsResponse } from "@/types/api";
import { Link } from "@/components/Link";
import { ResourceSprite } from "@/components/ResourceSprite";
import { UnitSprite } from "@/components/UnitSprite";
import { Button, Panel } from "@/components/ui";
import { useGameContextQuery } from "@/query/hooks";
import { useMemo, useState } from "preact/hooks";
import { buildingLabel, tribeLabel } from "@/lib/labels";
import type { ComponentChildren } from "preact";

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function formatReportDate(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
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

function playerLink(playerName: string, playerId?: string) {
  if (!playerId) return <span class="font-semibold text-stone-900">{playerName}</span>;
  return (
    <Link to={`/players/${playerId}`} class="font-semibold text-green-800 hover:underline">
      {playerName}
    </Link>
  );
}

type VillageNamesById = Map<number, string>;

function resolvedVillageName(villageId: number | undefined, fallbackName: string, villageNamesById: VillageNamesById) {
  if (!villageId) return fallbackName;
  return villageNamesById.get(villageId) ?? fallbackName;
}

function villageLinkById(
  villageName: string,
  villageId: number | undefined,
  position: unknown,
  worldSize: number,
  villageNamesById: VillageNamesById,
) {
  const resolvedName = resolvedVillageName(villageId, villageName, villageNamesById);
  if (villageId) {
    return (
      <Link to={`/map/field/${villageId}`} class="font-semibold text-green-800 hover:underline">
        {resolvedName}
      </Link>
    );
  }
  return villageFieldLink(resolvedName, position, worldSize);
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

type ReportSubjectInput = Pick<ReportListItem, "reportType" | "payload" | "actorVillageId" | "targetVillageId">
  | Pick<ReportDetailResponse, "reportType" | "payload" | "actorVillageId" | "targetVillageId">;

function reportSubject(report: ReportSubjectInput, villageNamesById: VillageNamesById) {
  const variant = reportPayloadVariant(report.payload);
  if (!variant) {
    return report.reportType;
  }

  if (variant.kind === "Battle") {
    const payload = variant.value;
    const attackType = String(payload.attack_type ?? "");
    const verb = attackType === "Raid" ? "raided" : "attacked";
    const attackerVillage = resolvedVillageName(
      report.actorVillageId,
      String(payload.attacker_village ?? "Unknown"),
      villageNamesById,
    );
    const defenderVillage = resolvedVillageName(
      report.targetVillageId,
      String(payload.defender_village ?? "Unknown"),
      villageNamesById,
    );
    return `${attackerVillage} ${verb} ${defenderVillage}`;
  }

  if (variant.kind === "Reinforcement") {
    const payload = variant.value;
    const senderVillage = resolvedVillageName(
      report.actorVillageId,
      String(payload.sender_village ?? "Unknown"),
      villageNamesById,
    );
    const receiverVillage = resolvedVillageName(
      report.targetVillageId,
      String(payload.receiver_village ?? "Unknown"),
      villageNamesById,
    );
    return `${senderVillage} reinforced ${receiverVillage}`;
  }

  if (variant.kind === "MarketplaceDelivery") {
    const payload = variant.value;
    const senderVillage = resolvedVillageName(
      report.actorVillageId,
      String(payload.sender_village ?? "Unknown"),
      villageNamesById,
    );
    const receiverVillage = resolvedVillageName(
      report.targetVillageId,
      String(payload.receiver_village ?? "Unknown"),
      villageNamesById,
    );
    return `${senderVillage} sent resources to ${receiverVillage}`;
  }

  return report.reportType;
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
    <div class="rounded-md border border-stone-200 bg-stone-50 p-2">
      <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-stone-500">{title}</p>
      <div class="overflow-x-auto">
        <table class="w-full table-fixed border-collapse overflow-hidden rounded-md border border-stone-200 bg-white text-xs">
          <thead>
            <tr>
              {Array.from({ length }, (_, idx) => (
                <th key={`u-${idx}`} class="border-b border-stone-200 p-0.5 text-center text-stone-500">
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
                    class={`border-r border-stone-200 p-1 text-center last:border-r-0 ${value === 0 ? "bg-stone-50 opacity-50" : "bg-white"}`}
                  >
                    <div class={value === 0 ? "text-stone-400" : "font-semibold text-stone-900"}>
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
                    <td key={`loss-${idx}`} class="border-r border-stone-200 bg-stone-100 p-1 text-center last:border-r-0">
                      <div class={loss > 0 ? "font-semibold text-stone-700" : "text-stone-300"}>
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

function sumArrays(left: number[], right: number[]) {
  const length = Math.max(left.length, right.length);
  return Array.from({ length }, (_, idx) => (left[idx] ?? 0) + (right[idx] ?? 0));
}

function TrapBattleSection({
  trapped,
  freed,
  tribe,
}: {
  trapped: Record<string, unknown> | null;
  freed: Record<string, unknown> | null;
  tribe?: string;
}) {
  if (!trapped && !freed) return null;

  return (
    <SectionCard title="Traps">
      {trapped ? (
        <div class="rounded-md border border-stone-200 bg-white p-3">
          <p class="mb-2 text-xs font-semibold uppercase text-stone-500">
            Captured by traps
          </p>
          <ArmyTable
            title={`${readNumber(trapped, "traps_used")} traps used`}
            before={troopArray(trapped.trapped_units)}
            tribe={tribe}
          />
        </div>
      ) : null}
      {freed ? (
        <div class="rounded-md border border-stone-200 bg-white p-3">
          <p class="mb-2 text-xs font-semibold uppercase text-stone-500">
            Freed from traps
          </p>
          <ArmyTable
            title={`${readNumber(freed, "traps_destroyed")} traps destroyed`}
            before={troopArray(freed.units_before)}
            losses={troopArray(freed.deaths)}
            tribe={tribe}
          />
          {sumTroops(freed.survivors) > 0 ? (
            <p class="mt-2 text-sm text-stone-700">
              {sumTroops(freed.survivors)} troops survived captivity and returned home.
            </p>
          ) : null}
        </div>
      ) : null}
    </SectionCard>
  );
}

function groupedReinforcementsByTribe(entries: unknown[]) {
  const groups = new Map<string, { before: number[]; losses: number[] }>();
  for (const entry of entries) {
    const record = asRecord(entry) ?? {};
    const tribe = readString(record, "tribe", "Unknown");
    const current = groups.get(tribe) ?? { before: [], losses: [] };
    current.before = sumArrays(current.before, troopArray(record.army_before));
    current.losses = sumArrays(current.losses, troopArray(record.losses));
    groups.set(tribe, current);
  }
  return Array.from(groups.entries()).map(([tribe, data]) => ({ tribe, ...data }));
}

function totalResources(resources: unknown) {
  const parsed = parseResourceGroup(resources);
  return parsed.lumber + parsed.clay + parsed.iron + parsed.crop;
}

function DamageLine({ damage, fallbackName }: { damage: Record<string, unknown> | null; fallbackName: string }) {
  if (!damage) {
    return <p class="text-sm text-stone-700">{fallbackName} hasn't been damaged.</p>;
  }
  const name = buildingLabel(readString(damage, "name", fallbackName));
  const before = readNumber(damage, "level_before");
  const after = readNumber(damage, "level_after");
  if (before === after) {
    return <p class="text-sm text-stone-700">{name} hasn't been damaged.</p>;
  }
  return (
    <p class="text-sm text-stone-700">
      {name} has been damaged from level {before} to level {after}.
    </p>
  );
}

function SectionCard({
  title,
  children,
}: {
  title: string;
  children: ComponentChildren;
}) {
  return (
    <section class="rounded-md border border-stone-200 bg-white p-4 shadow-sm">
      <h2 class="mb-3 text-sm font-semibold uppercase tracking-wide text-stone-600">{title}</h2>
      <div class="space-y-3">{children}</div>
    </section>
  );
}

function BattleReportDetail({
  data,
  payload,
  worldSize,
  villageNamesById,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
  villageNamesById: VillageNamesById;
}) {
  const subject = reportSubject(data, villageNamesById);
  const bounty = parseResourceGroup(payload.bounty);
  const attacker = asRecord(payload.attacker);
  const defender = asRecord(payload.defender);
  const reinforcements = Array.isArray(payload.reinforcements) ? payload.reinforcements : [];
  const scouting = asRecord(payload.scouting);
  const wallDamage = asRecord(payload.wall_damage);
  const catapultDamage = Array.isArray(payload.catapult_damage) ? payload.catapult_damage : [];
  const trapped = asRecord(payload.trapped);
  const freed = asRecord(payload.freed);
  const attackerBefore = troopArray(attacker?.army_before);
  const attackerLosses = troopArray(attacker?.losses);
  const defenderBefore = troopArray(defender?.army_before);
  const defenderLosses = troopArray(defender?.losses);
  const attackerTribe = readString(attacker ?? {}, "tribe");
  const defenderTribe = readString(defender ?? {}, "tribe");
  const groupedReinforcements = groupedReinforcementsByTribe(reinforcements);
  const scoutingTargetReport = scouting?.target_report;
  const scoutingTarget = normalizeScoutingTarget(scouting?.target);
  const ramsPresent = (attackerBefore[6] ?? 0) > 0;
  const catapultsPresent = (attackerBefore[7] ?? 0) > 0;
  const chiefsPresent = (attackerBefore[8] ?? 0) > 0;
  const loyaltyBefore = payload.loyalty_before;
  const loyaltyAfter = payload.loyalty_after;

  return (
    <div class="space-y-4">
      <Panel class="space-y-1">
        <h1 class="text-xl font-semibold text-stone-900">{subject}</h1>
        <div class="text-sm text-stone-500">{formatTimestamp(data.createdAt)}</div>
      </Panel>

      <div class="space-y-4">
        <SectionCard title="Attacker">
          <p class="text-sm text-stone-700">
            {playerLink(readString(payload, "attacker_player", "Unknown"), data.actorPlayerId)} from{" "}
            {villageLinkById(
              readString(payload, "attacker_village", "Unknown"),
              data.actorVillageId,
              payload.attacker_position,
              worldSize,
              villageNamesById,
            )}
          </p>
          <ArmyTable title="Sent troops" before={attackerBefore} losses={attackerLosses} tribe={attackerTribe} />
          {totalResources(bounty) > 0 ? (
            <div class="rounded-md border border-stone-200 bg-white p-3">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Bounty</p>
              <p class="font-mono text-stone-800"><ResourceSummaryInline resources={bounty} /></p>
            </div>
          ) : null}
          {ramsPresent ? (
            <div class="rounded-md border border-stone-200 bg-white p-3">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Wall damage</p>
              <DamageLine damage={wallDamage} fallbackName="Wall" />
            </div>
          ) : null}
          {catapultsPresent ? (
            <div class="rounded-md border border-stone-200 bg-white p-3">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Building damage</p>
              {catapultDamage.length > 0 ? (
                <div class="space-y-1">
                  {catapultDamage.map((entry, idx) => (
                    <DamageLine key={`catapult-${idx}`} damage={asRecord(entry)} fallbackName="Building" />
                  ))}
                </div>
              ) : (
                <p class="text-sm text-stone-700">Buildings haven't been damaged.</p>
              )}
            </div>
          ) : null}
          {chiefsPresent && (loyaltyBefore != null || loyaltyAfter != null || payload.conquered != null) ? (
            <div class="rounded-md border border-stone-200 bg-white p-3">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Loyalty</p>
              {payload.conquered === true ? (
                <p class="text-sm font-semibold text-green-800">The village has been conquered.</p>
              ) : Number(loyaltyBefore ?? 0) !== Number(loyaltyAfter ?? 0) ? (
                <p class="text-sm text-stone-700">
                  Loyalty lowered from {Number(loyaltyBefore ?? 0)} to {Number(loyaltyAfter ?? 0)}.
                </p>
              ) : (
                <p class="text-sm text-stone-700">Loyalty hasn't been changed.</p>
              )}
            </div>
          ) : null}
        </SectionCard>

        <TrapBattleSection trapped={trapped} freed={freed} tribe={attackerTribe} />

        <SectionCard title="Defender">
          <p class="text-sm text-stone-700">
            {playerLink(readString(payload, "defender_player", "Unknown"), data.targetPlayerId)} from{" "}
            {villageLinkById(
              readString(payload, "defender_village", "Unknown"),
              data.targetVillageId,
              payload.defender_position,
              worldSize,
              villageNamesById,
            )}
          </p>
          {defender ? (
            <ArmyTable title="Village troops" before={defenderBefore} losses={defenderLosses} tribe={defenderTribe} />
          ) : (
            <p class="rounded-md border border-stone-200 bg-white p-3 text-sm text-stone-500">No village troops were present.</p>
          )}
          {groupedReinforcements.length > 0 ? (
            <div class="space-y-3">
              <p class="text-xs font-semibold uppercase text-stone-500">Reinforcements</p>
              {groupedReinforcements.map((group) => (
                <ArmyTable
                  key={group.tribe}
                  title={`${tribeLabel(group.tribe)} reinforcements`}
                  before={group.before}
                  losses={group.losses}
                  tribe={group.tribe}

                />
              ))}
            </div>
          ) : null}
        </SectionCard>
      </div>

      {scouting ? (
        <SectionCard title="Scouting">
          <p class="text-sm text-stone-700">
            {Boolean(scouting.was_detected) ? "Scouts were detected." : "Scouts were not detected."}
          </p>
          {scoutingTarget === "resources" ? (
            <div class="rounded-md border border-stone-200 bg-white p-3 text-sm text-stone-700">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Revealed resources</p>
              <p class="font-mono"><ResourceSummaryInline resources={parseScoutingResources(scoutingTargetReport)} /></p>
            </div>
          ) : null}
          {scoutingTarget === "defenses" ? (
            <div class="space-y-1 rounded-md border border-stone-200 bg-white p-3 text-sm text-stone-700">
              <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Revealed defenses</p>
              <p>Wall: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "wall", 0)}</p>
              <p>Palace: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "palace", 0)}</p>
              <p>Residence: Level {readNumber(parseScoutingDefenses(scoutingTargetReport), "residence", 0)}</p>
            </div>
          ) : null}
          {scoutingTarget === "unknown" ? (
            <pre class="overflow-auto rounded bg-stone-950/95 p-3 text-xs text-stone-100">
              {JSON.stringify(scouting.target_report, null, 2)}
            </pre>
          ) : null}
        </SectionCard>
      ) : null}

    </div>
  );
}
function ReinforcementReportDetail({
  data,
  payload,
  worldSize,
  villageNamesById,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
  villageNamesById: VillageNamesById;
}) {
  const subject = reportSubject(data, villageNamesById);
  return (
    <div class="space-y-4">
      <Panel class="space-y-1">
        <h1 class="text-xl font-semibold text-stone-900">{subject}</h1>
        <div class="text-sm text-stone-500">{formatTimestamp(data.createdAt)}</div>
      </Panel>
      <div class="space-y-4">
        <SectionCard title="Sender">
          <p class="text-sm text-stone-700">
            {playerLink(readString(payload, "sender_player", "Unknown"), data.actorPlayerId)} from{" "}
            {villageLinkById(
              readString(payload, "sender_village", "Unknown"),
              data.actorVillageId,
              payload.sender_position,
              worldSize,
              villageNamesById,
            )}
          </p>
          <ArmyTable title="Sent troops" before={troopArray(payload.units)} tribe={readString(payload, "tribe")} />
        </SectionCard>
        <SectionCard title="Receiver">
          <p class="text-sm text-stone-700">
            {playerLink(readString(payload, "receiver_player", "Unknown"), data.targetPlayerId)} from{" "}
            {villageLinkById(
              readString(payload, "receiver_village", "Unknown"),
              data.targetVillageId,
              payload.receiver_position,
              worldSize,
              villageNamesById,
            )}
          </p>
        </SectionCard>
      </div>
    </div>
  );
}

function MarketplaceDeliveryReportDetail({
  data,
  payload,
  worldSize,
  villageNamesById,
}: {
  data: ReportDetailResponse;
  payload: Record<string, unknown>;
  worldSize: number;
  villageNamesById: VillageNamesById;
}) {
  const resources = asRecord(payload.resources) ?? {};
  const subject = reportSubject(data, villageNamesById);
  return (
    <div class="space-y-4">
      <Panel class="space-y-1">
        <h1 class="text-xl font-semibold text-stone-900">{subject}</h1>
        <div class="text-sm text-stone-500">{formatTimestamp(data.createdAt)}</div>
      </Panel>
      <SectionCard title="Delivery">
        <p class="text-sm text-stone-700">
          {playerLink(readString(payload, "sender_player", "Unknown"), data.actorPlayerId)} from{" "}
          {villageLinkById(
            readString(payload, "sender_village", "Unknown"),
            data.actorVillageId,
            payload.sender_position,
            worldSize,
            villageNamesById,
          )}{" "}
          sent resources to {playerLink(readString(payload, "receiver_player", "Unknown"), data.targetPlayerId)} from{" "}
          {villageLinkById(
            readString(payload, "receiver_village", "Unknown"),
            data.targetVillageId,
            payload.receiver_position,
            worldSize,
            villageNamesById,
          )}.
        </p>
        <div class="rounded-md border border-stone-200 bg-white p-3">
          <p class="mb-1 text-xs font-semibold uppercase text-stone-500">Resources</p>
          <p class="font-mono text-stone-800"><ResourceSummaryInline resources={resources} /></p>
        </div>
      </SectionCard>
    </div>
  );
}

export function ReportsPage({ data }: { data: ReportsResponse }) {
  const gameContext = useGameContextQuery();
  const villageNamesById = useMemo(
    () => new Map((gameContext.data?.villages ?? []).map((village) => [village.id, village.name] as const)),
    [gameContext.data?.villages],
  );
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
          <Button
            key={key}
            type="button"
            variant={filter === key ? "primary" : "secondary"}
            size="sm"
            onClick={() => setFilter(key)}
          >
            {key === "all" ? "All" : key === "attacks" ? "Attacks" : key === "reinforcements" ? "Reinforcements" : "Merchants"}
          </Button>
        ))}
      </div>
      {filtered.length === 0 ? (
        <Panel class="text-center text-sm text-gray-500">No reports available.</Panel>
      ) : null}
      {filtered.map((report) => {
        const subject = reportSubject(report, villageNamesById);
        return (
          <Link
            key={report.id}
            to={`/reports/${report.id}`}
            class={`block rounded-md border px-4 py-3 shadow-sm ${report.isRead ? "border-stone-200 bg-white" : "border-amber-200 bg-amber-50"}`}
          >
            <div class="flex min-w-0 items-center justify-between gap-4">
              <div class="min-w-0 truncate font-semibold text-stone-900">{subject}</div>
              <div class="shrink-0 text-xs text-stone-500">{formatReportDate(report.createdAt)}</div>
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
  const gameContext = useGameContextQuery();
  const worldSize = gameContext.data?.worldSize ?? 100;
  const villageNamesById = useMemo(
    () => new Map((gameContext.data?.villages ?? []).map((village) => [village.id, village.name] as const)),
    [gameContext.data?.villages],
  );
  const variant = reportPayloadVariant(data.payload);

  return (
    <div class="mx-auto max-w-4xl px-4 py-6 space-y-4">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-semibold text-gray-800">Report</h1>
        <Link to="/reports" class="text-sm text-green-700 hover:underline">
          Back to reports
        </Link>
      </div>

      {variant?.kind === "Battle" ? (
        <BattleReportDetail data={data} payload={variant.value} worldSize={worldSize} villageNamesById={villageNamesById} />
      ) : null}
      {variant?.kind === "Reinforcement" ? (
        <ReinforcementReportDetail
          data={data}
          payload={variant.value}
          worldSize={worldSize}
          villageNamesById={villageNamesById}
        />
      ) : null}
      {variant?.kind === "MarketplaceDelivery" ? (
        <MarketplaceDeliveryReportDetail
          data={data}
          payload={variant.value}
          worldSize={worldSize}
          villageNamesById={villageNamesById}
        />
      ) : null}
      {!variant || !["Battle", "Reinforcement", "MarketplaceDelivery"].includes(variant.kind) ? (
        <Panel>
          <div class="text-sm text-gray-500">{formatTimestamp(data.createdAt)}</div>
          <div class="mt-2 text-sm font-semibold text-gray-700">Type: {data.reportType}</div>
          <pre class="mt-4 overflow-auto rounded bg-stone-950/95 p-4 text-xs text-stone-100">
            {JSON.stringify(data.payload, null, 2)}
          </pre>
        </Panel>
      ) : null}
    </div>
  );
}
