import { useEffect, useState } from "preact/hooks";
import { api } from "@/lib/api";
import { formatDurationHms, secondsUntilIso } from "@/lib/time";
import { Link } from "@/components/Link";
import { ResourceSprite } from "@/components/ResourceSprite";
import { LiveCountdown } from "@/components/buildings/buildingShared";
import { Button, DataTable, Panel, SectionHeader } from "@/components/ui";
import {
  useAcceptMarketplaceOfferMutation,
  useCancelMarketplaceOfferMutation,
  useCreateMarketplaceOfferMutation,
  useSendResourcesMutation,
} from "@/query/mutations";
import type { BuildingPageResponse, MarketplaceOffer, ResourceAmounts } from "@/types/api";

type ResourceKey = "lumber" | "clay" | "iron" | "crop";

const RESOURCE_KEYS: ResourceKey[] = ["lumber", "clay", "iron", "crop"];
const RESOURCE_LABELS: Record<ResourceKey, string> = {
  lumber: "Lumber",
  clay: "Clay",
  iron: "Iron",
  crop: "Crop",
};

function nonZeroResourceKeys(resources: ResourceAmounts): ResourceKey[] {
  return RESOURCE_KEYS.filter((key) => resources[key] > 0);
}

function isValidMarketplaceOfferShape(offerResources: ResourceAmounts, seekResources: ResourceAmounts): boolean {
  const offerKeys = nonZeroResourceKeys(offerResources);
  const seekKeys = nonZeroResourceKeys(seekResources);
  if (offerKeys.length !== 1 || seekKeys.length !== 1) return false;
  if (offerKeys[0] === seekKeys[0]) return false;
  const offerTotal = offerResources.lumber + offerResources.clay + offerResources.iron + offerResources.crop;
  const seekTotal = seekResources.lumber + seekResources.clay + seekResources.iron + seekResources.crop;
  if (offerTotal <= 0 || seekTotal <= 0) return false;
  const maxSide = Math.max(offerTotal, seekTotal);
  const minSide = Math.min(offerTotal, seekTotal);
  return maxSide <= minSide * 3;
}

function formatRelativeTime(timestamp: number) {
  const now = Date.now();
  const diffMs = now - timestamp * 1000;
  const diffSec = Math.max(0, Math.floor(diffMs / 1000));
  if (diffSec < 60) return "just now";
  if (diffSec < 3600) {
    const mins = Math.floor(diffSec / 60);
    return `${mins} minute${mins === 1 ? "" : "s"} ago`;
  }
  if (diffSec < 86_400) {
    const hours = Math.floor(diffSec / 3600);
    return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  }
  const days = Math.floor(diffSec / 86_400);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

function ResourceAmountsInline({ resources }: { resources: ResourceAmounts }) {
  return (
    <span class="inline-flex items-center gap-2">
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="lumber" size={12} label="Lumber" />{resources.lumber}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="clay" size={12} label="Clay" />{resources.clay}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="iron" size={12} label="Iron" />{resources.iron}</span>
      <span class="inline-flex items-center gap-1"><ResourceSprite kind="crop" size={12} label="Crop" />{resources.crop}</span>
    </span>
  );
}

function OffersTable({
  title,
  offers,
  actionLabel,
  enforceTradeRules,
  onAction,
}: {
  title: string;
  offers: MarketplaceOffer[];
  actionLabel: string;
  enforceTradeRules?: boolean;
  onAction: (offer: MarketplaceOffer) => Promise<void>;
}) {
  return (
    <Panel class="space-y-3">
      <SectionHeader title={title} />
      {offers.length === 0 ? (
        <p class="text-sm text-gray-500">No offers available.</p>
      ) : (
        <DataTable>
            <thead class="text-left text-xs uppercase text-gray-500 border-b">
              <tr>
                <th class="py-2 pr-4">Village</th>
                <th class="py-2 pr-4">Offering</th>
                <th class="py-2 pr-4">Seeking</th>
                <th class="py-2 pr-4">Merchants</th>
                <th class="py-2 pr-4">Created</th>
                <th class="py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {offers.map((offer) => (
                <tr key={offer.offerId} class="border-b last:border-b-0">
                  <td class="py-2 pr-4">
                    <Link to={`/map/field/${offer.villageId}`} class="text-green-700 hover:underline">
                      {offer.villageName} ({offer.position.x}|{offer.position.y})
                    </Link>
                  </td>
                  <td class="py-2 pr-4"><ResourceAmountsInline resources={offer.offerResources} /></td>
                  <td class="py-2 pr-4"><ResourceAmountsInline resources={offer.seekResources} /></td>
                  <td class="py-2 pr-4">{offer.merchantsRequired}</td>
                  <td class="py-2 pr-4 text-gray-600">{formatRelativeTime(offer.createdAt)}</td>
                  <td class="py-2">
                    {(() => {
                      const invalidTrade =
                        Boolean(enforceTradeRules) &&
                        !isValidMarketplaceOfferShape(offer.offerResources, offer.seekResources);
                      return (
                        <Button
                          type="button"
                          variant={actionLabel === "Cancel" ? "danger" : "primary"}
                          size="sm"
                          class={invalidTrade ? "cursor-not-allowed border-stone-400 bg-stone-400 hover:bg-stone-400" : ""}
                          disabled={invalidTrade}
                          title={invalidTrade ? "Invalid offer rules (single-resource and ratio constraints)." : undefined}
                          onClick={() => onAction(offer)}
                        >
                          {actionLabel}
                        </Button>
                      );
                    })()}
                  </td>
                </tr>
              ))}
            </tbody>
        </DataTable>
      )}
    </Panel>
  );
}

export function MarketplaceBuilding({
  detail,
  initialTargetX,
  initialTargetY,
  onMutate,
}: {
  detail: BuildingPageResponse["detail"];
  initialTargetX: number;
  initialTargetY: number;
  onMutate: () => Promise<void>;
}) {
  const [send, setSend] = useState({
    targetX: initialTargetX,
    targetY: initialTargetY,
    lumber: 0,
    clay: 0,
    iron: 0,
    crop: 0,
  });
  const [offer, setOffer] = useState({
    offerResource: "lumber" as ResourceKey,
    offerAmount: 0,
    seekResource: "clay" as ResourceKey,
    seekAmount: 0,
  });
  const [error, setError] = useState<string | null>(null);
  const [previewingSend, setPreviewingSend] = useState(false);
  const [sending, setSending] = useState(false);
  const [sendPreview, setSendPreview] = useState<{ arrivesAt: string } | null>(null);
  const [sendPreviewStartedAtMs, setSendPreviewStartedAtMs] = useState<number | null>(null);
  const [sendPreviewTravelSeconds, setSendPreviewTravelSeconds] = useState(0);
  const [sendPreviewTick, setSendPreviewTick] = useState(0);
  const sendResources = useSendResourcesMutation();
  const createOffer = useCreateMarketplaceOfferMutation();
  const cancelOffer = useCancelMarketplaceOfferMutation();
  const acceptOffer = useAcceptMarketplaceOfferMutation();

  useEffect(() => {
    if (!sendPreview) return;
    const timer = window.setInterval(() => setSendPreviewTick((v) => v + 1), 1000);
    return () => window.clearInterval(timer);
  }, [sendPreview]);

  if (!detail.marketplace) return null;

  return (
    <>
      <Panel class="space-y-4">
        <div>
          <SectionHeader title="Send resources" class="mb-1" />
          <p class="text-sm text-gray-500">
            Available merchants: {detail.marketplace.availableMerchants}/{detail.marketplace.totalMerchants}
          </p>
          <p class="text-sm text-gray-500">
            Capacity per merchant: {detail.marketplace.merchantCapacity}
          </p>
          <p class="text-sm text-gray-500">
            Merchant speed: {detail.marketplace.merchantSpeed} fields/hour
          </p>
        </div>
        <div class="flex flex-wrap items-end gap-2 text-sm">
          <label class="text-gray-600">
            X
            <input
              type="number"
              value={send.targetX}
              onInput={(e) => setSend((v) => ({ ...v, targetX: Number((e.target as HTMLInputElement).value || "0") }))}
              class="mt-1 w-20 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          <label class="text-gray-600">
            Y
            <input
              type="number"
              value={send.targetY}
              onInput={(e) => setSend((v) => ({ ...v, targetY: Number((e.target as HTMLInputElement).value || "0") }))}
              class="mt-1 w-20 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          {(["lumber", "clay", "iron", "crop"] as const).map((key) => (
            <label key={key} class="inline-flex items-center gap-1 text-gray-600">
              <ResourceSprite kind={key} size={14} label={RESOURCE_LABELS[key]} />
              <input
                type="number"
                min="0"
                value={send[key]}
                onInput={(e) =>
                  setSend((v) => ({ ...v, [key]: Math.max(0, Number((e.target as HTMLInputElement).value || "0")) }))
                }
                class="w-20 border rounded px-2 py-1.5 text-gray-700"
              />
            </label>
          ))}
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={previewingSend || sending}
            onClick={async () => {
              setError(null);
              try {
                setPreviewingSend(true);
                const preview = await api.previewSendResources({ slotId: detail.slotId, ...send });
                setSendPreview(preview);
                setSendPreviewStartedAtMs(Date.now());
                setSendPreviewTravelSeconds(secondsUntilIso(preview.arrivesAt));
              } catch (err) {
                setError((err as Error).message);
              } finally {
                setPreviewingSend(false);
              }
            }}
          >
            {previewingSend ? "Calculating..." : "Preview"}
          </Button>
        </div>
        {sendPreview ? (
          <div class="rounded-md border border-green-200 bg-green-50 p-3 space-y-2 text-sm">
            {(() => {
              void sendPreviewTick;
              void sendPreviewStartedAtMs;
              const dynamicArrivesAt = new Date(Date.now() + sendPreviewTravelSeconds * 1000);
              return (
                <div>
                  Arrives at: <span class="font-semibold">{dynamicArrivesAt.toLocaleString()}</span>
                </div>
              );
            })()}
            <div>
              Arrives in: <span class="font-semibold">{formatDurationHms(sendPreviewTravelSeconds)}</span>
            </div>
            <Button
              type="button"
              size="sm"
              disabled={sending}
              onClick={async () => {
                setError(null);
                try {
                  setSending(true);
                  await sendResources.mutateAsync({ slotId: detail.slotId, ...send });
                  setSendPreview(null);
                } catch (err) {
                  setError((err as Error).message);
                } finally {
                  setSending(false);
                }
              }}
            >
              {sending ? "Sending..." : "Confirm and send"}
            </Button>
          </div>
        ) : null}
      </Panel>

      <Panel class="space-y-4">
        <div>
          <SectionHeader title="Create offer" class="mb-1" />
          <p class="text-sm text-gray-500">
            One resource type per side. Offer and seek resources must be different. Ratio must stay
            between 1:3 and 3:1.
          </p>
        </div>
        <div class="flex flex-wrap items-end gap-2 text-sm">
          <label class="inline-flex items-center gap-1 text-gray-600">
            <ResourceSprite kind={offer.offerResource} size={14} label="Offer" />
            <select
              value={offer.offerResource}
              onChange={(e) =>
                setOffer((v) => ({ ...v, offerResource: (e.target as HTMLSelectElement).value as ResourceKey }))
              }
              class="border rounded px-2 py-1.5 text-gray-700"
            >
              {RESOURCE_KEYS.map((key) => (
                <option value={key} key={key}>
                  {RESOURCE_LABELS[key]}
                </option>
              ))}
            </select>
            <input
              type="number"
              min="0"
              value={offer.offerAmount}
              onInput={(e) =>
                setOffer((v) => ({
                  ...v,
                  offerAmount: Math.max(0, Number((e.target as HTMLInputElement).value || "0")),
                }))
              }
              class="w-24 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          <span class="text-gray-500">for</span>
          <label class="inline-flex items-center gap-1 text-gray-600">
            <ResourceSprite kind={offer.seekResource} size={14} label="Seek" />
            <select
              value={offer.seekResource}
              onChange={(e) =>
                setOffer((v) => ({ ...v, seekResource: (e.target as HTMLSelectElement).value as ResourceKey }))
              }
              class="border rounded px-2 py-1.5 text-gray-700"
            >
              {RESOURCE_KEYS.map((key) => (
                <option value={key} key={key}>
                  {RESOURCE_LABELS[key]}
                </option>
              ))}
            </select>
            <input
              type="number"
              min="0"
              value={offer.seekAmount}
              onInput={(e) =>
                setOffer((v) => ({
                  ...v,
                  seekAmount: Math.max(0, Number((e.target as HTMLInputElement).value || "0")),
                }))
              }
              class="w-24 border rounded px-2 py-1.5 text-gray-700"
            />
          </label>
          <Button
            type="button"
            size="sm"
            onClick={async () => {
              setError(null);
              try {
                if (offer.offerAmount <= 0 || offer.seekAmount <= 0) {
                  throw new Error("Offer and seek amounts must be greater than zero.");
                }
                if (offer.offerResource === offer.seekResource) {
                  throw new Error("Offer and seek resources must be different.");
                }
                const maxSide = Math.max(offer.offerAmount, offer.seekAmount);
                const minSide = Math.min(offer.offerAmount, offer.seekAmount);
                if (maxSide > minSide * 3) {
                  throw new Error("Offer ratio must stay between 1:3 and 3:1.");
                }

                await createOffer.mutateAsync({
                  slotId: detail.slotId,
                  offerLumber: offer.offerResource === "lumber" ? offer.offerAmount : 0,
                  offerClay: offer.offerResource === "clay" ? offer.offerAmount : 0,
                  offerIron: offer.offerResource === "iron" ? offer.offerAmount : 0,
                  offerCrop: offer.offerResource === "crop" ? offer.offerAmount : 0,
                  seekLumber: offer.seekResource === "lumber" ? offer.seekAmount : 0,
                  seekClay: offer.seekResource === "clay" ? offer.seekAmount : 0,
                  seekIron: offer.seekResource === "iron" ? offer.seekAmount : 0,
                  seekCrop: offer.seekResource === "crop" ? offer.seekAmount : 0,
                });
              } catch (err) {
                setError((err as Error).message);
              }
            }}
          >
            Create offer
          </Button>
        </div>
      </Panel>

      <OffersTable title="Your offers" offers={detail.marketplace.ownOffers} actionLabel="Cancel" onAction={async (offer) => {
        await cancelOffer.mutateAsync({ offerId: offer.offerId, slotId: detail.slotId });
      }} />
      <OffersTable title="Global marketplace" offers={detail.marketplace.globalOffers} actionLabel="Accept" enforceTradeRules onAction={async (offer) => {
        await acceptOffer.mutateAsync({ offerId: offer.offerId, slotId: detail.slotId });
      }} />

      <Panel class="space-y-3">
        <SectionHeader title="Merchant movements" />
        {detail.marketplace.merchantMovements.length === 0 ? (
          <p class="text-sm text-gray-500">No merchant movements to display.</p>
        ) : (
          <DataTable>
              <thead class="text-left text-xs uppercase text-gray-500 border-b">
                <tr>
                  <th class="py-2 pr-4">Direction</th>
                  <th class="py-2 pr-4">Route</th>
                  <th class="py-2 pr-4">Resources</th>
                  <th class="py-2 pr-4">Merchants</th>
                  <th class="py-2">Arrives</th>
                </tr>
              </thead>
              <tbody>
                {detail.marketplace.merchantMovements.map((movement) => {
                  const origin = movement.originPosition
                    ? `${movement.originName} (${movement.originPosition.x}|${movement.originPosition.y})`
                    : movement.originName;
                  const destination = movement.destinationPosition
                    ? `${movement.destinationName} (${movement.destinationPosition.x}|${movement.destinationPosition.y})`
                    : movement.destinationName;
                  return (
                    <tr key={movement.jobId} class="border-b last:border-b-0">
                      <td class="py-2 pr-4 text-gray-700">
                        {movement.direction} ({movement.kind})
                      </td>
                      <td class="py-2 pr-4">{origin} → {destination}</td>
                      <td class="py-2 pr-4"><ResourceAmountsInline resources={movement.resources} /></td>
                      <td class="py-2 pr-4">{movement.merchantsUsed}</td>
                      <td class="py-2 font-mono text-gray-600">
                        <LiveCountdown
                          seconds={secondsUntilIso(movement.arrivesAt)}
                          onElapsed={() => {
                            void onMutate();
                          }}
                        />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
          </DataTable>
        )}
      </Panel>

      {error ? <div class="text-sm text-red-600">{error}</div> : null}
    </>
  );
}
