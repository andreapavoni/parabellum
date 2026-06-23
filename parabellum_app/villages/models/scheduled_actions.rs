//! Scheduled action records and payload dispatch.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use super::workflows::{
    ArmyReturnWorkflow, AttackArrivalWorkflow, BuildingWorkflow, HeroRevivalWorkflow,
    MerchantArrivalWorkflow, MerchantReturnWorkflow, ReinforcementArrivalWorkflow,
    ResearchWorkflow, ScoutArrivalWorkflow, SettlersArrivalWorkflow, TrainingWorkflow,
    TrapBuildWorkflow,
};

/// Processing status of a scheduled action projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledActionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Canceled,
}

impl fmt::Display for ScheduledActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        };
        f.write_str(value)
    }
}

impl FromStr for ScheduledActionStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "canceled" => Ok(Self::Canceled),
            _ => Err("invalid scheduled action status"),
        }
    }
}

/// Canonical scheduled action category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledActionType {
    ReinforcementArrival,
    SettlersArrival,
    AttackArrival,
    ArmyReturn,
    ScoutArrival,
    MerchantsArrival,
    MerchantsReturn,
    AddBuilding,
    UpgradeBuilding,
    DowngradeBuilding,
    TrainUnit,
    ResearchAcademy,
    ResearchSmithy,
    HeroRevival,
    TrapBuild,
}

/// Scheduled action projection row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAction {
    pub id: Uuid,
    pub action_type: ScheduledActionType,
    pub execute_at: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub status: ScheduledActionStatus,
    pub created_at: Option<DateTime<Utc>>,
}

/// Serialized workflow payload for a scheduled action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduledActionPayload {
    ReinforcementArrival {
        workflow: ReinforcementArrivalWorkflow,
    },
    SettlersArrival {
        workflow: SettlersArrivalWorkflow,
    },
    AttackArrival {
        workflow: AttackArrivalWorkflow,
    },
    ArmyReturn {
        workflow: ArmyReturnWorkflow,
    },
    ScoutArrival {
        workflow: ScoutArrivalWorkflow,
    },
    MerchantsArrival {
        workflow: MerchantArrivalWorkflow,
    },
    MerchantsReturn {
        workflow: MerchantReturnWorkflow,
    },
    Building {
        workflow: BuildingWorkflow,
    },
    Training {
        workflow: TrainingWorkflow,
    },
    Research {
        workflow: ResearchWorkflow,
    },
    HeroRevival {
        workflow: HeroRevivalWorkflow,
    },
    TrapBuild {
        workflow: TrapBuildWorkflow,
    },
}

impl ScheduledActionPayload {
    /// Returns the canonical action type for this payload.
    pub fn action_type(&self) -> ScheduledActionType {
        match self {
            ScheduledActionPayload::ReinforcementArrival { .. } => {
                ScheduledActionType::ReinforcementArrival
            }
            ScheduledActionPayload::SettlersArrival { .. } => ScheduledActionType::SettlersArrival,
            ScheduledActionPayload::AttackArrival { .. } => ScheduledActionType::AttackArrival,
            ScheduledActionPayload::ArmyReturn { .. } => ScheduledActionType::ArmyReturn,
            ScheduledActionPayload::ScoutArrival { .. } => ScheduledActionType::ScoutArrival,
            ScheduledActionPayload::MerchantsArrival { .. } => {
                ScheduledActionType::MerchantsArrival
            }
            ScheduledActionPayload::MerchantsReturn { .. } => ScheduledActionType::MerchantsReturn,
            ScheduledActionPayload::Building { workflow } => workflow.kind.action_type(),
            ScheduledActionPayload::Training { .. } => ScheduledActionType::TrainUnit,
            ScheduledActionPayload::Research { workflow } => workflow.kind.action_type(),
            ScheduledActionPayload::HeroRevival { .. } => ScheduledActionType::HeroRevival,
            ScheduledActionPayload::TrapBuild { .. } => ScheduledActionType::TrapBuild,
        }
    }
}
