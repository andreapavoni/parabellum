use sqlx::PgPool;
use std::sync::Arc;

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
        BuildingSettings, BuildingUseCases, DevelopmentSettings, DevelopmentUseCases, HeroSettings,
        HeroUseCases, MarketplaceSettings, MarketplaceUseCases, MovementControlUseCases,
        MovementSettings, MovementUseCases, ReinforcementSettings, ReinforcementUseCases,
        ReportUseCases, SystemClock, TrapUseCases, UuidGenerator, VillageActivityUseCases,
        VillageArmyUseCases, VillageExpansionUseCases, VillageProfileUseCases,
        VillageReferenceUseCases, VillageStateUseCases,
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
use parabellum_infra::identity::{IdentityService, repositories::PostgresPlayerRepository};
use parabellum_infra::{
    adapters::VillageEsAdapter, bootstrap_world_map, es::EsScheduledActionWorker,
    es::VillageEsService, establish_connection_pool, map::PostgresMapRepository,
};
use parabellum_server::logs::setup_logging;
use parabellum_types::{Result, errors::ApplicationError};
use parabellum_web::{AppState, WebRouter};
use tracing::{error, info};

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();
    info!("starting parabellum runtime");
    let (config, game_app, es_worker, db_pool) = setup_app().await?;
    let state = AppState::new(game_app, db_pool, &config);
    let port = config.port;

    es_worker.run();
    info!(port, "runtime initialized; launching web server");
    WebRouter::serve(state, port).await
}

async fn setup_app() -> Result<
    (
        Arc<Config>,
        Arc<GameApplication>,
        Arc<EsScheduledActionWorker>,
        PgPool,
    ),
    ApplicationError,
> {
    info!("loading runtime configuration and database connection");
    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;

    info!("running database migrations");
    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    info!("ensuring world map state");
    setup_world_map(&db_pool, &config).await?;

    let village_service = VillageEsService::new(db_pool.clone());
    let game_app = build_game_application(db_pool.clone(), config.clone(), village_service.clone());
    let es_worker = Arc::new(EsScheduledActionWorker::new(village_service, 1000));

    Ok((config, game_app, es_worker, db_pool))
}

fn build_game_application(
    db_pool: PgPool,
    config: Arc<Config>,
    village_service: VillageEsService,
) -> Arc<GameApplication> {
    let identity = Arc::new(IdentityService::new(db_pool.clone()));
    let leaderboards =
        LeaderboardUseCases::new(Arc::new(PostgresPlayerRepository::new(db_pool.clone())));
    let map = MapUseCases::new(Arc::new(PostgresMapRepository::new(db_pool)));
    let movement_settings = MovementSettings {
        world_size: config.world_size as i32,
        server_speed: config.speed as u8,
    };
    let marketplace_settings = MarketplaceSettings {
        world_size: config.world_size as i32,
        server_speed: config.speed,
    };
    let reinforcement_settings = ReinforcementSettings {
        world_size: config.world_size as i32,
        server_speed: config.speed as u8,
    };
    let building_settings = BuildingSettings {
        server_speed: config.speed,
    };
    let development_settings = DevelopmentSettings {
        server_speed: config.speed,
    };
    let hero_settings = HeroSettings {
        server_speed: config.speed,
    };
    let villages_adapter = Arc::new(VillageEsAdapter::new(village_service));
    let registration_identities: Arc<dyn RegistrationIdentityPort> = identity.clone();
    let initial_village_executor: Arc<dyn InitialVillageCommandExecutor> = villages_adapter.clone();
    let registration = RegistrationUseCases::new(
        registration_identities,
        initial_village_executor,
        Arc::new(UuidGenerator),
        RegistrationSettings {
            world_size: config.world_size as i32,
            server_speed: config.speed,
        },
    );
    let building_reads: Arc<dyn BuildingReadPort> = villages_adapter.clone();
    let building_executor: Arc<dyn BuildingCommandExecutor> = villages_adapter.clone();
    let development_reads: Arc<dyn DevelopmentReadPort> = villages_adapter.clone();
    let development_executor: Arc<dyn DevelopmentCommandExecutor> = villages_adapter.clone();
    let hero_reads: Arc<dyn HeroReadPort> = villages_adapter.clone();
    let hero_executor: Arc<dyn HeroCommandExecutor> = villages_adapter.clone();
    let village_profile_executor: Arc<dyn VillageProfileCommandExecutor> = villages_adapter.clone();
    let movement_reads: Arc<dyn MovementReadPort> = villages_adapter.clone();
    let movement_executor: Arc<dyn VillageCommandExecutor> = villages_adapter.clone();
    let movement_control_reads: Arc<dyn MovementControlReadPort> = villages_adapter.clone();
    let movement_control_executor: Arc<dyn MovementControlCommandExecutor> =
        villages_adapter.clone();
    let marketplace_reads: Arc<dyn MarketplaceReadPort> = villages_adapter.clone();
    let marketplace_executor: Arc<dyn MarketplaceCommandExecutor> = villages_adapter.clone();
    let reinforcement_reads: Arc<dyn ReinforcementReadPort> = villages_adapter.clone();
    let reinforcement_executor: Arc<dyn ReinforcementCommandExecutor> = villages_adapter.clone();
    let report_reads: Arc<dyn ReportReadPort> = villages_adapter.clone();
    let report_executor: Arc<dyn ReportCommandExecutor> = villages_adapter.clone();
    let activity_reads: Arc<dyn VillageActivityReadPort> = villages_adapter.clone();
    let army_reads: Arc<dyn VillageArmyReadPort> = villages_adapter.clone();
    let expansion_reads: Arc<dyn ExpansionReadPort> = villages_adapter.clone();
    let village_reference_reads: Arc<dyn VillageReferenceReadPort> = villages_adapter.clone();
    let village_state_reads: Arc<dyn VillageStateReadPort> = villages_adapter.clone();
    let trap_reads: Arc<dyn TrapReadPort> = villages_adapter.clone();
    let trap_executor: Arc<dyn TrapCommandExecutor> = villages_adapter.clone();
    let scheduler_port: Arc<dyn SchedulerPort> = villages_adapter.clone();
    let scheduler = SchedulerUseCases::new(scheduler_port);
    let buildings = BuildingUseCases::new(
        building_reads,
        building_executor,
        Arc::new(SystemClock),
        building_settings,
    );
    let village_profile = VillageProfileUseCases::new(village_profile_executor);
    let development = DevelopmentUseCases::new(
        development_reads,
        development_executor,
        development_settings,
    );
    let heroes = HeroUseCases::new(
        hero_reads,
        hero_executor,
        Arc::new(SystemClock),
        Arc::new(UuidGenerator),
        hero_settings,
    );
    let movements = MovementUseCases::new(
        movement_reads,
        movement_executor,
        Arc::new(SystemClock),
        Arc::new(UuidGenerator),
        movement_settings,
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
        marketplace_settings,
    );
    let reinforcements = ReinforcementUseCases::new(
        reinforcement_reads,
        reinforcement_executor,
        Arc::new(SystemClock),
        Arc::new(UuidGenerator),
        reinforcement_settings,
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

async fn setup_world_map(pool: &PgPool, config: &Config) -> Result<(), ApplicationError> {
    match bootstrap_world_map(pool, config.world_size).await {
        Ok(true) => tracing::info!("World Map successfully bootstrapped."),
        Ok(false) => tracing::info!("World Map already set. Skipping bootstrap."),
        Err(e) => {
            error!(error = %e, "world map initialization failed");
            return Err(e);
        }
    }

    Ok(())
}
