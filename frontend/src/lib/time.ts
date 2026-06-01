export function formatDurationHms(totalSeconds: number): string {
  const normalized = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(normalized / 3600);
  const minutes = Math.floor((normalized % 3600) / 60);
  const seconds = normalized % 60;
  return [hours, minutes, seconds].map((value) => value.toString().padStart(2, "0")).join(":");
}

export function secondsUntilIso(
  timestamp: string,
  options?: { clockSkewMs?: number; nowMs?: number },
): number {
  const targetMs = new Date(timestamp).getTime();
  if (Number.isNaN(targetMs)) return 0;
  const nowMs = options?.nowMs ?? Date.now();
  const skewedNowMs = nowMs + (options?.clockSkewMs ?? 0);
  return Math.max(0, Math.floor((targetMs - skewedNowMs) / 1000));
}

export function clockSkewMsFromServerTime(serverTimeSeconds: number): number {
  return serverTimeSeconds * 1000 - Date.now();
}
