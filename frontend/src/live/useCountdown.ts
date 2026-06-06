import { useEffect, useRef, useState } from "preact/hooks";

const ZERO_RETRY_MAX = 5;
const ZERO_RETRY_DELAY_MS = 1200;

export function useCountdown(seconds: number, onElapsed?: () => void) {
  const [remaining, setRemaining] = useState(seconds);
  const notifiedRef = useRef(false);
  const zeroRetryCountRef = useRef(0);

  useEffect(() => {
    setRemaining(seconds);
    notifiedRef.current = false;
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
    if (!onElapsed || notifiedRef.current || remaining > 0) {
      return;
    }
    notifiedRef.current = true;
    onElapsed();
  }, [remaining, onElapsed]);

  useEffect(() => {
    if (!onElapsed || remaining > 0 || zeroRetryCountRef.current >= ZERO_RETRY_MAX) {
      return;
    }

    const retryTimer = window.setTimeout(() => {
      zeroRetryCountRef.current += 1;
      onElapsed();
    }, ZERO_RETRY_DELAY_MS);

    return () => window.clearTimeout(retryTimer);
  }, [remaining, onElapsed]);

  return remaining;
}
