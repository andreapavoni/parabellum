use anyhow::Result;
use parabellum::{
    app::{
        commands::{FoundVillage, RegisterPlayer},
        queries::GetUnoccupiedValley,
        App,
    },
    db::{establish_connection_pool, repository::PostgresRepository},
    game::models::Tribe,
    jobs::worker::JobWorker,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let db_pool = establish_connection_pool();
    let repo = Arc::new(PostgresRepository::new(db_pool));
    let config = Arc::new(parabellum::config::Config::from_env());

    let app = App::new(
        config,
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );

    let worker = Arc::new(JobWorker::new(repo.clone(), repo.clone(), repo.clone()));
    worker.run();

    println!("App initialized. Executing a use case");

    let register_player_cmd = RegisterPlayer::new("pavonz".to_string(), Tribe::Roman);
    let player = match app.register_player(register_player_cmd).await {
        Ok(p) => {
            println!("Player  '{}' successfully registered!", p.username);
            p
        }
        Err(e) => {
            eprintln!("Error during player registration: {}", e);
            return Err(e);
        }
    };

    let get_valley_query = GetUnoccupiedValley::new(None);
    let valley = app.get_unoccupied_valley(get_valley_query).await?;
    println!(
        "Found available valley at: x={}, y={}",
        valley.position.x, valley.position.y
    );

    let found_village_cmd = FoundVillage::new(player, valley.position);
    let village = app.found_village(found_village_cmd).await?;
    println!("Village '{}' has been successfully founded!", village.name);

    println!("Done.");

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}
