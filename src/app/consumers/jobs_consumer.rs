use anyhow::Result;

use super::EventConsumer;
use crate::app::events::GameEvent;

#[derive(Debug, Clone)]
pub struct JobConsumer;

impl EventConsumer for JobConsumer {
    fn process(_: GameEvent) -> Result<()> {
        // TODO: extract job from event -> store on db
        Ok(())
    }
}
