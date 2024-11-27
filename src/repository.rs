use anyhow::Result;

use crate::game::models::{
    map::{Oasis, Valley},
    village::Village,
};

#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    async fn bootstrap_new_map(&self, size: u32) -> Result<()>;
    async fn get_village_by_id(&self, village_id: u32) -> Result<Village>;
    async fn get_valley_by_id(&self, valley_id: u32) -> Result<Valley>;
    async fn get_oasis_by_id(&self, oasis_id: u32) -> Result<Oasis>;
}
