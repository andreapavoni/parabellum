use std::{fs, path::PathBuf};

use parabellum_app::config::Config;
use parabellum_infra::establish_connection_pool;
use parabellum_infra::seed::{SeedRunResult, parse_seed_file, run_seed};
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

fn print_seed_results(path: &PathBuf, results: Vec<SeedRunResult>) {
    println!("Seed completed from {}:", path.display());
    let mut current_player: Option<(String, String)> = None;
    for result in results {
        let player_key = (result.username.clone(), result.email.clone());
        if current_player.as_ref() != Some(&player_key) {
            if current_player.is_some() {
                println!();
            }
            println!(
                "  username={}\n  email={}\n  password={}",
                result.username, result.email, result.password
            );
            current_player = Some(player_key);
        }
        println!(
            "    village='{}' village_id={} position=({}, {})",
            result.village_name,
            result.village_id,
            result.village_position.x,
            result.village_position.y
        );
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

    print_seed_results(&path, results);

    Ok(())
}
