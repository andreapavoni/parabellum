// use std::sync::Arc;

// use std::sync::Arc;

use anyhow::{Error, Result};
// use parabellum::{
//     app::{commands::RegisterPlayer, App},
//     db::repository::Repository,
//     game::models::Tribe,
// };

#[tokio::main]
async fn main() -> Result<(), Error> {
    // let db = Arc::new(
    //     Repository::new_from_env()
    //         .await
    //         .expect("failed to create repository"),
    // );

    // TODO: put this into a cli command as part of a reset/setup task
    // db.bootstrap_new_map(100).await?;

    // let app = App::new();

    // app.command(RegisterPlayer::new("pavonz".to_string(), Tribe::Roman))
    //     .await?;

    Ok(())
}
