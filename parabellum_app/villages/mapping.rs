use super::models::VillageModel;
use parabellum_game::models::{
    army::Army,
    village::{Village, VillageSnapshot},
};
/// `VillageModel` intentionally carries only the village economy/read-model
/// fields. Callers that need troop-aware domain behavior must load this context
/// from the canonical army read model.
#[derive(Debug, Clone, Default)]
pub struct VillageArmyContext {
    pub home: Option<Army>,
    pub stationed: Vec<Army>,
    pub deployed: Vec<Army>,
    pub moving: Vec<Army>,
    pub trapped_here: Vec<Army>,
    pub trapped_away: Vec<Army>,
}

/// Hydrates a domain `Village` from the village read model plus explicit army
/// context.
pub fn hydrate_village(model: VillageModel, armies: VillageArmyContext) -> Village {
    let busy_merchants = model.busy_merchants;
    let mut village = Village::rehydrate(VillageSnapshot {
        id: model.village_id,
        name: model.village_name,
        player_id: model.player_id,
        position: model.position,
        tribe: model.tribe,
        buildings: model.buildings,
        oases: vec![],
        army: armies.home,
        reinforcements: armies.stationed,
        deployed_armies: armies.deployed,
        loyalty: model.loyalty,
        is_capital: model.is_capital,
        smithy: model.smithy_upgrades,
        stocks: model.stocks,
        academy_research: model.academy_research,
        culture_points: 0,
        updated_at: model.updated_at,
        parent_village_id: model.parent_village_id,
    });
    village.busy_merchants = busy_merchants.min(village.total_merchants);
    village
}

#[cfg(test)]
mod tests {
    use parabellum_game::models::village::{AcademyResearch, VillageProduction, VillageStocks};
    use parabellum_types::army::UnitName;
    use parabellum_types::{map::Position, tribe::Tribe};
    use uuid::Uuid;

    use crate::villages::VillageArmyContext;
    use crate::villages::models::VillageModel;

    #[test]
    fn village_model_maps_to_domain_village_with_projected_research_state() {
        let position = Position { x: 0, y: 0 };
        let village_id = position.to_id(100);
        let mut academy = AcademyResearch::default();
        academy.set(0, true);
        let model = VillageModel {
            village_id,
            player_id: Uuid::new_v4(),
            village_name: "v".to_string(),
            position,
            tribe: Tribe::Roman,
            buildings: vec![],
            production: VillageProduction::default(),
            stocks: VillageStocks::default(),
            population: 12,
            loyalty: 100,
            is_capital: true,
            culture_points_production: 0,
            smithy_upgrades: [1, 2, 0, 0, 0, 0, 0, 0],
            academy_research: academy.clone(),
            total_merchants: 0,
            busy_merchants: 0,
            trapper: Default::default(),
            loyalty_updated_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent_village_id: None,
        };

        let village = crate::villages::hydrate_village(model, VillageArmyContext::default());

        assert_eq!(village.id, village_id);
        assert_eq!(village.culture_points, 0);
        assert_eq!(village.culture_points_production, 0);
        assert_eq!(village.smithy()[0], 1);
        assert_eq!(village.smithy()[1], 2);
        let idx = village
            .tribe
            .get_unit_idx_by_name(&UnitName::Legionnaire)
            .unwrap();
        assert_eq!(village.academy_research().get(idx), academy.get(idx));
    }
}
