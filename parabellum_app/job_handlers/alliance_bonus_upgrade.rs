use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_core::ApplicationError;

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
        bonus_type = self.payload.bonus_type as i16,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Executing AllianceBonusUpgrade job");

        let mut alliance = ctx.uow.alliances().get_by_id(self.payload.alliance_id).await?;

        alliance.upgrade_bonus(self.payload.bonus_type)?;

        ctx.uow.alliances().save(&alliance).await?;

        Ok(())
    }
}
