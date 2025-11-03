use async_trait::async_trait;
use serde_json::Value;

use crate::{
    Result,
    app::{
        AppError,
        job_handlers::{
            army_return::ArmyReturnJobHandler, attack::AttackJobHandler,
            research_academy::ResearchAcademyJobHandler, research_smithy::ResearchSmithyJobHandler,
            train_units::TrainUnitsJobHandler,
        },
    },
    error::ApplicationError,
    jobs::{
        handler::{JobHandler, JobRegistry},
        tasks::*,
    },
};

/// This enum lists all possible job types in the application.
/// It's used for compile-time matching.
enum AppTaskType {
    Attack,
    TrainUnits,
    ArmyReturn,
    ResearchAcademy,
    ResearchSmithy,
}

impl AppTaskType {
    /// Parse &str into enum variant.
    fn from_str(task_type: &str) -> Option<Self> {
        match task_type {
            "Attack" => Some(Self::Attack),
            "TrainUnits" => Some(Self::TrainUnits),
            "ArmyReturn" => Some(Self::ArmyReturn),
            "ResearchAcademy" => Some(Self::ResearchAcademy),
            "ResearchSmithy" => Some(Self::ResearchSmithy),
            _ => None,
        }
    }
}

/// This is the concrete implementation of the JobRegistry trait.
/// It holds the logic for mapping task_type strings to concrete handlers.
#[derive(Default)]
pub struct AppJobRegistry;

impl AppJobRegistry {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl JobRegistry for AppJobRegistry {
    fn get_handler(
        &self,
        task_type: &str,
        data: &Value,
    ) -> Result<Box<dyn JobHandler>, ApplicationError> {
        let task = AppTaskType::from_str(task_type)
            .ok_or_else(|| ApplicationError::App(AppError::NoJobHandler(task_type.to_string())))?;

        match task {
            AppTaskType::Attack => {
                let payload: AttackTask = serde_json::from_value(data.clone())?;
                Ok(Box::new(AttackJobHandler::new(payload)))
            }
            AppTaskType::TrainUnits => {
                let payload: TrainUnitsTask = serde_json::from_value(data.clone())?;
                Ok(Box::new(TrainUnitsJobHandler::new(payload)))
            }
            AppTaskType::ArmyReturn => {
                let payload: ArmyReturnTask = serde_json::from_value(data.clone())?;
                Ok(Box::new(ArmyReturnJobHandler::new(payload)))
            }
            AppTaskType::ResearchAcademy => {
                let payload: ResearchAcademyTask = serde_json::from_value(data.clone())?;
                Ok(Box::new(ResearchAcademyJobHandler::new(payload)))
            }
            AppTaskType::ResearchSmithy => {
                let payload: ResearchSmithyTask = serde_json::from_value(data.clone())?;
                Ok(Box::new(ResearchSmithyJobHandler::new(payload)))
            }
        }
    }
}
