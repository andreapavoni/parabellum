use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_core::ApplicationError;
use parabellum_game::models::alliance::BonusType;

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::AllianceBonusUpgradeTask,
};

pub struct AllianceBonusUpgradeJobHandler {
    payload: AllianceBonusUpgradeTask,
}

impl AllianceBonusUpgradeJobHandler {
    pub fn new(payload: AllianceBonusUpgradeTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for AllianceBonusUpgradeJobHandler {
    #[instrument(skip_all, fields(
        task_type = "AllianceBonusUpgrade",
        alliance_id = %self.payload.alliance_id,
        bonus_type = self.payload.bonus_type,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing AllianceBonusUpgrade job");

        let mut alliance = ctx.uow.alliances().get_by_id(self.payload.alliance_id).await?;
        let bonus_type = BonusType::from_i16(self.payload.bonus_type)
            .ok_or(parabellum_core::GameError::InvalidBonusType(self.payload.bonus_type))?;

        alliance.upgrade_bonus(bonus_type)?;

        ctx.uow.alliances().save(&alliance).await?;

        Ok(())
    }
}
