use std::{collections::HashSet, sync::Arc};

use parabellum_types::Result;

use crate::{
    config::Config,
    cqrs::{
        QueryHandler,
        queries::{GetMarketplaceData, MarketplaceData},
    },
    uow::UnitOfWork,
};

pub struct GetMarketplaceDataHandler;

impl GetMarketplaceDataHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetMarketplaceDataHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetMarketplaceData> for GetMarketplaceDataHandler {
    async fn handle(
        &self,
        query: GetMarketplaceData,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<MarketplaceData> {
        // Load current village to get its position for distance calculations
        let village = uow.villages().get_by_id(query.village_id).await?;
        let village_position = village.position;
        let player_id = village.player_id;

        // Fetch own offers
        let own_offers = uow.marketplace().list_by_village(query.village_id).await?;

        // Fetch all global offers
        let all_offers = uow.marketplace().list_all().await?;

        // Filter out own offers and calculate distances
        let mut global_offers_with_distance: Vec<_> = all_offers
            .into_iter()
            .filter(|offer| offer.village_id != query.village_id && offer.player_id != player_id)
            .map(|offer| {
                // We'll need to fetch the offer village's position to calculate distance
                // Store offer with a placeholder distance for now
                (offer, 0)
            })
            .collect();

        // Collect all village IDs we need info for
        let mut village_ids: HashSet<u32> = HashSet::new();
        for offer in &own_offers {
            village_ids.insert(offer.village_id);
        }
        for (offer, _) in &global_offers_with_distance {
            village_ids.insert(offer.village_id);
        }

        // Fetch village info for all referenced villages
        let village_ids_vec: Vec<u32> = village_ids.into_iter().collect();
        let village_info = uow.villages().get_info_by_ids(&village_ids_vec).await?;

        // Now calculate actual distances and sort
        for (offer, distance) in &mut global_offers_with_distance {
            if let Some(offer_village_info) = village_info.get(&offer.village_id) {
                *distance = village_position
                    .distance(&offer_village_info.position, config.world_size as i32);
            }
        }

        // Sort by distance (closest first)
        global_offers_with_distance.sort_by(|(_, dist_a), (_, dist_b)| {
            dist_a
                .partial_cmp(dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Extract just the offers (drop distances)
        let global_offers: Vec<_> = global_offers_with_distance
            .into_iter()
            .map(|(offer, _)| offer)
            .collect();

        Ok(MarketplaceData {
            own_offers,
            global_offers,
            village_info,
        })
    }
}
