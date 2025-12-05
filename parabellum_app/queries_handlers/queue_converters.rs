use crate::{
    cqrs::queries::{AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem},
    jobs::{
        Job,
        tasks::{
            AddBuildingTask, BuildingUpgradeTask, ResearchAcademyTask, ResearchSmithyTask,
            TrainUnitsTask,
        },
    },
};

pub fn building_queue_item_from_job(job: &Job) -> Option<BuildingQueueItem> {
    let parsed = match job.task.task_type.as_str() {
        "AddBuilding" => serde_json::from_value(job.task.data.clone())
            .map(|payload: AddBuildingTask| (payload.slot_id, payload.name, 1)),
        "BuildingUpgrade" => {
            serde_json::from_value(job.task.data.clone()).map(|payload: BuildingUpgradeTask| {
                (payload.slot_id, payload.building_name, payload.level)
            })
        }
        _ => return None,
    };

    let (slot_id, building_name, target_level) = parsed.ok()?;
    Some(BuildingQueueItem {
        job_id: job.id,
        slot_id,
        building_name,
        target_level,
        status: job.status.clone(),
        finishes_at: job.completed_at,
    })
}

pub fn training_queue_item_from_job(job: &Job) -> Option<TrainingQueueItem> {
    if job.task.task_type != "TrainUnits" {
        return None;
    }

    let payload: TrainUnitsTask = serde_json::from_value(job.task.data.clone()).ok()?;
    Some(TrainingQueueItem {
        job_id: job.id,
        slot_id: payload.slot_id,
        unit: payload.unit,
        quantity: payload.quantity,
        time_per_unit: payload.time_per_unit,
        status: job.status.clone(),
        finishes_at: job.completed_at,
    })
}

pub fn academy_queue_item_from_job(job: &Job) -> Option<AcademyQueueItem> {
    if job.task.task_type != "ResearchAcademy" {
        return None;
    }

    let payload: ResearchAcademyTask = serde_json::from_value(job.task.data.clone()).ok()?;
    Some(AcademyQueueItem {
        job_id: job.id,
        unit: payload.unit,
        status: job.status.clone(),
        finishes_at: job.completed_at,
    })
}

pub fn smithy_queue_item_from_job(job: &Job) -> Option<SmithyQueueItem> {
    if job.task.task_type != "ResearchSmithy" {
        return None;
    }

    let payload: ResearchSmithyTask = serde_json::from_value(job.task.data.clone()).ok()?;
    Some(SmithyQueueItem {
        job_id: job.id,
        unit: payload.unit,
        status: job.status.clone(),
        finishes_at: job.completed_at,
    })
}
