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

    refreshed.production.effective.lumber = refreshed
        .production
        .effective
        .lumber
        .saturating_add(hero_resources.lumber());
    refreshed.production.effective.clay = refreshed
        .production
        .effective
        .clay
        .saturating_add(hero_resources.clay());
    refreshed.production.effective.iron = refreshed
        .production
        .effective
        .iron
        .saturating_add(hero_resources.iron());
    refreshed.production.effective.crop = refreshed
        .production
        .effective
        .crop
        .saturating_add(hero_resources.crop() as i64);

    let elapsed = (chrono::Utc::now() - previous_updated_at).num_seconds() as f64;
    if elapsed <= 0.0 {
        return;
    }

    let add = |current: u32, per_hour: u32, capacity: u32| -> u32 {
        (current as f64 + elapsed * (per_hour as f64 / 3600.0))
            .min(capacity as f64)
            .max(0.0)
            .floor() as u32
    };
    refreshed.stocks.lumber = add(
        refreshed.stocks.lumber,
        hero_resources.lumber(),
        refreshed.stocks.warehouse_capacity,
    );
    refreshed.stocks.clay = add(
        refreshed.stocks.clay,
        hero_resources.clay(),
        refreshed.stocks.warehouse_capacity,
    );
    refreshed.stocks.iron = add(
        refreshed.stocks.iron,
        hero_resources.iron(),
        refreshed.stocks.warehouse_capacity,
    );
    refreshed.stocks.crop = add(
        refreshed.stocks.crop.max(0) as u32,
        hero_resources.crop(),
        refreshed.stocks.granary_capacity,
    ) as i64;
}

fn moving_armies_upkeep_for_read_projection(
    village: &parabellum_game::models::village::Village,
    armies: &[parabellum_game::models::army::Army],
) -> u32 {
    armies.iter().map(|army| village.army_upkeep(army)).sum()
}
