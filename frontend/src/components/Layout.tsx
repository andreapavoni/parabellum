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

export function Layout(props: LayoutProps) {
  const [serverTime, setServerTime] = useState(props.bootstrap?.serverTime ?? Date.now() / 1000);

  useEffect(() => {
    setServerTime(props.bootstrap?.serverTime ?? Date.now() / 1000);
  }, [props.bootstrap?.serverTime]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setServerTime((value) => value + 1);
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  const serverClock = useMemo(() => {
    const date = new Date(serverTime * 1000);
    return [date.getHours(), date.getMinutes(), date.getSeconds()]
      .map((value) => value.toString().padStart(2, "0"))
      .join(":");
  }, [serverTime]);

  const village = props.bootstrap?.village;
  const villages = props.bootstrap?.villages ?? [];
  const player = props.bootstrap?.player;

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
            </div>

            {village ? (
              <div class="flex justify-center items-center py-2 bg-white flex-wrap px-2">
                <div class="res-item">{resourceLabel(village.resources.lumber, village.warehouseCapacity, "🌲")}</div>
                <div class="res-item">{resourceLabel(village.resources.clay, village.warehouseCapacity, "🧱")}</div>
                <div class="res-item">{resourceLabel(village.resources.iron, village.warehouseCapacity, "⛏️")}</div>
                <div class="res-item">{resourceLabel(village.resources.crop, village.granaryCapacity, "🌾")}</div>
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

      <main class="flex-grow container mx-auto">{props.children}</main>

      {player && villages.length > 0 ? (
        <aside class="mx-auto mt-4 mb-8 w-full max-w-5xl px-4">
          <VillagesList villages={villages} onSwitchVillage={props.onSwitchVillage} />
        </aside>
      ) : null}
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
    <div class="w-full max-w-[400px] md:w-56 border-t border-gray-200 md:border-none pt-4">
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
