import { useAppStore } from "@/state/appStore";
import { useMutation, useQueryClient } from "@tanstack/preact-query";
import { api } from "@/lib/api";
import { queryKeys } from "@/query/keys";

function useInvalidateGameState() {
  const queryClient = useQueryClient();

  const invalidateCurrentVillage = async (villageId: number | undefined) => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.gameContextFor(villageId) });
  };

  const invalidateBuildingCommand = async (slotId: number, villageId: number | undefined) => {
    await Promise.all([
      invalidateCurrentVillage(villageId),
      queryClient.invalidateQueries({ queryKey: queryKeys.building(villageId, slotId) }),
    ]);
  };

  const invalidateCurrentVillageBuildings = async (villageId: number | undefined) => {
    await Promise.all([
      invalidateCurrentVillage(villageId),
      queryClient.invalidateQueries({ queryKey: queryKeys.buildings(villageId) }),
    ]);
  };

  const invalidateReports = async (villageId: number | undefined) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.gameContextFor(villageId) }),
      queryClient.invalidateQueries({ queryKey: ["reports"] }),
      queryClient.invalidateQueries({ queryKey: ["report"] }),
    ]);
  };

  const invalidateMap = async (fieldId: number | undefined, villageId: number | undefined) => {
    await Promise.all([
      invalidateCurrentVillage(villageId),
      queryClient.invalidateQueries({ queryKey: ["mapRegion"] }),
      fieldId
        ? queryClient.invalidateQueries({ queryKey: queryKeys.mapField(fieldId) })
        : queryClient.invalidateQueries({ queryKey: ["mapField"] }),
    ]);
  };

  return {
    invalidateCurrentVillage,
    invalidateBuildingCommand,
    invalidateCurrentVillageBuildings,
    invalidateReports,
    invalidateMap,
  };
}

export function useRenameVillageMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillage, invalidateMap } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.renameVillage>[0]) => api.renameVillage(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await Promise.all([invalidateCurrentVillage(villageId), invalidateMap(undefined, villageId)]);
    },
  });
}

export function useAddBuildingMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.addBuilding>[0]) => api.addBuilding(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useUpgradeBuildingMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.upgradeBuilding>[0]) => api.upgradeBuilding(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useDowngradeBuildingMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const queryClient = useQueryClient();
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.downgradeBuilding>[0]) => api.downgradeBuilding(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await Promise.all([
        invalidateBuildingCommand(payload.slotId, villageId),
        queryClient.invalidateQueries({ queryKey: queryKeys.building(villageId, 19) }),
      ]);
    },
  });
}

export function useCancelBuildingConstructionMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.cancelBuildingConstruction>[0]) => api.cancelBuildingConstruction(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await invalidateCurrentVillageBuildings(villageId);
    },
  });
}

export function useTrainUnitsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.trainUnits>[0]) => api.trainUnits(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useAssignHeroPointsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const queryClient = useQueryClient();
  const { invalidateCurrentVillage } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.assignHeroPoints>[0]) => api.assignHeroPoints(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await Promise.all([
        invalidateCurrentVillage(villageId),
        queryClient.invalidateQueries({ queryKey: queryKeys.currentHero }),
      ]);
    },
  });
}

export function useResetHeroPointsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const queryClient = useQueryClient();
  const { invalidateCurrentVillage } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.resetHeroPoints>[0]) => api.resetHeroPoints(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await Promise.all([
        invalidateCurrentVillage(villageId),
        queryClient.invalidateQueries({ queryKey: queryKeys.currentHero }),
      ]);
    },
  });
}

export function useSetHeroResourceFocusMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const queryClient = useQueryClient();
  const { invalidateCurrentVillage } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.setHeroResourceFocus>[0]) => api.setHeroResourceFocus(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await Promise.all([
        invalidateCurrentVillage(villageId),
        queryClient.invalidateQueries({ queryKey: queryKeys.currentHero }),
      ]);
    },
  });
}

export function useReviveHeroMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const queryClient = useQueryClient();
  const { invalidateCurrentVillage } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.reviveHero>[0]) => api.reviveHero(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await Promise.all([
        invalidateCurrentVillage(villageId),
        queryClient.invalidateQueries({ queryKey: queryKeys.currentHero }),
      ]);
    },
  });
}

export function useResearchAcademyMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.researchAcademy>[0]) => api.researchAcademy(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useResearchSmithyMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.researchSmithy>[0]) => api.researchSmithy(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useSendResourcesMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand, invalidateReports } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.sendResources>[0]) => api.sendResources(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await Promise.all([invalidateBuildingCommand(payload.slotId, villageId), invalidateReports(villageId)]);
    },
  });
}

export function useCreateMarketplaceOfferMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.createMarketplaceOffer>[0]) => api.createMarketplaceOffer(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useAcceptMarketplaceOfferMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.acceptMarketplaceOffer>[0]) => api.acceptMarketplaceOffer(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useCancelMarketplaceOfferMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.cancelMarketplaceOffer>[0]) => api.cancelMarketplaceOffer(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await invalidateBuildingCommand(payload.slotId, villageId);
    },
  });
}

export function useSendTroopsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateBuildingCommand, invalidateReports } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.sendTroops>[0]) => api.sendTroops(payload, villageId),
    onSuccess: async (_result, payload, villageId) => {
      await Promise.all([invalidateBuildingCommand(payload.slotId, villageId), invalidateReports(villageId)]);
    },
  });
}

export function useRecallTroopsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.recallTroops>[0]) => api.recallTroops(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useReleaseReinforcementsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.releaseReinforcements>[0]) => api.releaseReinforcements(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useReleaseTrappedTroopsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.releaseTrappedTroops>[0]) => api.releaseTrappedTroops(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useDisbandTrappedTroopsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.disbandTrappedTroops>[0]) => api.disbandTrappedTroops(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useBuildTrapsMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.buildTraps>[0]) => api.buildTraps(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useCancelTroopMovementMutation() {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.cancelTroopMovement>[0]) => api.cancelTroopMovement(payload, villageId),
    onSuccess: (_result, _payload, villageId) => invalidateCurrentVillageBuildings(villageId),
  });
}

export function useFoundVillageMutation(fieldId?: number) {
  const villageId = useAppStore().session.currentVillageId;
  const { invalidateMap } = useInvalidateGameState();
  return useMutation({
    onMutate: () => villageId,
    mutationFn: (payload: Parameters<typeof api.foundVillage>[0]) => api.foundVillage(payload, villageId),
    onSuccess: async (_result, _payload, villageId) => {
      await invalidateMap(fieldId, villageId);
    },
  });
}
