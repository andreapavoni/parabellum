/// Request compact references for a set of villages.
#[derive(Debug, Clone)]
pub struct GetVillageReferencesRequest {
    /// Village ids to resolve.
    pub village_ids: Vec<u32>,
}
