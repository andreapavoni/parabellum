import { useEffect, useRef, useState } from "preact/hooks";
import { formatDurationHms } from "@/lib/time";

export function LiveCountdown({
  seconds,
  onElapsed,
}: {
  seconds: number;
  onElapsed?: () => void;
}) {
  const ZERO_RETRY_MAX = 5;
  const ZERO_RETRY_DELAY_MS = 1200;
  const [remaining, setRemaining] = useState(seconds);
  const startedFromPositiveRef = useRef(seconds > 0);
  const notifiedRef = useRef(seconds <= 0);
  const zeroRetryCountRef = useRef(0);

  useEffect(() => {
    setRemaining(seconds);
    startedFromPositiveRef.current = seconds > 0;
    notifiedRef.current = seconds <= 0;
    if (seconds > 0) {
      zeroRetryCountRef.current = 0;
    }
  }, [seconds]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setRemaining((value) => Math.max(0, value - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (
      !onElapsed ||
      notifiedRef.current ||
      remaining > 0 ||
      !startedFromPositiveRef.current
    ) {
      return;
    }
    notifiedRef.current = true;
    onElapsed();
  }, [remaining, onElapsed]);

  useEffect(() => {
    if (
      !onElapsed ||
      remaining > 0 ||
      startedFromPositiveRef.current ||
      zeroRetryCountRef.current >= ZERO_RETRY_MAX
    ) {
      return;
    }

    const retryTimer = window.setTimeout(() => {
      zeroRetryCountRef.current += 1;
      onElapsed();
    }, ZERO_RETRY_DELAY_MS);

    return () => window.clearTimeout(retryTimer);
  }, [remaining, onElapsed]);

  return <span class="font-mono">{formatDurationHms(remaining)}</span>;
}

