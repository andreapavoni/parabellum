#[cfg(test)]
pub mod tests {
    use axum::http::{HeaderValue, StatusCode};
    use parabellum_web::session::current_user_by_ids;
    use parabellum_web::{AppState, WebRouter};
    use reqwest::{Client, header, redirect::Policy};
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::{env, net::TcpListener, sync::Arc, time::Duration};
    use uuid::Uuid;

    use parabellum_app::auth::hash_password;
    use parabellum_app::{
        application::GameApplication,
        config::Config,
        identity::{
            InitialVillageCommandExecutor, RegistrationIdentityPort, RegistrationSettings,
            RegistrationUseCases,
        },
        leaderboards::LeaderboardUseCases,
        map::MapUseCases,
        scheduler::{SchedulerPort, SchedulerUseCases},
        villages::{
            BuildingSettings, BuildingUseCases, DevelopmentSettings, DevelopmentUseCases,
            HeroSettings, HeroUseCases, MarketplaceSettings, MarketplaceUseCases,
            MovementControlUseCases, MovementSettings, MovementUseCases, ReinforcementSettings,
            ReinforcementUseCases, ReportUseCases, SystemClock, TrapUseCases, UuidGenerator,
            VillageActivityUseCases, VillageArmyUseCases, VillageExpansionUseCases,
            VillageProfileUseCases, VillageReferenceUseCases, VillageStateUseCases,
            ports::{
                BuildingCommandExecutor, BuildingReadPort, DevelopmentCommandExecutor,
                DevelopmentReadPort, ExpansionReadPort, HeroCommandExecutor, HeroReadPort,
                MarketplaceCommandExecutor, MarketplaceReadPort, MovementControlCommandExecutor,
                MovementControlReadPort, MovementReadPort, ReinforcementCommandExecutor,
                ReinforcementReadPort, ReportCommandExecutor, ReportReadPort, TrapCommandExecutor,
                TrapReadPort, VillageActivityReadPort, VillageArmyReadPort, VillageCommandExecutor,
                VillageProfileCommandExecutor, VillageReferenceReadPort, VillageStateReadPort,
            },
        },
    };
    use parabellum_game::models::{
        buildings::Building,
        map::Valley,
        village::{Village, VillageBuilding},
    };
    use parabellum_infra::{
        adapters::VillageEsAdapter,
        bootstrap_world_map,
        es::VillageEsService,
        identity::{IdentityService, repositories::PostgresPlayerRepository},
        map::PostgresMapRepository,
    };
    use parabellum_types::buildings::BuildingName;
    use parabellum_types::common::Player;
    use parabellum_types::map::Position;
    use parabellum_types::map::ValleyTopology;
    use parabellum_types::tribe::Tribe;
    use parabellum_types::{Result, errors::ApplicationError};

    #[derive(Clone)]
    pub struct IsolatedSchemaHandle {
        root_url: String,
        schema_name: String,
    }

    #[derive(Clone)]
    pub struct SeededAuthUser {
        pub username: String,
        pub email: String,
        pub password: String,
    }

    impl Drop for IsolatedSchemaHandle {
        fn drop(&mut self) {
            let root_url = self.root_url.clone();
            let schema_name = self.schema_name.clone();
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Runtime::new() {
                    rt.block_on(async move {
                        if let Ok(pool) = PgPoolOptions::new()
                            .max_connections(1)
                            .connect(&root_url)
                            .await
                        {
                            let query =
                                format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", schema_name);
                            let _ = sqlx::query(&query).execute(&pool).await;
                        }
                    });
                }
            });
        }
    }

    fn append_search_path(url: &str, schema_name: &str) -> String {
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}options=-csearch_path%3D{schema_name}%2Cpublic")
    }

    async fn create_isolated_test_pool() -> Result<(PgPool, Arc<IsolatedSchemaHandle>)> {
        dotenvy::dotenv().ok();
        let root_url = env::var("TEST_DATABASE_URL")
            .map_err(|_| ApplicationError::Unknown("TEST_DATABASE_URL must be set".to_string()))?;
        let schema_name = format!("test_{}", Uuid::new_v4().simple());

        let root_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&root_url)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#)
            .execute(&root_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let create_query = format!("CREATE SCHEMA \"{}\"", schema_name);
        sqlx::query(&create_query)
            .execute(&root_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let isolated_url = append_search_path(&root_url, &schema_name);
        let isolated_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&isolated_url)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::migrate!("../migrations")
            .run(&isolated_pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let handle = Arc::new(IsolatedSchemaHandle {
            root_url,
            schema_name,
        });

        Ok((isolated_pool, handle))
    }

    fn build_game_app(pool: &PgPool, config: Arc<Config>) -> Arc<GameApplication> {
        let leaderboards =
            LeaderboardUseCases::new(Arc::new(PostgresPlayerRepository::new(pool.clone())));
        let map = MapUseCases::new(Arc::new(PostgresMapRepository::new(
            parabellum_infra::ProjectionDb::new(pool.clone()),
        )));
        let identity = Arc::new(IdentityService::new(pool.clone()));
        let villages = Arc::new(VillageEsAdapter::new(VillageEsService::new(pool.clone())));
        let registration_identities: Arc<dyn RegistrationIdentityPort> = identity.clone();
        let initial_village_executor: Arc<dyn InitialVillageCommandExecutor> = villages.clone();
        let registration = RegistrationUseCases::new(
            registration_identities,
            initial_village_executor,
            Arc::new(UuidGenerator),
            RegistrationSettings {
                world_size: config.world_size as i32,
                server_speed: config.speed,
            },
        );
        let building_reads: Arc<dyn BuildingReadPort> = villages.clone();
        let building_executor: Arc<dyn BuildingCommandExecutor> = villages.clone();
        let development_reads: Arc<dyn DevelopmentReadPort> = villages.clone();
        let development_executor: Arc<dyn DevelopmentCommandExecutor> = villages.clone();
        let hero_reads: Arc<dyn HeroReadPort> = villages.clone();
        let hero_executor: Arc<dyn HeroCommandExecutor> = villages.clone();
        let village_profile_executor: Arc<dyn VillageProfileCommandExecutor> = villages.clone();
        let movement_reads: Arc<dyn MovementReadPort> = villages.clone();
        let movement_executor: Arc<dyn VillageCommandExecutor> = villages.clone();
        let movement_control_reads: Arc<dyn MovementControlReadPort> = villages.clone();
        let movement_control_executor: Arc<dyn MovementControlCommandExecutor> = villages.clone();
        let marketplace_reads: Arc<dyn MarketplaceReadPort> = villages.clone();
        let marketplace_executor: Arc<dyn MarketplaceCommandExecutor> = villages.clone();
        let reinforcement_reads: Arc<dyn ReinforcementReadPort> = villages.clone();
        let reinforcement_executor: Arc<dyn ReinforcementCommandExecutor> = villages.clone();
        let report_reads: Arc<dyn ReportReadPort> = villages.clone();
        let report_executor: Arc<dyn ReportCommandExecutor> = villages.clone();
        let activity_reads: Arc<dyn VillageActivityReadPort> = villages.clone();
        let army_reads: Arc<dyn VillageArmyReadPort> = villages.clone();
        let expansion_reads: Arc<dyn ExpansionReadPort> = villages.clone();
        let village_reference_reads: Arc<dyn VillageReferenceReadPort> = villages.clone();
        let village_state_reads: Arc<dyn VillageStateReadPort> = villages.clone();
        let trap_reads: Arc<dyn TrapReadPort> = villages.clone();
        let trap_executor: Arc<dyn TrapCommandExecutor> = villages.clone();
        let scheduler_port: Arc<dyn SchedulerPort> = villages.clone();
        let scheduler = SchedulerUseCases::new(scheduler_port);
        let buildings = BuildingUseCases::new(
            building_reads,
            building_executor,
            Arc::new(SystemClock),
            BuildingSettings {
                server_speed: config.speed,
            },
        );
        let village_profile = VillageProfileUseCases::new(village_profile_executor);
        let development = DevelopmentUseCases::new(
            development_reads,
            development_executor,
            DevelopmentSettings {
                server_speed: config.speed,
            },
        );
        let heroes = HeroUseCases::new(
            hero_reads,
            hero_executor,
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
            HeroSettings {
                server_speed: config.speed,
            },
        );
        let movements = MovementUseCases::new(
            movement_reads,
            movement_executor,
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
            MovementSettings {
                world_size: config.world_size as i32,
                server_speed: config.speed as u8,
            },
        );
        let movement_control = MovementControlUseCases::new(
            movement_control_reads,
            movement_control_executor,
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
        );
        let marketplace = MarketplaceUseCases::new(
            marketplace_reads,
            marketplace_executor,
            Arc::new(SystemClock),
            MarketplaceSettings {
                world_size: config.world_size as i32,
                server_speed: config.speed,
            },
        );
        let reinforcements = ReinforcementUseCases::new(
            reinforcement_reads,
            reinforcement_executor,
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
            ReinforcementSettings {
                world_size: config.world_size as i32,
                server_speed: config.speed as u8,
            },
        );
        let reports = ReportUseCases::new(report_reads, report_executor, Arc::new(SystemClock));
        let activity = VillageActivityUseCases::new(activity_reads, Arc::new(SystemClock));
        let army = VillageArmyUseCases::new(army_reads);
        let expansion = VillageExpansionUseCases::new(expansion_reads);
        let village_references = VillageReferenceUseCases::new(village_reference_reads);
        let village_state = VillageStateUseCases::new(village_state_reads);
        let traps = TrapUseCases::new(
            trap_reads,
            trap_executor,
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
        );
        Arc::new(GameApplication::new(
            identity,
            registration,
            leaderboards,
            map,
            village_profile,
            buildings,
            development,
            heroes,
            movements,
            movement_control,
            marketplace,
            reinforcements,
            reports,
            activity,
            army,
            expansion,
            village_references,
            village_state,
            traps,
            scheduler,
        ))
    }

    async fn start_web_server(
        game_app: Arc<GameApplication>,
        pool: PgPool,
        config: Arc<Config>,
    ) -> Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?
            .port();
        drop(listener);

        let state = AppState::new(game_app, pool, &config);
        tokio::spawn(WebRouter::serve(state, port));

        let base_url = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::new();
        let health_url = format!("{base_url}/health");
        let mut ready = false;
        for _ in 0..40 {
            if let Ok(response) = client.get(&health_url).send().await
                && response.status() == StatusCode::OK
            {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if !ready {
            return Err(ApplicationError::Unknown(
                "Web test app did not become ready".to_string(),
            ));
        }
        Ok(base_url)
    }

    #[allow(dead_code)]
    pub async fn setup_web_app() -> Result<(Arc<IsolatedSchemaHandle>, String)> {
        let (pool, schema_handle) = create_isolated_test_pool().await?;
        let config = Arc::new(Config::from_env());
        bootstrap_world_map(&pool, config.world_size).await?;
        let game_app = build_game_app(&pool, config.clone());
        let base_url = start_web_server(game_app, pool, config).await?;

        Ok((schema_handle, base_url))
    }

    #[allow(dead_code)]
    pub async fn setup_web_app_with_seeded_user()
    -> Result<(Arc<IsolatedSchemaHandle>, String, SeededAuthUser)> {
        let (pool, schema_handle) = create_isolated_test_pool().await?;
        let config = Arc::new(Config::from_env());
        bootstrap_world_map(&pool, config.world_size).await?;
        let game_app = build_game_app(&pool, config.clone());

        let short = &Uuid::new_v4().simple().to_string()[..10];
        let seeded = SeededAuthUser {
            username: format!("seeded{short}"),
            email: format!("seeded{short}@example.com"),
            password: "Password123!".to_string(),
        };
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let password_hash = hash_password(&seeded.password)?;

        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&seeded.email)
            .bind(password_hash)
            .execute(&pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::query(
            "INSERT INTO players (id, username, tribe, user_id, culture_points) VALUES ($1, $2, 'Teuton', $3, 0)",
        )
        .bind(player_id)
        .bind(&seeded.username)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let (village_id, x, y): (i32, i32, i32) = sqlx::query_as(
            "SELECT id, (position->>'x')::int, (position->>'y')::int FROM rm_map_fields WHERE village_id IS NULL ORDER BY id ASC LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let valley = Valley {
            id: village_id as u32,
            position: Position { x, y },
            topology: ValleyTopology(1, 1, 1, 1),
            player_id: None,
            village_id: None,
        };
        let player = Player {
            id: player_id,
            username: seeded.username.clone(),
            tribe: Tribe::Teuton,
            user_id,
            culture_points: 0,
        };
        let village = Village::new(
            format!("{}'s Village", seeded.username),
            &valley,
            &player,
            true,
            config.world_size as i32,
            config.speed,
        );
        let mut seeded_buildings = village.buildings().clone();
        seeded_buildings.push(VillageBuilding {
            slot_id: 28,
            building: Building::new(BuildingName::Marketplace, 1),
        });
        seeded_buildings.push(VillageBuilding {
            slot_id: 29,
            building: Building::new(BuildingName::Warehouse, 1),
        });
        seeded_buildings.push(VillageBuilding {
            slot_id: 30,
            building: Building::new(BuildingName::Granary, 1),
        });
        VillageEsService::new(pool.clone())
            .found_village(
                village_id as u32,
                &parabellum_app::villages::FoundVillage {
                    village_name: village.name.clone(),
                    position: village.position.clone(),
                    tribe: Tribe::Teuton,
                    player_id,
                    parent_village_id: None,
                    buildings: seeded_buildings,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        sqlx::query("UPDATE rm_map_fields SET village_id = $1, player_id = $2 WHERE id = $1")
            .bind(village_id)
            .bind(player_id)
            .execute(&pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let auth_user = game_app
            .authenticate_user(&seeded.username, &seeded.password)
            .await?;
        let player = game_app.get_player_by_user_id(auth_user.id).await?;
        let _ = game_app.list_player_village_states(player.id).await?;
        let villages_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::bigint FROM rm_village WHERE player_id = $1")
                .bind(player.id)
                .fetch_one(&pool)
                .await
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if villages_count == 0 {
            return Err(ApplicationError::Unknown(
                "seeded auth fixture created no village projection".to_string(),
            ));
        }

        let state = AppState::new(game_app.clone(), pool.clone(), &config);
        current_user_by_ids(&state, auth_user.id, None)
            .await
            .map_err(|_| {
                ApplicationError::Unknown(
                    "seeded auth fixture cannot resolve current user context".to_string(),
                )
            })?;
        let base_url = start_web_server(game_app, pool, config).await?;

        let client = reqwest::Client::new();
        let login_probe = client
            .post(format!("{base_url}/api/v1/auth/token/login"))
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "username": seeded.username,
                    "password": seeded.password,
                })
                .to_string(),
            )
            .send()
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if login_probe.status() != StatusCode::OK {
            let status = login_probe.status();
            let body = login_probe
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_string());
            return Err(ApplicationError::Unknown(format!(
                "seeded login probe failed ({status}): {body}"
            )));
        }

        Ok((schema_handle, base_url, seeded))
    }

    #[allow(dead_code)]
    pub async fn setup_http_client(cookie: Option<HeaderValue>, redirects: Option<u8>) -> Client {
        let redirect_policy = redirects.map_or(Policy::none(), |n| Policy::limited(n as usize));
        let client = Client::builder().redirect(redirect_policy);

        if cookie.is_none() {
            return client.build().unwrap();
        }

        let cookie = cookie.unwrap();
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert(header::COOKIE, cookie);
        client.default_headers(request_headers).build().unwrap()
    }
}
