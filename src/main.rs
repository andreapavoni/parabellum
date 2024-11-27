use anyhow::{Error, Result};

use parabellum::db::repository::Repository;
use parabellum::repository::Repository as GameRepository;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Repository::new_from_env()
        .await
        .expect("failed to create repository");

    db.bootstrap_new_map(100).await?;
    // let v = db.get_village_by_id(100).await?;
    // println!("Village? v")

    Ok(())
}
