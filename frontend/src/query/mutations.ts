import { useMutation, useQueryClient } from "@tanstack/preact-query";
import { api } from "@/lib/api";
import { useGameContextQuery } from "@/query/hooks";
import { queryKeys } from "@/query/keys";

function useInvalidateGameState() {
  const queryClient = useQueryClient();
  const gameContext = useGameContextQuery();

  const invalidateCurrentVillage = async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.gameContext });
  };

  const invalidateBuildingCommand = async (slotId: number) => {
    await Promise.all([
      invalidateCurrentVillage(),
      queryClient.invalidateQueries({ queryKey: queryKeys.building(slotId) }),
    ]);
  };

  const invalidateCurrentVillageBuildings = async () => {
    await Promise.all([
      invalidateCurrentVillage(),
      queryClient.invalidateQueries({ queryKey: ["building"] }),
    ]);
  };

  const invalidateReports = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.gameContext }),
      queryClient.invalidateQueries({ queryKey: ["reports"] }),
      queryClient.invalidateQueries({ queryKey: ["report"] }),
    ]);
  };

  const invalidateMap = async (fieldId?: number) => {
    await Promise.all([
      invalidateCurrentVillage(),
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
  const { invalidateCurrentVillage, invalidateMap } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.renameVillage,
    onSuccess: async () => {
      await Promise.all([invalidateCurrentVillage(), invalidateMap()]);
    },
  });
}

export function useAddBuildingMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.addBuilding,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useUpgradeBuildingMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.upgradeBuilding,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useDowngradeBuildingMutation() {
  const queryClient = useQueryClient();
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.downgradeBuilding,
    onSuccess: async (_result, payload) => {
      await Promise.all([
        invalidateBuildingCommand(payload.slotId),
        queryClient.invalidateQueries({ queryKey: queryKeys.building(19) }),
      ]);
    },
  });
}

export function useCancelBuildingConstructionMutation() {
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.cancelBuildingConstruction,
    onSuccess: async () => {
      await invalidateCurrentVillageBuildings();
    },
  });
}

export function useTrainUnitsMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.trainUnits,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useResearchAcademyMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.researchAcademy,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useResearchSmithyMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.researchSmithy,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useSendResourcesMutation() {
  const { invalidateBuildingCommand, invalidateReports } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.sendResources,
    onSuccess: async (_result, payload) => {
      await Promise.all([invalidateBuildingCommand(payload.slotId), invalidateReports()]);
    },
  });
}

export function useCreateMarketplaceOfferMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.createMarketplaceOffer,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useAcceptMarketplaceOfferMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.acceptMarketplaceOffer,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useCancelMarketplaceOfferMutation() {
  const { invalidateBuildingCommand } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.cancelMarketplaceOffer,
    onSuccess: async (_result, payload) => {
      await invalidateBuildingCommand(payload.slotId);
    },
  });
}

export function useSendTroopsMutation() {
  const { invalidateBuildingCommand, invalidateReports } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.sendTroops,
    onSuccess: async (_result, payload) => {
      await Promise.all([invalidateBuildingCommand(payload.slotId), invalidateReports()]);
    },
  });
}

export function useRecallTroopsMutation() {
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.recallTroops,
    onSuccess: invalidateCurrentVillageBuildings,
  });
}

export function useReleaseReinforcementsMutation() {
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.releaseReinforcements,
    onSuccess: invalidateCurrentVillageBuildings,
  });
}

export function useCancelTroopMovementMutation() {
  const { invalidateCurrentVillageBuildings } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.cancelTroopMovement,
    onSuccess: invalidateCurrentVillageBuildings,
  });
}

export function useFoundVillageMutation(fieldId?: number) {
  const { invalidateMap } = useInvalidateGameState();
  return useMutation({
    mutationFn: api.foundVillage,
    onSuccess: async () => {
      await invalidateMap(fieldId);
    },
  });
}
