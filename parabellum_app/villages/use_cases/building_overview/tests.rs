use super::*;
use crate::villages::{
    models::{ScheduledActionStatus, VillageModel},
    read_models::{BuildingQueueItem, VillageQueues, VillageTroopMovements},
    state::VillageState,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_game::models::{
    buildings::Building,
    village::{VillageBuilding, VillageStocks},
};
use parabellum_types::{
    buildings::BuildingName,
    common::ResourceGroup,
    errors::{AppError, DbError},
    map::Position,
    tribe::Tribe,
};
use std::{
    collections::HashSet,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use uuid::Uuid;

fn at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}
fn model() -> VillageModel {
    VillageModel {
        village_id: 7,
        player_id: Uuid::nil(),
        village_name: "Preview".into(),
        position: Position { x: 0, y: 0 },
        tribe: Tribe::Roman,
        buildings: vec![
            VillageBuilding {
                slot_id: 19,
                building: Building::new(BuildingName::MainBuilding, 1)
                    .at_level(10, 1)
                    .unwrap(),
            },
            VillageBuilding {
                slot_id: 20,
                building: Building::new(BuildingName::Cranny, 1),
            },
        ],
        production: Default::default(),
        stocks: VillageStocks::default(),
        population: 0,
        loyalty: 100,
        loyalty_updated_at: at(),
        is_capital: true,
        culture_points_production: 0,
        smithy_upgrades: Default::default(),
        academy_research: Default::default(),
        total_merchants: 0,
        busy_merchants: 0,
        trapper: Default::default(),
        updated_at: at(),
        parent_village_id: None,
    }
}
fn queued(
    slot: u8,
    name: BuildingName,
    kind: BuildingWorkflowKind,
    level: u8,
) -> BuildingQueueItem {
    BuildingQueueItem {
        job_id: Uuid::new_v4(),
        kind,
        slot_id: slot,
        building_name: name,
        target_level: level,
        status: ScheduledActionStatus::Pending,
        finishes_at: at(),
    }
}
struct Reads {
    village: VillageModel,
    queue: VillageQueues,
    calls: Mutex<Vec<&'static str>>,
    fail_queue: bool,
}
#[async_trait]
impl VillageStateReadPort for Reads {
    async fn get_village_state(&self, _: u32) -> Result<VillageModel, ApplicationError> {
        self.calls.lock().unwrap().push("village");
        Ok(self.village.clone())
    }
    async fn list_player_village_states(
        &self,
        _: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        panic!("overview must not load all villages")
    }
}
#[async_trait]
impl VillageActivityReadPort for Reads {
    async fn get_village_queues(&self, _: u32) -> Result<VillageQueues, ApplicationError> {
        self.calls.lock().unwrap().push("queues");
        if self.fail_queue {
            return Err(DbError::VillageNotFound(7).into());
        }
        Ok(self.queue.clone())
    }
    async fn get_village_troop_movements(
        &self,
        _: u32,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        panic!("ordinary previews must not load movements")
    }
    async fn list_cancelable_outgoing_movement_ids(
        &self,
        _: u32,
        _: DateTime<Utc>,
    ) -> Result<HashSet<Uuid>, ApplicationError> {
        panic!("ordinary previews must not load movement controls")
    }
}
#[derive(Default)]
struct FixedClock(AtomicUsize);
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0.fetch_add(1, Ordering::Relaxed);
        at()
    }
}
fn setup(
    village: VillageModel,
    queue: Vec<BuildingQueueItem>,
    fail_queue: bool,
) -> (BuildingOverviewUseCases, Arc<Reads>, Arc<FixedClock>) {
    let reads = Arc::new(Reads {
        village,
        queue: VillageQueues {
            building: queue,
            ..Default::default()
        },
        calls: Mutex::new(vec![]),
        fail_queue,
    });
    let clock = Arc::new(FixedClock::default());
    (
        BuildingOverviewUseCases::new(
            reads.clone(),
            reads.clone(),
            clock.clone(),
            BuildingSettings { server_speed: 1 },
        ),
        reads,
        clock,
    )
}
fn request(slot_id: u8) -> GetBuildingOverviewRequest {
    GetBuildingOverviewRequest {
        player_id: Uuid::nil(),
        village_id: 7,
        slot_id,
    }
}
fn command_state(model: &VillageModel, queue: &[BuildingQueueItem]) -> VillageState {
    let mut state = VillageState::founded_at(
        model.village_id,
        model.village_name.clone(),
        model.position.clone(),
        model.tribe.clone(),
        model.player_id,
        None,
        model.buildings.clone(),
        at(),
    );
    state.village = hydrate_village_at(model.clone(), VillageArmyContext::default(), at());
    for item in queue {
        state.record_building_action_scheduled(
            item.job_id,
            item.kind.clone(),
            item.slot_id,
            item.building_name.clone(),
            item.finishes_at,
        );
    }
    state
}

#[tokio::test]
async fn ordinary_and_empty_slots_load_only_one_village_and_queue_snapshot() {
    for slot_id in [20, 21] {
        let (query, reads, clock) = setup(model(), vec![], false);
        let result = query.get_building_overview(request(slot_id)).await.unwrap();
        assert_eq!(*reads.calls.lock().unwrap(), vec!["village", "queues"]);
        assert_eq!(clock.0.load(Ordering::Relaxed), 1);
        assert_eq!(result.server_time, at());
        assert_eq!(result.village.stocks, reads.village.stocks);
        assert_eq!(
            matches!(result.slot, BuildingSlotOverview::Empty(_)),
            slot_id == 21
        );
    }
}

#[tokio::test]
async fn rejects_invalid_slots_and_foreign_owners_before_reading_queues() {
    let (query, reads, _) = setup(model(), vec![], false);
    assert!(query.get_building_overview(request(0)).await.is_err());
    assert!(reads.calls.lock().unwrap().is_empty());
    let mut foreign = request(20);
    foreign.player_id = Uuid::new_v4();
    assert!(matches!(
        query.get_building_overview(foreign).await,
        Err(ApplicationError::Game(GameError::VillageNotOwned { .. }))
    ));
    assert_eq!(*reads.calls.lock().unwrap(), vec!["village"]);
}

#[tokio::test]
async fn storage_errors_remain_typed() {
    let (query, _, _) = setup(model(), vec![], true);
    assert!(matches!(
        query.get_building_overview(request(20)).await,
        Err(ApplicationError::Db(DbError::VillageNotFound(7)))
    ));
}

#[tokio::test]
async fn tribe_queue_limits_and_pending_downgrades_match_command_validation() {
    for tribe in [Tribe::Roman, Tribe::Gaul] {
        let mut model = model();
        model.tribe = tribe;
        for count in [1, 2, 3] {
            let queue: Vec<_> = (0..count)
                .map(|idx| {
                    queued(
                        25 + idx,
                        BuildingName::Cranny,
                        BuildingWorkflowKind::Upgrade,
                        2,
                    )
                })
                .collect();
            let state = command_state(&model, &queue);
            let (query, _, _) = setup(model.clone(), queue, false);
            let result = query.get_building_overview(request(20)).await.unwrap();
            let command = state.schedule_upgrade_building(20, 1);
            assert_eq!(
                result.queue_full,
                matches!(
                    command,
                    Err(ApplicationError::App(AppError::QueueLimitReached { .. }))
                )
            );
        }
    }
    let queue = vec![queued(
        20,
        BuildingName::Cranny,
        BuildingWorkflowKind::Downgrade,
        0,
    )];
    let state = command_state(&model(), &queue);
    let (query, _, _) = setup(model(), queue, false);
    assert!(
        query
            .get_building_overview(request(20))
            .await
            .unwrap()
            .queue_full
    );
    assert!(matches!(
        state.schedule_upgrade_building(20, 1),
        Err(ApplicationError::App(
            AppError::QueueItemAlreadyQueued { .. }
        ))
    ));
}

#[tokio::test]
async fn upgrade_preview_matches_command_cost_and_accounts_for_committed_levels() {
    let queue = vec![queued(
        20,
        BuildingName::Cranny,
        BuildingWorkflowKind::Upgrade,
        2,
    )];
    let state = command_state(&model(), &queue);
    let (query, _, _) = setup(model(), queue, false);
    let BuildingSlotOverview::Occupied { upgrade, .. } =
        query.get_building_overview(request(20)).await.unwrap().slot
    else {
        panic!()
    };
    let (_, level, seconds, cost) = state.schedule_upgrade_building(20, 1).unwrap();
    assert_eq!(upgrade.next_level, level);
    assert_eq!(upgrade.time_secs as i64, seconds);
    assert_eq!(upgrade.cost, cost);
    let mut max = model();
    max.buildings[1].building = Building::new(BuildingName::Cranny, 1)
        .at_level(10, 1)
        .unwrap();
    let (query, _, _) = setup(max, vec![], false);
    let BuildingSlotOverview::Occupied { upgrade, .. } =
        query.get_building_overview(request(20)).await.unwrap().slot
    else {
        panic!()
    };
    assert!(upgrade.at_max_level);
    assert_eq!(upgrade.next_level, 10);
    assert_eq!(upgrade.time_secs, 0);
}

#[tokio::test]
async fn empty_slot_filters_queued_unique_buildings_and_conflicts_for_commands_too() {
    let queue = vec![queued(
        22,
        BuildingName::Residence,
        BuildingWorkflowKind::Add,
        1,
    )];
    let state = command_state(&model(), &queue);
    let (query, _, _) = setup(model(), queue, false);
    let BuildingSlotOverview::Empty(empty) =
        query.get_building_overview(request(21)).await.unwrap().slot
    else {
        panic!()
    };
    for name in [BuildingName::Residence, BuildingName::Palace] {
        assert!(
            !empty
                .buildable_buildings
                .iter()
                .chain(&empty.locked_buildings)
                .any(|option| option.building_name == name)
        );
        assert!(state.schedule_add_building(21, name, 1).is_err());
    }
}

#[tokio::test]
async fn queued_empty_slot_has_preview_and_poor_village_retains_structural_options() {
    let queue = vec![
        queued(21, BuildingName::Cranny, BuildingWorkflowKind::Add, 1),
        queued(21, BuildingName::Cranny, BuildingWorkflowKind::Upgrade, 2),
    ];
    let state = command_state(&model(), &queue);
    let (query, _, _) = setup(model(), queue, false);
    let BuildingSlotOverview::Empty(empty) =
        query.get_building_overview(request(21)).await.unwrap().slot
    else {
        panic!()
    };
    assert!(empty.has_queue_for_slot);
    assert!(empty.buildable_buildings.is_empty());
    assert_eq!(
        empty.queued_upgrade_preview.unwrap().next_level,
        state.schedule_upgrade_building(21, 1).unwrap().1
    );

    let mut poor = model();
    poor.buildings.retain(|building| building.slot_id != 20);
    poor.stocks.lumber = 0;
    poor.stocks.clay = 0;
    poor.stocks.iron = 0;
    poor.stocks.crop = 0;
    let state = command_state(&poor, &[]);
    let (query, _, _) = setup(poor, vec![], false);
    let BuildingSlotOverview::Empty(empty) =
        query.get_building_overview(request(20)).await.unwrap().slot
    else {
        panic!()
    };
    assert!(
        empty
            .buildable_buildings
            .iter()
            .any(|option| option.building_name == BuildingName::Cranny
                && option.cost != ResourceGroup::default())
    );
    assert!(matches!(
        state.schedule_add_building(20, BuildingName::Cranny, 1),
        Err(ApplicationError::Game(GameError::NotEnoughResources))
    ));
    assert!(!empty.locked_buildings.is_empty());
}
