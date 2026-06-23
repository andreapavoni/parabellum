use std::sync::Arc;

use parabellum_game::models::culture_points::required_cp;
use parabellum_types::errors::ApplicationError;

use crate::villages::{
    ports::ExpansionReadPort, requests::expansion::GetExpansionCultureInfoRequest,
};

/// Culture-point information needed by expansion building views.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpansionCultureInfo {
    /// Culture points produced per day by the selected village.
    pub village_culture_points_production: u32,
    /// Current player culture points after refresh.
    pub player_culture_points: u32,
    /// Culture points produced per day by all player villages.
    pub player_culture_points_production: u32,
    /// Culture points required for the next village slot.
    pub next_cp_required: u32,
}

/// Application service for village expansion reads.
#[derive(Clone)]
pub struct VillageExpansionUseCases {
    reads: Arc<dyn ExpansionReadPort>,
}

impl VillageExpansionUseCases {
    /// Creates expansion use cases from the expansion read port.
    pub fn new(reads: Arc<dyn ExpansionReadPort>) -> Self {
        Self { reads }
    }

    /// Loads expansion culture information and refreshes player culture points first.
    pub async fn get_expansion_culture_info(
        &self,
        request: GetExpansionCultureInfoRequest,
    ) -> Result<ExpansionCultureInfo, ApplicationError> {
        let culture = self
            .reads
            .get_expansion_culture_snapshot(request.player_id, request.village_id)
            .await?;

        self.reads
            .refresh_player_culture_points(request.player_id)
            .await?;
        let player = self.reads.get_player(request.player_id).await?;

        let next_cp_required = required_cp(
            request.server_speed.into(),
            culture.player_village_count + 1,
        );

        Ok(ExpansionCultureInfo {
            village_culture_points_production: culture.village_culture_points_production,
            player_culture_points: player.culture_points,
            player_culture_points_production: culture.player_culture_points_production,
            next_cp_required,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use async_trait::async_trait;
    use parabellum_game::models::culture_points::required_cp;
    use parabellum_types::{
        common::{Player, Speed},
        errors::ApplicationError,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{
        ports::ExpansionReadPort, projection_repositories::ExpansionCultureSnapshot,
        requests::expansion::GetExpansionCultureInfoRequest,
        use_cases::expansion::VillageExpansionUseCases,
    };

    struct FakeExpansionReads {
        refreshed: AtomicBool,
        calls: Mutex<Vec<&'static str>>,
    }

    impl Default for FakeExpansionReads {
        fn default() -> Self {
            Self {
                refreshed: AtomicBool::new(false),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ExpansionReadPort for FakeExpansionReads {
        async fn get_expansion_culture_snapshot(
            &self,
            _player_id: Uuid,
            _village_id: u32,
        ) -> Result<ExpansionCultureSnapshot, ApplicationError> {
            self.calls.lock().unwrap().push("snapshot");
            Ok(ExpansionCultureSnapshot {
                village_culture_points_production: 6,
                player_culture_points_production: 11,
                player_village_count: 1,
            })
        }

        async fn refresh_player_culture_points(
            &self,
            _player_id: Uuid,
        ) -> Result<(), ApplicationError> {
            self.calls.lock().unwrap().push("refresh");
            self.refreshed.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn get_player(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
            self.calls.lock().unwrap().push("player");
            assert!(self.refreshed.load(Ordering::SeqCst));
            Ok(Player {
                id: player_id,
                username: "player".to_string(),
                tribe: Tribe::Roman,
                user_id: Uuid::nil(),
                culture_points: 42,
            })
        }
    }

    #[tokio::test]
    async fn expansion_culture_info_refreshes_player_points_and_calculates_next_requirement() {
        let reads = Arc::new(FakeExpansionReads::default());
        let use_cases = VillageExpansionUseCases::new(reads.clone());
        let player_id = Uuid::new_v4();

        let info = use_cases
            .get_expansion_culture_info(GetExpansionCultureInfoRequest {
                player_id,
                village_id: 10,
                server_speed: 3,
            })
            .await
            .unwrap();

        assert_eq!(
            reads.calls.lock().unwrap().as_slice(),
            &["snapshot", "refresh", "player"]
        );
        assert_eq!(info.village_culture_points_production, 6);
        assert_eq!(info.player_culture_points_production, 11);
        assert_eq!(info.player_culture_points, 42);
        assert_eq!(info.next_cp_required, required_cp(Speed::X3, 2));
    }
}
