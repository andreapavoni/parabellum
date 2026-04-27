use std::sync::Arc;

use mini_cqrs_es::CqrsError;
use parabellum_types::buildings::BuildingName;

use crate::repository::CqrsEventStoreRepository;

pub use crate::cqrs_es::building_queue::{
    InMemoryBuildingQueueEventStore as InMemoryVillageEventStore, VillageAggregate,
    build_building_queue_cqrs as build_village_cqrs,
    build_building_queue_cqrs_with_projection as build_village_cqrs_with_projection,
    village_stream_id,
};

pub type VillageEvent = crate::cqrs_es::building_queue::BuildingQueueEvent;

pub async fn load_village_aggregate(
    event_store: &Arc<dyn CqrsEventStoreRepository>,
    village_id: u32,
) -> Result<VillageAggregate, CqrsError> {
    crate::cqrs_es::building_queue::load_village_building_queue_aggregate(event_store, village_id)
        .await
}

pub async fn queue_building_construction_event(
    event_store: Arc<dyn CqrsEventStoreRepository>,
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
) -> Result<VillageEvent, CqrsError> {
    crate::cqrs_es::building_queue::queue_add_event_via_cqrs(event_store, village_id, slot_id, name)
        .await
}

pub async fn queue_building_upgrade_event(
    event_store: Arc<dyn CqrsEventStoreRepository>,
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
    current_level: u8,
) -> Result<VillageEvent, CqrsError> {
    crate::cqrs_es::building_queue::queue_upgrade_event_via_cqrs(
        event_store,
        village_id,
        slot_id,
        name,
        current_level,
    )
    .await
}

pub async fn queue_building_downgrade_event(
    event_store: Arc<dyn CqrsEventStoreRepository>,
    village_id: u32,
    slot_id: u8,
    name: BuildingName,
    current_level: u8,
) -> Result<VillageEvent, CqrsError> {
    crate::cqrs_es::building_queue::queue_downgrade_event_via_cqrs(
        event_store,
        village_id,
        slot_id,
        name,
        current_level,
    )
    .await
}
