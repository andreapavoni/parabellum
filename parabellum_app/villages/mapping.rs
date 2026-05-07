use chrono::Utc;
use parabellum_types::errors::ApplicationError;

use super::models::VillageModel;

impl TryFrom<VillageModel> for parabellum_game::models::village::Village {
    type Error = ApplicationError;

    fn try_from(model: VillageModel) -> Result<Self, Self::Error> {
        Ok(parabellum_game::models::village::Village::from_persistence(
            model.village_id,
            model.village_name,
            model.player_id,
            model.position,
            model.tribe,
            model.buildings,
            vec![],
            model.population,
            model.army,
            model.reinforcements,
            model.deployed_armies,
            model.loyalty,
            model.production,
            model.is_capital,
            model.smithy_upgrades,
            model.stocks,
            model.academy_research,
            model.culture_points,
            model.culture_points_production,
            Utc::now(),
            model.parent_village_id,
        ))
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
            culture_points: 55,
            culture_points_production: 0,
            smithy_upgrades: [1, 2, 0, 0, 0, 0, 0, 0],
            academy_research: academy.clone(),
            total_merchants: 0,
            busy_merchants: 0,
            parent_village_id: None,
            army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
        };

        let village = parabellum_game::models::village::Village::try_from(model).unwrap();

        assert_eq!(village.id, village_id);
        assert_eq!(village.culture_points, 55);
        assert_eq!(village.culture_points_production, 0);
        assert_eq!(village.smithy()[0], 1);
        assert_eq!(village.smithy()[1], 2);
        let idx = village
            .tribe
            .get_unit_idx_by_name(&UnitName::Legionnaire)
            .unwrap();
        assert_eq!(village.academy_research().get(idx), academy.get(idx));
        assert!(village.reinforcements().is_empty());
        assert!(village.deployed_armies().is_empty());
    }
}
