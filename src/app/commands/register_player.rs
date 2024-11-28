use std::sync::Arc;

use crate::{game::models::Tribe, repository::Repository};

pub struct RegisterPlayerCommand {
    repo: Arc<dyn Repository>,
    username: String,
    tribe: Tribe,
}

impl RegisterPlayerCommand {
    pub fn new(repo: Arc<dyn Repository>, username: String, tribe: Tribe) -> Self {
        Self {
            repo: repo.clone(),
            username,
            tribe,
        }
    }
}
