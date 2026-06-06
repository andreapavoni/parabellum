import { useEffect, useMemo, useState } from "preact/hooks";

export function useServerClock(serverTime?: number) {
  const [liveServerTime, setLiveServerTime] = useState(serverTime ?? Date.now() / 1000);

  useEffect(() => {
    setLiveServerTime(serverTime ?? Date.now() / 1000);
  }, [serverTime]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setLiveServerTime((value) => value + 1);
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  return useMemo(() => {
    const date = new Date(liveServerTime * 1000);
    return [date.getHours(), date.getMinutes(), date.getSeconds()]
      .map((value) => value.toString().padStart(2, "0"))
      .join(":");
  }, [liveServerTime]);
}
