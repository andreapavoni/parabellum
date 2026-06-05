use parabellum_types::{common::ResourceQuantity, errors::GameError};
use uuid::Uuid;

use crate::villages::models::MarketplaceOfferSnapshot;

#[derive(Debug, Clone)]
pub struct MarketplaceAcceptance<'a> {
    pub accepting_player_id: Uuid,
    pub accepting_village_id: u32,
    pub offer: &'a MarketplaceOfferSnapshot,
}

impl MarketplaceAcceptance<'_> {
    pub fn validate(&self) -> Result<(), GameError> {
        if self.accepting_village_id == self.offer.owner_village_id
            || self.accepting_player_id == self.offer.owner_player_id
        {
            return Err(GameError::InvalidMarketplaceOffer);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketplaceOfferCreation {
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
}

impl MarketplaceOfferCreation {
    pub fn validate(&self) -> Result<(), GameError> {
        if self.offer_resources.quantity == 0
            || self.seek_resources.quantity == 0
            || self.offer_resources.resource == self.seek_resources.resource
        {
            return Err(GameError::InvalidMarketplaceOffer);
        }

        let (max_side, min_side) = if self.offer_resources.quantity >= self.seek_resources.quantity
        {
            (self.offer_resources.quantity, self.seek_resources.quantity)
        } else {
            (self.seek_resources.quantity, self.offer_resources.quantity)
        };
        if max_side > min_side.saturating_mul(3) {
            return Err(GameError::InvalidMarketplaceOffer);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_types::common::{ResourceKind, ResourceQuantity};

    use super::*;

    fn offer(owner_player_id: Uuid, owner_village_id: u32) -> MarketplaceOfferSnapshot {
        MarketplaceOfferSnapshot {
            offer_id: Uuid::new_v4(),
            owner_player_id,
            owner_village_id,
            offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
            seek_resources: ResourceQuantity::new(ResourceKind::Clay, 100),
            merchants_reserved: 1,
        }
    }

    #[test]
    fn allows_valid_offer_creation_terms() {
        assert!(
            MarketplaceOfferCreation {
                offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 300),
                seek_resources: ResourceQuantity::new(ResourceKind::Clay, 100),
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn rejects_zero_same_resource_or_ratio_above_three_to_one() {
        assert_eq!(
            MarketplaceOfferCreation {
                offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 0),
                seek_resources: ResourceQuantity::new(ResourceKind::Clay, 100),
            }
            .validate(),
            Err(GameError::InvalidMarketplaceOffer)
        );
        assert_eq!(
            MarketplaceOfferCreation {
                offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
                seek_resources: ResourceQuantity::new(ResourceKind::Lumber, 100),
            }
            .validate(),
            Err(GameError::InvalidMarketplaceOffer)
        );
        assert_eq!(
            MarketplaceOfferCreation {
                offer_resources: ResourceQuantity::new(ResourceKind::Lumber, 301),
                seek_resources: ResourceQuantity::new(ResourceKind::Clay, 100),
            }
            .validate(),
            Err(GameError::InvalidMarketplaceOffer)
        );
    }

    #[test]
    fn allows_cross_player_cross_village_acceptance() {
        let owner_player_id = Uuid::new_v4();
        let offer = offer(owner_player_id, 1);

        assert!(
            MarketplaceAcceptance {
                accepting_player_id: Uuid::new_v4(),
                accepting_village_id: 2,
                offer: &offer,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn rejects_owner_or_owner_village_acceptance() {
        let owner_player_id = Uuid::new_v4();
        let offer = offer(owner_player_id, 1);

        assert_eq!(
            MarketplaceAcceptance {
                accepting_player_id: owner_player_id,
                accepting_village_id: 2,
                offer: &offer,
            }
            .validate(),
            Err(GameError::InvalidMarketplaceOffer)
        );
        assert_eq!(
            MarketplaceAcceptance {
                accepting_player_id: Uuid::new_v4(),
                accepting_village_id: 1,
                offer: &offer,
            }
            .validate(),
            Err(GameError::InvalidMarketplaceOffer)
        );
    }

    #[test]
    fn acceptance_does_not_revalidate_offer_terms() {
        let owner_player_id = Uuid::new_v4();
        let mut offer = offer(owner_player_id, 1);
        offer.seek_resources = ResourceQuantity::new(ResourceKind::Lumber, 100);

        assert!(
            MarketplaceAcceptance {
                accepting_player_id: Uuid::new_v4(),
                accepting_village_id: 2,
                offer: &offer,
            }
            .validate()
            .is_ok()
        );
    }
}
