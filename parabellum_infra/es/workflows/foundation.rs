//! Settlers arrival and village foundation workflow orchestration.
//!
//! Foundation uses `rm_map_fields` as the canonical map read model. A missing
//! row is an infrastructure/data error; an existing but occupied or non-valley
//! field is treated as an unavailable target and schedules the settlers return.

use mini_cqrs_es::CqrsError;
use parabellum_app::ports::map::MapRepository;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::SettlersArrivalWorkflow;
use parabellum_app::villages::repositories::{ArmyRepository, ScheduledActionRepository};
use parabellum_game::models::buildings::Building;
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::buildings::BuildingName;
use parabellum_types::map::ValleyTopology;

use crate::es::{
    PostgresArmyRepository, PostgresScheduledActionRepository, VillageEsService, workflows,
};
use crate::map::PostgresMapRepository;

enum FoundationTarget {
    Available(ValleyTopology),
    Unavailable,
}

pub(crate) async fn settlers_arrival_events(
    svc: &VillageEsService,
    workflow: SettlersArrivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    if let FoundationTarget::Available(topology) = foundation_target(svc, &workflow).await? {
        let cfg = parabellum_app::config::Config::from_env();
        let default_buildings = default_founded_village_buildings(&topology, cfg.speed)?;
        return Ok(settlers_foundation_events(workflow, default_buildings));
    }

    schedule_settlers_return(svc, workflow).await?;
    Ok(super::WorkflowEvents::new())
}

async fn foundation_target(
    svc: &VillageEsService,
    workflow: &SettlersArrivalWorkflow,
) -> Result<FoundationTarget, CqrsError> {
    let topology = PostgresMapRepository::new(svc.pool().clone())
        .get_foundation_target_topology(workflow.target_village_id, workflow.player_id)
        .await
        .map_err(CqrsError::domain_source)?;

    Ok(match topology {
        Some(topology) => FoundationTarget::Available(topology),
        None => FoundationTarget::Unavailable,
    })
}

async fn schedule_settlers_return(
    svc: &VillageEsService,
    workflow: SettlersArrivalWorkflow,
) -> Result<(), CqrsError> {
    tracing::warn!(
        action_id = %workflow.action_id,
        player_id = %workflow.player_id,
        source_village_id = workflow.source_village_id,
        target_village_id = workflow.target_village_id,
        "settlers arrival target unavailable, scheduling army return"
    );

    let army_repo = PostgresArmyRepository::new(svc.pool().clone());
    let army = army_repo
        .get_moving_army(workflow.army_id)
        .await
        .map_err(CqrsError::domain_source)?;
    let source = svc.get_village(workflow.source_village_id).await?;
    let cfg = parabellum_app::config::Config::from_env();
    let travel_secs = source.position.calculate_travel_time_secs(
        workflow.target_position.clone(),
        army.speed(),
        cfg.world_size as i32,
        cfg.speed as u8,
    ) as i64;
    let returns_at = workflow.arrives_at + chrono::Duration::seconds(std::cmp::max(1, travel_secs));
    let return_action_id = uuid::Uuid::new_v4();
    let return_workflow = workflows::movements::army_return_workflow(
        workflow.movement_id,
        workflow.army_id,
        workflow.source_village_id,
        workflow.source_village_id,
        workflow.target_village_id,
        workflow.player_id,
        army,
        None,
        returns_at,
    );
    let return_action = workflows::movements::army_return_scheduled_action_from_workflow(
        return_action_id,
        return_workflow,
    )?;
    PostgresScheduledActionRepository::new(svc.pool().clone())
        .add(&return_action)
        .await
        .map_err(CqrsError::domain_source)?;

    Ok(())
}

fn settlers_foundation_events(
    workflow: SettlersArrivalWorkflow,
    default_buildings: Vec<VillageBuilding>,
) -> super::WorkflowEvents {
    super::WorkflowEvents::from_events(vec![
        (
            workflow.source_village_id,
            VillageEvent::SettlersArrived {
                action_id: workflow.action_id,
                movement_id: workflow.movement_id,
                army_id: workflow.army_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                target_position: workflow.target_position.clone(),
                village_name: workflow.village_name.clone(),
                tribe: workflow.tribe.clone(),
                arrives_at: workflow.arrives_at,
            },
        ),
        (
            workflow.target_village_id,
            VillageEvent::VillageFounded {
                village_id: workflow.target_village_id,
                village_name: workflow.village_name,
                position: workflow.target_position,
                tribe: workflow.tribe,
                player_id: workflow.player_id,
                parent_village_id: Some(workflow.source_village_id),
                buildings: default_buildings,
            },
        ),
    ])
}

fn default_founded_village_buildings(
    topology: &ValleyTopology,
    speed: i8,
) -> Result<Vec<VillageBuilding>, CqrsError> {
    let mut slot_id: u8 = 1;
    let mut buildings = Vec::with_capacity(19);

    let mut push_n = |name: BuildingName, count: u8| -> Result<(), CqrsError> {
        for _ in 0..count {
            let building = Building::new(name.clone(), speed)
                .at_level(0, speed)
                .map_err(CqrsError::domain_source)?;
            buildings.push(VillageBuilding { slot_id, building });
            slot_id += 1;
        }
        Ok(())
    };

    push_n(BuildingName::Woodcutter, topology.lumber())?;
    push_n(BuildingName::ClayPit, topology.clay())?;
    push_n(BuildingName::IronMine, topology.iron())?;
    push_n(BuildingName::Cropland, topology.crop())?;

    let main_building = Building::new(BuildingName::MainBuilding, speed)
        .at_level(1, speed)
        .map_err(CqrsError::domain_source)?;
    buildings.push(VillageBuilding {
        slot_id: 19,
        building: main_building,
    });

    Ok(buildings)
}
