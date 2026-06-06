import { formatDurationHms } from "@/lib/time";
import { useCountdown } from "@/live/useCountdown";

export function LiveCountdown({
  seconds,
  onElapsed,
}: {
  seconds: number;
  onElapsed?: () => void;
}) {
  const remaining = useCountdown(seconds, onElapsed);
  return <span class="font-mono">{formatDurationHms(remaining)}</span>;
}
