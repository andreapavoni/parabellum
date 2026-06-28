//! Typed SQL rows for active merchant movement reads.

use chrono::{DateTime, Utc};
use parabellum_app::villages::read_models::{
    MerchantMovement, MerchantMovementDirection, MerchantMovementKind,
};
use parabellum_types::common::ResourceGroup;
use parabellum_types::errors::ApplicationError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(super) struct DbMerchantMovementRow {
    id: Uuid,
    direction: String,
    kind: String,
    origin_village_id: i32,
    destination_village_id: i32,
    resources: serde_json::Value,
    merchants_used: i16,
    arrives_at: DateTime<Utc>,
}

impl TryFrom<DbMerchantMovementRow> for MerchantMovement {
    type Error = ApplicationError;

    fn try_from(row: DbMerchantMovementRow) -> Result<Self, Self::Error> {
        let direction = match row.direction.as_str() {
            "outgoing" => MerchantMovementDirection::Outgoing,
            "incoming" => MerchantMovementDirection::Incoming,
            value => {
                return Err(ApplicationError::Unknown(format!(
                    "unknown merchant movement direction {value}"
                )));
            }
        };
        let kind = match row.kind.as_str() {
            "going" => MerchantMovementKind::Going,
            "return" => MerchantMovementKind::Return,
            value => {
                return Err(ApplicationError::Unknown(format!(
                    "unknown merchant movement kind {value}"
                )));
            }
        };
        let resources: ResourceGroup = serde_json::from_value(row.resources)
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        Ok(MerchantMovement {
            job_id: row.id,
            direction,
            kind,
            origin_village_id: row.origin_village_id as u32,
            destination_village_id: row.destination_village_id as u32,
            resources,
            merchants_used: row.merchants_used as u8,
            arrives_at: row.arrives_at,
        })
    }
}
