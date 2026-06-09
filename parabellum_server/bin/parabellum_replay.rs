use std::env;

use parabellum_infra::es::{ReplayMode, ReplayRequest, ReplayService, ReplayTarget};
use parabellum_infra::establish_connection_pool;
use parabellum_server::logs::setup_logging;
use parabellum_types::errors::ApplicationError;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();

    let args = ReplayCliArgs::from_env_args(env::args().collect())?;
    if args.rebuild_snapshots {
        let pool = establish_connection_pool().await?;
        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let replay = ReplayService::new(pool);
        let count = replay
            .rebuild_all_snapshots()
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        tracing::info!(count, "snapshots rebuild completed");
        return Ok(());
    }

    tracing::info!(
        target = ?args.target,
        mode = ?args.mode,
        from_global_seq = args.from_global_seq,
        to_global_seq = args.to_global_seq,
        aggregate_id = args.aggregate_id.as_deref().unwrap_or(""),
        "starting replay CLI"
    );
    let pool = establish_connection_pool().await?;
    tracing::info!("running database migrations for replay");
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    let replay = ReplayService::new(pool);
    let summary = replay
        .replay(ReplayRequest {
            target: args.target,
            mode: args.mode,
            from_global_seq: args.from_global_seq,
            to_global_seq: args.to_global_seq,
            aggregate_id: args.aggregate_id,
        })
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

    tracing::info!(
        scanned = summary.scanned,
        applied = summary.applied,
        skipped = summary.skipped,
        first_global_seq = summary.first_global_seq,
        last_global_seq = summary.last_global_seq,
        "replay completed"
    );

    Ok(())
}

#[derive(Debug, Clone)]
struct ReplayCliArgs {
    rebuild_snapshots: bool,
    target: ReplayTarget,
    mode: ReplayMode,
    from_global_seq: i64,
    to_global_seq: Option<i64>,
    aggregate_id: Option<String>,
}

impl ReplayCliArgs {
    fn from_env_args(args: Vec<String>) -> Result<Self, ApplicationError> {
        let mut rebuild_snapshots = false;
        let mut target = ReplayTarget::All;
        let mut mode = ReplayMode::DryRun;
        let mut from_global_seq = 1_i64;
        let mut to_global_seq = None;
        let mut aggregate_id = None;

        let mut i = 1usize;
        while i < args.len() {
            match args[i].as_str() {
                "--rebuild-snapshots" => {
                    rebuild_snapshots = true;
                }
                "--target" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| {
                        ApplicationError::Unknown("missing --target value".to_string())
                    })?;
                    target = parse_target(value)?;
                }
                "--mode" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| {
                        ApplicationError::Unknown("missing --mode value".to_string())
                    })?;
                    mode = parse_mode(value)?;
                }
                "--from" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| {
                        ApplicationError::Unknown("missing --from value".to_string())
                    })?;
                    from_global_seq = value.parse::<i64>().map_err(|_| {
                        ApplicationError::Unknown("invalid --from value".to_string())
                    })?;
                }
                "--to" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| {
                        ApplicationError::Unknown("missing --to value".to_string())
                    })?;
                    to_global_seq = Some(value.parse::<i64>().map_err(|_| {
                        ApplicationError::Unknown("invalid --to value".to_string())
                    })?);
                }
                "--aggregate-id" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| {
                        ApplicationError::Unknown("missing --aggregate-id value".to_string())
                    })?;
                    aggregate_id = Some(value.clone());
                }
                "--help" | "-h" => {
                    return Err(ApplicationError::Unknown(help_text()));
                }
                unknown => {
                    return Err(ApplicationError::Unknown(format!(
                        "unknown argument: {unknown}\n{}",
                        help_text()
                    )));
                }
            }
            i += 1;
        }

        Ok(Self {
            rebuild_snapshots,
            target,
            mode,
            from_global_seq,
            to_global_seq,
            aggregate_id,
        })
    }
}

fn parse_target(value: &str) -> Result<ReplayTarget, ApplicationError> {
    match value {
        "village" => Ok(ReplayTarget::Village),
        "reports" => Ok(ReplayTarget::Reports),
        "all" => Ok(ReplayTarget::All),
        _ => Err(ApplicationError::Unknown(format!(
            "invalid --target value: {value}\n{}",
            help_text()
        ))),
    }
}

fn parse_mode(value: &str) -> Result<ReplayMode, ApplicationError> {
    match value {
        "dry-run" => Ok(ReplayMode::DryRun),
        "full" => Ok(ReplayMode::Full),
        _ => Err(ApplicationError::Unknown(format!(
            "invalid --mode value: {value}\n{}",
            help_text()
        ))),
    }
}

fn help_text() -> String {
    "Usage: parabellum-replay [--target village|reports|all] [--mode dry-run|full] [--from N] [--to N] [--aggregate-id ID] [--rebuild-snapshots]".to_string()
}
