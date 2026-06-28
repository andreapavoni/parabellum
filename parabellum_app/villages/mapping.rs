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

/// Copies domain-owned village state back into a projected village model.
///
/// Projection-only fields such as trapper state and loyalty timestamps are
/// preserved on the provided model. Call this after mutating a hydrated
/// `Village` so repository writes can store the complete row without
/// duplicating domain calculations in infrastructure code.
pub fn apply_domain_village_state(model: &mut VillageModel, village: &Village) {
    model.village_id = village.id;
    model.player_id = village.player_id;
    model.village_name = village.name.clone();
    model.position = village.position.clone();
    model.tribe = village.tribe.clone();
    model.buildings = village.buildings().clone();
    model.production = village.production.clone();
    model.stocks = village.stocks().clone();
    model.population = village.population;
    model.loyalty = village.loyalty();
    model.is_capital = village.is_capital;
    model.culture_points_production = village.culture_points_production;
    model.smithy_upgrades = *village.smithy();
    model.academy_research = village.academy_research().clone();
    model.total_merchants = village.total_merchants;
    model.busy_merchants = village.busy_merchants;
    model.updated_at = village.updated_at;
    model.parent_village_id = village.parent_village_id;
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

    #[test]
    fn domain_village_state_updates_projected_model_without_resetting_projection_only_fields() {
        let position = Position { x: 0, y: 0 };
        let village_id = position.to_id(100);
        let loyalty_updated_at = chrono::Utc::now() - chrono::Duration::hours(2);
        let mut model = VillageModel {
            village_id,
            player_id: Uuid::new_v4(),
            village_name: "old".to_string(),
            position: position.clone(),
            tribe: Tribe::Roman,
            buildings: vec![],
            production: VillageProduction::default(),
            stocks: VillageStocks::default(),
            population: 12,
            loyalty: 80,
            is_capital: true,
            culture_points_production: 0,
            smithy_upgrades: [0; 8],
            academy_research: AcademyResearch::default(),
            total_merchants: 0,
            busy_merchants: 0,
            trapper: Default::default(),
            loyalty_updated_at,
            updated_at: chrono::Utc::now(),
            parent_village_id: None,
        };
        let mut village =
            crate::villages::hydrate_village(model.clone(), VillageArmyContext::default());
        village.name = "new".to_string();
        village.busy_merchants = 3;

        super::apply_domain_village_state(&mut model, &village);

        assert_eq!(model.village_name, "new");
        assert_eq!(model.busy_merchants, 3);
        assert_eq!(model.loyalty_updated_at, loyalty_updated_at);
    }
}
