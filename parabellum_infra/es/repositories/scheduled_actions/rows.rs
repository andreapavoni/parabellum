//! Typed SQL rows and enum conversions for scheduled actions.

use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionStatus, ScheduledActionType,
};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbScheduledActionRow {
    pub(super) id: Uuid,
    pub(super) action_type: DbScheduledActionType,
    pub(super) execute_at: chrono::DateTime<chrono::Utc>,
    pub(super) payload: serde_json::Value,
    pub(super) status: DbScheduledActionStatus,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

impl From<DbScheduledActionRow> for ScheduledAction {
    fn from(value: DbScheduledActionRow) -> Self {
        Self {
            id: value.id,
            action_type: value.action_type.into(),
            execute_at: value.execute_at,
            payload: value.payload,
            status: value.status.into(),
            created_at: Some(value.created_at),
        }
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "scheduled_action_status", rename_all = "lowercase")]
pub(super) enum DbScheduledActionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "scheduled_action_type", rename_all = "PascalCase")]
pub(super) enum DbScheduledActionType {
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

impl From<DbScheduledActionStatus> for ScheduledActionStatus {
    fn from(value: DbScheduledActionStatus) -> Self {
        match value {
            DbScheduledActionStatus::Pending => Self::Pending,
            DbScheduledActionStatus::Processing => Self::Processing,
            DbScheduledActionStatus::Completed => Self::Completed,
            DbScheduledActionStatus::Failed => Self::Failed,
            DbScheduledActionStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<ScheduledActionStatus> for DbScheduledActionStatus {
    fn from(value: ScheduledActionStatus) -> Self {
        match value {
            ScheduledActionStatus::Pending => Self::Pending,
            ScheduledActionStatus::Processing => Self::Processing,
            ScheduledActionStatus::Completed => Self::Completed,
            ScheduledActionStatus::Failed => Self::Failed,
            ScheduledActionStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<DbScheduledActionType> for ScheduledActionType {
    fn from(value: DbScheduledActionType) -> Self {
        match value {
            DbScheduledActionType::ReinforcementArrival => Self::ReinforcementArrival,
            DbScheduledActionType::SettlersArrival => Self::SettlersArrival,
            DbScheduledActionType::AttackArrival => Self::AttackArrival,
            DbScheduledActionType::ArmyReturn => Self::ArmyReturn,
            DbScheduledActionType::ScoutArrival => Self::ScoutArrival,
            DbScheduledActionType::MerchantsArrival => Self::MerchantsArrival,
            DbScheduledActionType::MerchantsReturn => Self::MerchantsReturn,
            DbScheduledActionType::AddBuilding => Self::AddBuilding,
            DbScheduledActionType::UpgradeBuilding => Self::UpgradeBuilding,
            DbScheduledActionType::DowngradeBuilding => Self::DowngradeBuilding,
            DbScheduledActionType::TrainUnit => Self::TrainUnit,
            DbScheduledActionType::ResearchAcademy => Self::ResearchAcademy,
            DbScheduledActionType::ResearchSmithy => Self::ResearchSmithy,
            DbScheduledActionType::HeroRevival => Self::HeroRevival,
            DbScheduledActionType::TrapBuild => Self::TrapBuild,
        }
    }
}

impl From<ScheduledActionType> for DbScheduledActionType {
    fn from(value: ScheduledActionType) -> Self {
        match value {
            ScheduledActionType::ReinforcementArrival => Self::ReinforcementArrival,
            ScheduledActionType::SettlersArrival => Self::SettlersArrival,
            ScheduledActionType::AttackArrival => Self::AttackArrival,
            ScheduledActionType::ArmyReturn => Self::ArmyReturn,
            ScheduledActionType::ScoutArrival => Self::ScoutArrival,
            ScheduledActionType::MerchantsArrival => Self::MerchantsArrival,
            ScheduledActionType::MerchantsReturn => Self::MerchantsReturn,
            ScheduledActionType::AddBuilding => Self::AddBuilding,
            ScheduledActionType::UpgradeBuilding => Self::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding => Self::DowngradeBuilding,
            ScheduledActionType::TrainUnit => Self::TrainUnit,
            ScheduledActionType::ResearchAcademy => Self::ResearchAcademy,
            ScheduledActionType::ResearchSmithy => Self::ResearchSmithy,
            ScheduledActionType::HeroRevival => Self::HeroRevival,
            ScheduledActionType::TrapBuild => Self::TrapBuild,
        }
    }
}
