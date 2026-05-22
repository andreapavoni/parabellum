use std::{fs, path::PathBuf};

use parabellum_app::config::Config;
use parabellum_infra::seed::{parse_seed_file, run_seed};
use parabellum_infra::establish_connection_pool;
use parabellum_types::errors::ApplicationError;

fn seed_inputs_from_args() -> PathBuf {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let default_path = PathBuf::from("seed/default.json");

    match args.as_slice() {
        [] => default_path,
        [one] if one.ends_with(".json") => PathBuf::from(one),
        _ => default_path,
    }
}

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let path = seed_inputs_from_args();
    let raw = fs::read_to_string(&path)
        .map_err(|e| ApplicationError::Unknown(format!("cannot read {}: {e}", path.display())))?;
    let seed = parse_seed_file(&raw)?;

    let config = Config::from_env();
    let pool = establish_connection_pool().await?;
    let results = run_seed(&pool, &config, seed, &path).await?;

    println!("Seed completed from {}:", path.display());
    for result in results {
        println!(
            "  username={}\n  email={}\n  password={}\n  village='{}'\n  village_id={}\n  position=({}, {})",
            result.username,
            result.email,
            result.password,
            result.village_name,
            result.village_id,
            result.village_position.x,
            result.village_position.y
        );
    }

    Ok(())
}
