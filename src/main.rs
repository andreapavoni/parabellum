// use std::sync::Arc;

use anyhow::{Error, Result};

// use parabellum::app::commands::Cmd;
// use parabellum::app::App;
// // use parabellum::db::repository::Repository;
// use parabellum::game::models::Tribe;
// use parabellum::repository::Repository as GameRepository;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // let db = Repository::new_from_env()
    //     .await
    //     .expect("failed to create repository");

    // // TODO: put this into a cli command as part of a reset/setup task
    // // use parabellum::repository::Repository as GameRepository;
    // db.bootstrap_new_map(100).await?;

    // // for _ in 0..10 {
    // //     let valley = db.get_unoccupied_valley(None).await?;
    // //     println!("Valley random -> {:?}", valley);
    // // }

    // // let valley = db
    // //     .get_unoccupied_valley(Some(parabellum::game::models::map::Quadrant::NorthEast))
    // //     .await?;
    // // println!("Valley NorthEast -> {:?}", valley);

    // let app = App::new(Arc::new(db.clone()));

    // app.command(Cmd::RegisterPlayer {
    //     username: "pavonz".to_string(),
    //     tribe: Tribe::Gaul,
    // })
    // .await?;

    Ok(())
}
