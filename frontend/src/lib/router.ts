export type Route =
  | { name: "home" }
  | { name: "login" }
  | { name: "register" }
  | { name: "village" }
  | { name: "resources" }
  | { name: "building"; slotId: number }
  | { name: "map"; x?: number; y?: number }
  | { name: "mapField"; fieldId: number }
  | { name: "stats"; page: number }
  | { name: "player"; playerId: string }
  | { name: "reports" }
  | { name: "report"; reportId: string }
  | { name: "notFound" };

function parseRouteParts(path: string, search: URLSearchParams): Route {
  if (path === "/") return { name: "home" };
  if (path === "/login") return { name: "login" };
  if (path === "/register") return { name: "register" };
  if (path === "/village") return { name: "village" };
  if (path === "/resources") return { name: "resources" };
  if (/^\/app\/build\/\d+$/.test(path)) {
    return { name: "building", slotId: Number(path.split("/")[3]) };
  }
  if (path === "/map") {
    const rawX = search.get("x");
    const rawY = search.get("y");
    if (rawX !== null && rawY !== null) {
      const parsedX = Number(rawX);
      const parsedY = Number(rawY);
      if (Number.isFinite(parsedX) && Number.isFinite(parsedY)) {
        return { name: "map", x: parsedX, y: parsedY };
      }
    }
    return { name: "map" };
  }
  if (/^\/map\/field\/\d+$/.test(path)) {
    return { name: "mapField", fieldId: Number(path.split("/")[3]) };
  }
  if (path === "/stats") {
    return { name: "stats", page: Number(search.get("page") ?? "1") || 1 };
  }
  if (/^\/players\/[^/]+$/.test(path)) {
    return { name: "player", playerId: path.split("/")[2] };
  }
  if (path === "/reports") return { name: "reports" };
  if (/^\/reports\/[^/]+$/.test(path)) {
    return { name: "report", reportId: path.split("/")[2] };
  }

  return { name: "notFound" };
}

export function parseRoute(location: Location): Route {
  return parseRouteParts(location.pathname, new URLSearchParams(location.search));
}

export function navigate(path: string, replace = false) {
  if (replace) {
    window.history.replaceState(null, "", path);
  } else {
    window.history.pushState(null, "", path);
  }
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export function isProtectedRoute(route: Route) {
  return !["home", "login", "register", "notFound"].includes(route.name);
}

const LEGACY_ROUTE_PREFIXES = [
  "/army/",
  "/marketplace/",
  "/academy/",
  "/smithy/",
  "/village/switch/",
];

const LEGACY_ROUTE_EXACT = ["/logout"];

export function shouldUseClientNavigation(href: string): boolean {
  const url = new URL(href, window.location.href);
  if (url.origin !== window.location.origin) {
    return false;
  }

  if (LEGACY_ROUTE_EXACT.includes(url.pathname)) {
    return false;
  }

  if (LEGACY_ROUTE_PREFIXES.some((prefix) => url.pathname.startsWith(prefix))) {
    return false;
  }

  const route = parseRouteParts(url.pathname, new URLSearchParams(url.search));
  return route.name !== "notFound";
}
