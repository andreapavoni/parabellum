import { afterEach, expect, test } from "bun:test";
import { QueryClient } from "@tanstack/query-core";
import { api } from "../lib/api";
import { queryKeys } from "./keys";

const originalFetch = globalThis.fetch;
afterEach(() => { globalThis.fetch = originalFetch; });
const json = (value: unknown, status = 200) => new Response(JSON.stringify(value), { status });
const token = (id: number) => ({ accessToken: `token-${id}`, refreshToken: "refresh", currentVillageId: id });
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
async function login() { await api.tokenLogin({ username: "test", password: "test" }); }

test("delayed village A response cannot populate village B's building cache", async () => {
  const a = deferred<Response>();
  const requested = deferred<void>();
  globalThis.fetch = (async (url, init) => {
    if (String(url).endsWith("/auth/token/login")) return json(token(1));
    if (String(url).endsWith("/me/village/current")) return json({ villageId: 2, accessToken: "token-2" });
    const id = new Headers(init?.headers).get("X-Village-Id");
    if (id === "1") { requested.resolve(); return a.promise; }
    return json({ villageId: 2 });
  }) as typeof fetch;
  await login();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  try {
    const first = client.fetchQuery({ queryKey: queryKeys.building(1, 19), queryFn: ({ signal }) => api.building(1, 19, signal) });
    await requested.promise;
    await api.switchVillage({ villageId: 2 });
    await client.fetchQuery({ queryKey: queryKeys.building(2, 19), queryFn: ({ signal }) => api.building(2, 19, signal) });
    a.resolve(json({ villageId: 1 }));
    await first;
    expect(client.getQueryData<{ villageId: number }>(queryKeys.building(1, 19))).toEqual({ villageId: 1 });
    expect(client.getQueryData<{ villageId: number }>(queryKeys.building(2, 19))).toEqual({ villageId: 2 });
  } finally { client.clear(); }
});

test("cancelling a building query aborts its HTTP request", async () => {
  const started = deferred<void>();
  let aborted = false;
  globalThis.fetch = (async (url, init) => {
    if (String(url).endsWith("/auth/token/login")) return json(token(1));
    return new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => { aborted = true; reject(new DOMException("Aborted", "AbortError")); });
      started.resolve();
    });
  }) as typeof fetch;
  await login();
  const client = new QueryClient();
  const result = client.fetchQuery({ queryKey: queryKeys.building(1, 19), queryFn: ({ signal }) => api.building(1, 19, signal) }).catch(() => undefined);
  await started.promise;
  await client.cancelQueries({ queryKey: queryKeys.buildings(1) });
  await result;
  expect(aborted).toBe(true);
  expect(client.getQueryData<{ villageId: number }>(queryKeys.building(1, 19))).toBeUndefined();
  client.clear();
});

test("a mutation retry keeps the village where it was submitted", async () => {
  const rejected = deferred<Response>();
  const started = deferred<void>();
  const targets: (string | null)[] = [];
  globalThis.fetch = (async (url, init) => {
    const path = String(url);
    if (path.endsWith("/auth/token/login")) return json(token(1));
    if (path.endsWith("/me/village/current")) return json({ villageId: 2, accessToken: "token-2" });
    if (path.endsWith("/auth/refresh")) return json(token(2));
    targets.push(new Headers(init?.headers).get("X-Village-Id"));
    if (targets.length === 1) { started.resolve(); return rejected.promise; }
    return json({ success: true });
  }) as typeof fetch;
  await login();
  const mutation = api.upgradeBuilding({ slotId: 19 });
  await started.promise;
  await api.switchVillage({ villageId: 2 });
  rejected.resolve(json({ code: "token_expired", message: "Expired" }, 401));
  await mutation;
  expect(targets).toEqual(["1", "1"]);
});

test("rapid village switches are serialized in selection order", async () => {
  const first = deferred<Response>();
  const started = deferred<void>();
  const targets: number[] = [];
  globalThis.fetch = (async (url, init) => {
    if (String(url).endsWith("/auth/token/login")) return json(token(1));
    const target = JSON.parse(String(init?.body)).villageId;
    targets.push(target);
    if (targets.length === 1) { started.resolve(); return first.promise; }
    return json({ villageId: target, accessToken: `token-${target}` });
  }) as typeof fetch;
  await login();
  const b = api.switchVillage({ villageId: 2 });
  const a = api.switchVillage({ villageId: 1 });
  await started.promise;
  expect(targets).toEqual([2]);
  first.resolve(json({ villageId: 2, accessToken: "token-2" }));
  await Promise.all([b, a]);
  expect(targets).toEqual([2, 1]);
  expect(api.currentVillageId()).toBe(1);
});

test("explicit mutation context survives a changed token village", async () => {
  const targets: (string | null)[] = [];
  globalThis.fetch = (async (url, init) => {
    if (String(url).endsWith("/auth/token/login")) return json(token(1));
    if (String(url).endsWith("/me/village/current")) return json({ villageId: 2, accessToken: "token-2" });
    targets.push(new Headers(init?.headers).get("X-Village-Id"));
    return json({ success: true });
  }) as typeof fetch;
  await login();
  await api.switchVillage({ villageId: 2 });
  await api.upgradeBuilding({ slotId: 19 }, 1);
  await api.cancelTroopMovement({ movementId: "pending" }, 1);
  expect(targets).toEqual(["1", "1"]);
});
