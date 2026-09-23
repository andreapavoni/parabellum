//! Derived village read-model refresh.
//!
//! This module rehydrates a domain `Village` to reuse domain-owned production,
//! stock, upkeep, merchant, culture-point, and loyalty behavior. It only applies
//! projection-time adjustments for read-model concerns that are not currently
//! part of `Village` hydration, such as active hero resource bonuses and upkeep
//! for moving armies.

use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::{VillageArmyContext, hydrate_village};
use parabellum_types::common::ResourceGroup;

/// Recomputes derived read fields before returning or storing a village model.
pub(super) fn refresh_materialized_village_state(
    model: VillageModel,
    army_context: VillageArmyContext,
    hero_resources: ResourceGroup,
) -> VillageModel {
    let moving_armies = army_context.moving.clone();
    let mut hydrated = hydrate_village(model.clone(), army_context);
    let busy_merchants = model.busy_merchants;
    let previous_updated_at = model.updated_at;
    let mut refreshed = model;
    refreshed.production = hydrated.production.clone();
    refreshed.production.upkeep =
        refreshed
            .production
            .upkeep
            .saturating_add(moving_armies_upkeep_for_read_projection(
                &hydrated,
                &moving_armies,
            ));
    refreshed.production.calculate_effective_production();
    refreshed.stocks = hydrated.stocks().clone();
    apply_hero_resource_read_projection(&mut refreshed, previous_updated_at, hero_resources);
    refreshed.population = hydrated.population;
    refreshed.culture_points_production = hydrated.culture_points_production;
    refreshed.total_merchants = hydrated.total_merchants;

    let loyalty_elapsed = chrono::Utc::now() - refreshed.loyalty_updated_at;
    hydrated.regenerate_loyalty(
        loyalty_elapsed,
        parabellum_app::config::Config::from_env().speed as f64,
    );
    refreshed.loyalty = hydrated.loyalty();

    // Busy merchants are operational state managed by movement/marketplace flows,
    // and `Village::from_persistence` resets it to zero internally.
    // Preserve the persisted value from the read model.
    refreshed.busy_merchants = busy_merchants.min(refreshed.total_merchants);
    refreshed.updated_at = hydrated.updated_at;
    refreshed
}

fn apply_hero_resource_read_projection(
    refreshed: &mut VillageModel,
    previous_updated_at: chrono::DateTime<chrono::Utc>,
    hero_resources: ResourceGroup,
) {
    if hero_resources == ResourceGroup::default() {
        return;
    }

    refreshed
        .production
        .add_flat_effective_production(&hero_resources);
    refreshed
        .stocks
        .store_hourly_production(&hero_resources, chrono::Utc::now() - previous_updated_at);
}

fn moving_armies_upkeep_for_read_projection(
    village: &parabellum_game::models::village::Village,
    armies: &[parabellum_game::models::army::Army],
) -> u32 {
    armies.iter().map(|army| village.army_upkeep(army)).sum()
}
