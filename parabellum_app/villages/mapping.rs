use super::models::VillageModel;
use parabellum_game::models::{army::Army, village::Village};
/// `VillageModel` intentionally carries only the village economy/read-model
/// fields. Callers that need troop-aware domain behavior must load this context
/// from the canonical army read model.
#[derive(Debug, Clone, Default)]
pub struct VillageArmyContext {
    pub home: Option<Army>,
    pub stationed: Vec<Army>,
    pub deployed: Vec<Army>,
    pub moving: Vec<Army>,
}

/// Hydrates a domain `Village` from the village read model plus explicit army
/// context.
pub fn hydrate_village(model: VillageModel, armies: VillageArmyContext) -> Village {
    let busy_merchants = model.busy_merchants;
    let mut village = Village::from_persistence(
        model.village_id,
        model.village_name,
        model.player_id,
        model.position,
        model.tribe,
        model.buildings,
        vec![],
        model.population,
        armies.home,
        armies.stationed,
        armies.deployed,
        model.loyalty,
        model.production,
        model.is_capital,
        model.smithy_upgrades,
        model.stocks,
        model.academy_research,
        0,
        model.culture_points_production,
        model.updated_at,
        model.parent_village_id,
    );
    village.busy_merchants = busy_merchants.min(village.total_merchants);
    village
}

impl From<VillageModel> for parabellum_game::models::village::Village {
    fn from(model: VillageModel) -> Self {
        hydrate_village(model, VillageArmyContext::default())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_game::models::village::{AcademyResearch, VillageProduction, VillageStocks};
    use parabellum_types::army::UnitName;
    use parabellum_types::{map::Position, tribe::Tribe};
    use uuid::Uuid;

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
            loyalty_updated_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent_village_id: None,
        };

        let village = parabellum_game::models::village::Village::from(model);

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
