import type { ReportDetailResponse, ReportsResponse } from "@/types/api";
import { Link } from "@/components/Link";

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function formatResourceSummary(resources: Record<string, unknown>) {
  const lumber = Number(resources.lumber ?? 0);
  const clay = Number(resources.clay ?? 0);
  const iron = Number(resources.iron ?? 0);
  const crop = Number(resources.crop ?? 0);
  return `🌲 ${lumber} 🧱 ${clay} ⛏️ ${iron} 🌾 ${crop}`;
}

function sumTroops(units: unknown): number {
  if (Array.isArray(units)) {
    return units.reduce((acc, value) => acc + Number(value || 0), 0);
  }
  if (units && typeof units === "object") {
    return Object.values(units as Record<string, unknown>).reduce((acc, value) => acc + Number(value || 0), 0);
  }
  return 0;
}

function reportPayloadVariant(payload: unknown): { kind: string; value: Record<string, unknown> } | null {
  if (!payload || typeof payload !== "object") return null;
  const entries = Object.entries(payload as Record<string, unknown>);
  if (entries.length !== 1) return null;
  const [kind, value] = entries[0];
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
    const bounty = (payload.bounty as Record<string, unknown> | undefined) ?? {};
    const bountyTotal =
      Number(bounty.lumber ?? 0) + Number(bounty.clay ?? 0) + Number(bounty.iron ?? 0) + Number(bounty.crop ?? 0);
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

export function ReportsPage({ data }: { data: ReportsResponse }) {
  return (
    <div class="mx-auto max-w-4xl px-4 py-6 space-y-3">
      <h1 class="text-2xl font-semibold text-gray-800">Reports</h1>
      {data.reports.map((report) => {
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
    </div>
  );
}

export function ReportDetailPage({ data }: { data: ReportDetailResponse }) {
  return (
    <div class="mx-auto max-w-4xl px-4 py-6 space-y-4">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-semibold text-gray-800">Report</h1>
        <Link to="/reports" class="text-sm text-green-700 hover:underline">
          Back to reports
        </Link>
      </div>
      <div class="rounded border bg-white p-4 shadow-sm">
        <div class="text-sm text-gray-500">{formatTimestamp(data.createdAt)}</div>
        <div class="mt-2 text-sm font-semibold text-gray-700">Type: {data.reportType}</div>
        <pre class="mt-4 overflow-auto rounded bg-stone-950/95 p-4 text-xs text-stone-100">
          {JSON.stringify(data.payload, null, 2)}
        </pre>
      </div>
    </div>
  );
}
