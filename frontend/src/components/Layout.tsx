import type { ComponentChildren } from "preact";
import { useEffect, useMemo, useState } from "preact/hooks";
import type { BootstrapResponse, SessionResponse, VillageListItem } from "@/types/api";
import { Link } from "./Link";

type LayoutProps = {
  session: SessionResponse;
  bootstrap: BootstrapResponse | null;
  onLogout: () => void;
  onSwitchVillage: (villageId: number) => void;
  active: string;
  children: ComponentChildren;
};

function resourceLabel(value: number, capacity: number, icon: string) {
  return `${icon} ${value}/${capacity}`;
}

type LiveResources = {
  lumber: number;
  clay: number;
  iron: number;
  crop: number;
};

export function Layout(props: LayoutProps) {
  const [serverTime, setServerTime] = useState(props.bootstrap?.serverTime ?? Date.now() / 1000);
  const [liveResources, setLiveResources] = useState<LiveResources | null>(null);

  useEffect(() => {
    setServerTime(props.bootstrap?.serverTime ?? Date.now() / 1000);
  }, [props.bootstrap?.serverTime]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setServerTime((value) => value + 1);
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    const village = props.bootstrap?.village;
    if (!village) {
      setLiveResources(null);
      return;
    }

    setLiveResources({
      lumber: village.resources.lumber,
      clay: village.resources.clay,
      iron: village.resources.iron,
      crop: village.resources.crop,
    });

    const timer = window.setInterval(() => {
      setLiveResources((current) => {
        if (!current) return current;
        const next = {
          lumber: Math.min(village.warehouseCapacity, Math.max(0, current.lumber + village.productionPerHour.lumber / 3600)),
          clay: Math.min(village.warehouseCapacity, Math.max(0, current.clay + village.productionPerHour.clay / 3600)),
          iron: Math.min(village.warehouseCapacity, Math.max(0, current.iron + village.productionPerHour.iron / 3600)),
          crop: Math.min(village.granaryCapacity, Math.max(0, current.crop + village.productionPerHour.crop / 3600)),
        };
        return next;
      });
    }, 1000);

    return () => window.clearInterval(timer);
  }, [props.bootstrap?.village]);

  const serverClock = useMemo(() => {
    const date = new Date(serverTime * 1000);
    return [date.getHours(), date.getMinutes(), date.getSeconds()]
      .map((value) => value.toString().padStart(2, "0"))
      .join(":");
  }, [serverTime]);

  const village = props.bootstrap?.village;
  const villages = props.bootstrap?.villages ?? [];
  const player = props.bootstrap?.player;
  const showVillageSwitcher = Boolean(player) && props.active === "village" && villages.length > 0;

  return (
    <>
      <header class="bg-white border-b border-gray-300 shadow-sm">
        {player ? (
          <>
            <div class="flex justify-between items-center px-4 py-1 bg-gray-200 border-b border-gray-300 text-xs">
              <div class="font-serif font-bold text-lg text-gray-700 tracking-wide">
                <Link to="/">PARABELLUM</Link>
              </div>
              <div class="flex items-center gap-3 text-gray-600">
                <span class="font-bold text-gray-800">{player.username}</span>
                <button class="cursor-pointer font-bold hover:text-green-600 text-green-700 hover:underline" onClick={props.onLogout}>
                  Logout
                </button>
                <span class="sm:inline text-[12px] text-gray-600 font-mono">{serverClock}</span>
              </div>
            </div>

            <div class="flex justify-center space-x-2 md:space-x-3 py-3 bg-gray-100 border-b border-gray-300 px-2 overflow-x-auto scrollbar-hide">
              <NavIcon active={props.active === "resources"} to="/resources" label="🌾" />
              <NavIcon active={props.active === "village"} to="/village" label="🏠" />
              <NavIcon active={props.active === "map"} to="/map" label="🗺️" />
              <NavIcon active={props.active === "stats"} to="/stats" label="📊" />
              <NavIcon active={props.active === "reports"} to="/reports" label="📜" />
              <div class="nav-icon" title="Messages">
                ✉️
              </div>
            </div>

            {village ? (
              <div class="flex justify-center items-center py-2 bg-white flex-wrap px-2">
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.lumber ?? village.resources.lumber), village.warehouseCapacity, "🌲")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.clay ?? village.resources.clay), village.warehouseCapacity, "🧱")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.iron ?? village.resources.iron), village.warehouseCapacity, "⛏️")}</div>
                <div class="res-item">{resourceLabel(Math.floor(liveResources?.crop ?? village.resources.crop), village.granaryCapacity, "🌾")}</div>
                <div class="res-item">👤 {village.population}</div>
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
      </header>

      {player ? (
        <div class={`mx-auto w-full max-w-6xl px-4 mt-4 mb-8 ${showVillageSwitcher ? "md:flex md:items-start md:gap-8" : ""}`}>
          <main class="flex-grow min-w-0">{props.children}</main>
          {showVillageSwitcher ? (
            <aside class="w-full mt-4 md:mt-0 md:w-56 md:shrink-0">
              <VillagesList villages={villages} onSwitchVillage={props.onSwitchVillage} />
            </aside>
          ) : null}
        </div>
      ) : (
        <main class="flex-grow container mx-auto">{props.children}</main>
      )}

      <footer class="bg-white border-t border-gray-300 py-4 text-center text-xs text-gray-400">
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
      </footer>
    </>
  );
}

function NavIcon({ active, to, label }: { active: boolean; to: string; label: string }) {
  return (
    <div class={active ? "nav-icon nav-active" : "nav-icon"}>
      <Link to={to}>{label}</Link>
    </div>
  );
}

function VillagesList({
  villages,
  onSwitchVillage,
}: {
  villages: VillageListItem[];
  onSwitchVillage: (villageId: number) => void;
}) {
  return (
    <div class="w-full max-w-[400px] md:w-56 border-t border-gray-200 md:border-none pt-4 md:pt-0">
      <h3 class="font-bold mb-3 text-sm border-b border-gray-300 pb-2">Villages:</h3>
      <ul class="text-xs space-y-2 list-none pl-0">
        {villages.map((village) => (
          <li
            key={village.id}
            class={
              village.isCurrent
                ? "flex justify-between items-center p-1 rounded font-bold bg-gray-100 cursor-default"
                : "p-1 rounded hover:bg-gray-100"
            }
          >
            {village.isCurrent ? (
              <>
                <span class="flex items-center">
                  <span class="w-2 h-2 rounded-full mr-2 bg-orange-500" />
                  {village.name}
                </span>
                <span class="text-gray-600">
                  ({village.x}|{village.y})
                </span>
              </>
            ) : (
              <button
                class="flex justify-between items-center w-full text-left bg-transparent border-0 p-0"
                onClick={() => onSwitchVillage(village.id)}
              >
                <span class="flex items-center">
                  <span class="w-2 h-2 rounded-full mr-2 bg-green-500" />
                  {village.name}
                </span>
                <span class="text-gray-500">
                  ({village.x}|{village.y})
                </span>
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
