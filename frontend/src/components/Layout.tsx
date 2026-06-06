import type { ComponentChildren } from "preact";
import { BarChart3, FileText, Home, LogOut, Mail, Map, Users, Wheat } from "lucide-preact";
import type { GameShellContext, SessionResponse } from "@/types/api";
import { ResourceSprite, type ResourceSpriteKind } from "@/components/ResourceSprite";
import { VillageSelector } from "@/components/VillageHeader";
import { useLiveResources } from "@/live/useLiveResources";
import { useServerClock } from "@/live/useServerClock";
import { Link } from "./Link";
import { Button, Panel } from "./ui";

type LayoutProps = {
  session: SessionResponse;
  meContext: GameShellContext | null;
  onLogout: () => void;
  onSwitchVillage: (villageId: number) => void;
  active: string;
  children: ComponentChildren;
};

function resourceLabel(value: number, capacity: number, kind: ResourceSpriteKind, label: string) {
  return (
    <span class="inline-flex items-center gap-1">
      <ResourceSprite kind={kind} size={14} label={label} />
      {value}/{capacity}
    </span>
  );
}

export function Layout(props: LayoutProps) {
  const village = props.meContext?.currentVillage;
  const serverClock = useServerClock(props.meContext?.serverTime);
  const liveResources = useLiveResources(village);
  const hasUnreadReports = Boolean(props.meContext && props.meContext.unreadReportsCount > 0);
  const villages = props.meContext?.villages ?? [];
  const player = props.meContext?.player;
  const isGuestHome = !player && props.active === "home";
  const showVillageSwitcher =
    Boolean(player) &&
    props.active === "building" &&
    villages.length > 0;
  const mapHref = village ? `/map?x=${village.x}&y=${village.y}` : "/map";

  return (
    <>
      {!isGuestHome ? <header class="bg-white border-b border-gray-300 shadow-sm">
        {player ? (
          <>
            <div class="flex justify-between items-center px-4 py-2 bg-stone-100 border-b border-stone-200 text-xs">
              <div class="font-serif font-bold text-lg text-stone-800 tracking-wide">
                <Link to="/">PARABELLUM</Link>
              </div>
              <div class="flex items-center gap-3 text-stone-600">
                <span class="font-bold text-stone-800">{player.username}</span>
                <Button type="button" variant="ghost" size="sm" onClick={props.onLogout}>
                  <LogOut size={13} aria-hidden="true" />
                  Logout
                </Button>
                <span class="sm:inline text-[12px] text-stone-600 font-mono">{serverClock}</span>
              </div>
            </div>

            <div class="flex justify-center space-x-2 md:space-x-3 py-2 bg-stone-50 border-b border-stone-200 px-2 overflow-x-auto scrollbar-hide">
              <NavIcon active={props.active === "resources"} to="/resources" glyph={<Wheat size={17} aria-hidden="true" />} label="Fields" />
              <NavIcon active={props.active === "village"} to="/village" glyph={<Home size={17} aria-hidden="true" />} label="Village" />
              <NavIcon active={props.active === "map"} to={mapHref} glyph={<Map size={17} aria-hidden="true" />} label="Map" />
              <NavIcon active={props.active === "stats"} to="/stats" glyph={<BarChart3 size={17} aria-hidden="true" />} label="Ranks" />
              <NavIcon
                active={props.active === "reports"}
                alert={hasUnreadReports}
                to="/reports"
                glyph={<FileText size={17} aria-hidden="true" />}
                label="Reports"
              />
              <div class="nav-icon" title="Messages">
                <span class="nav-glyph"><Mail size={17} aria-hidden="true" /></span>
              </div>
            </div>

            {village ? (
              <div class="flex justify-center items-center py-2 bg-white flex-wrap px-2">
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.lumber ?? village.resources.lumber), village.warehouseCapacity, "lumber", "Lumber")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.clay ?? village.resources.clay), village.warehouseCapacity, "clay", "Clay")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.iron ?? village.resources.iron), village.warehouseCapacity, "iron", "Iron")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.crop ?? village.resources.crop), village.granaryCapacity, "crop", "Crop")}</div>
                <div class="res-item">
                  <span class="inline-flex items-center gap-1">
                    <Users size={14} aria-label="Population" />
                    {village.population}
                  </span>
                </div>
              </div>
            ) : null}
          </>
        ) : (
          <div class="container mx-auto flex justify-between items-center p-4">
            <div class="font-serif font-bold text-2xl text-gray-700 tracking-wide">
              <Link to="/">PARABELLUM</Link>
            </div>
            <div class="space-x-4 text-sm font-bold text-gray-600">
              <Link to="/login" class="hover:text-green-600 transition">
                Login
              </Link>
              <Link to="/register" class="text-green-700 hover:underline">
                Register
              </Link>
            </div>
          </div>
        )}
      </header> : null}

      {player ? (
        <div class={`mx-auto w-full max-w-6xl px-3 mt-3 mb-6 ${showVillageSwitcher ? "md:flex md:items-start md:gap-6" : ""}`}>
          <main class="flex-grow min-w-0">{props.children}</main>
          {showVillageSwitcher ? (
            <aside class="w-full mt-4 md:mt-0 md:w-56 md:shrink-0">
              <Panel>
                <VillageSelector villages={villages} onSwitchVillage={props.onSwitchVillage} />
              </Panel>
            </aside>
          ) : null}
        </div>
      ) : (
        isGuestHome ? <main class="flex-grow">{props.children}</main> : <main class="flex-grow container mx-auto">{props.children}</main>
      )}

      {!isGuestHome ? <footer class="bg-white border-t border-stone-200 py-4 text-center text-xs text-stone-400">
        <p>
          A{" "}
          <a class="hover:underline" href="https://pavonz.com">
            pavonz
          </a>{" "}
          joint | © 2025-2026 |{" "}
          <a class="hover:underline" href="https://github.com/andreapavoni/parabellum">
            Github
          </a>
        </p>
        <div class="mt-2 space-x-3">
          <span>Not affiliated with Travian Games GmbH</span>
        </div>
      </footer> : null}
    </>
  );
}

function NavIcon({
  active,
  alert,
  to,
  glyph,
  label,
}: {
  active: boolean;
  alert?: boolean;
  to: string;
  glyph: ComponentChildren;
  label: string;
}) {
  const className = [
    "nav-icon",
    active ? "nav-active" : "",
    alert && !active ? "nav-unread" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div class={className} title={label}>
      <Link to={to}>
        <span class="nav-glyph">{glyph}</span>
      </Link>
    </div>
  );
}
