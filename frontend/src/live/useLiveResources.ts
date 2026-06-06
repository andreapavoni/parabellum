import { useEffect, useState } from "preact/hooks";
import type { ResourceAmounts, VillageSummary } from "@/types/api";

export function useLiveResources(village?: VillageSummary | null): ResourceAmounts | null {
  const [liveResources, setLiveResources] = useState<ResourceAmounts | null>(null);

  useEffect(() => {
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
        return {
          lumber: Math.min(
            village.warehouseCapacity,
            Math.max(0, current.lumber + village.productionPerHour.lumber / 3600),
          ),
          clay: Math.min(
            village.warehouseCapacity,
            Math.max(0, current.clay + village.productionPerHour.clay / 3600),
          ),
          iron: Math.min(
            village.warehouseCapacity,
            Math.max(0, current.iron + village.productionPerHour.iron / 3600),
          ),
          crop: Math.min(
            village.granaryCapacity,
            Math.max(0, current.crop + village.productionPerHour.crop / 3600),
          ),
        };
      });
    }, 1000);

    return () => window.clearInterval(timer);
  }, [village]);

  return liveResources;
}
