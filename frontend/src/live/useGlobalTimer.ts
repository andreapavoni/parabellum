import { useEffect, useRef } from "preact/hooks";
import { secondsUntilIso, clockSkewMsFromServerTime } from "@/lib/time";
import type { GameContextResponse } from "@/types/api";

export function useGlobalTimer(
  gameContext: GameContextResponse | null,
  serverTimeObservedAtMs: number,
  onElapsed: () => void,
) {
  const onElapsedRef = useRef(onElapsed);
  onElapsedRef.current = onElapsed;
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!gameContext) return;

    const skewMs = clockSkewMsFromServerTime(gameContext.serverTime, serverTimeObservedAtMs);
    const timers: number[] = [];

    for (const item of gameContext.buildingQueue) {
      const secs = secondsUntilIso(item.finishesAt, { clockSkewMs: skewMs });
      if (secs > 0) timers.push(secs);
    }

    const s = gameContext.troopMovementSummary;
    const nextAts = [
      s.incomingAttacksNextAt,
      s.incomingRaidsNextAt,
      s.incomingReturnsReinforcementsNextAt,
      s.outgoingAttacksNextAt,
      s.outgoingRaidsNextAt,
      s.outgoingReinforcementsNextAt,
    ] as const;
    for (const nextAt of nextAts) {
      if (nextAt) {
        const secs = secondsUntilIso(nextAt, { clockSkewMs: skewMs });
        if (secs > 0) timers.push(secs);
      }
    }

    if (timers.length === 0) return;

    const ms = Math.min(...timers) * 1000;

    if (timeoutRef.current !== null) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => {
      onElapsedRef.current();
    }, ms);

    return () => {
      if (timeoutRef.current !== null) clearTimeout(timeoutRef.current);
    };
  }, [gameContext, serverTimeObservedAtMs]);

  useEffect(() => {
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        onElapsedRef.current();
      }
    };
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => document.removeEventListener("visibilitychange", onVisibilityChange);
  }, []);
}
