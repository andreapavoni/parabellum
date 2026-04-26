use std::env;

use parabellum_types::errors::DbError;

pub async fn establish_toasty_db() -> Result<toasty::Db, DbError> {
    init_toasty_db("DATABASE_URL").await
}

pub async fn establish_test_toasty_db() -> Result<toasty::Db, DbError> {
    init_toasty_db("TEST_DATABASE_URL").await
}

async fn init_toasty_db(database_env: &'static str) -> Result<toasty::Db, DbError> {
    dotenvy::dotenv().ok();

    let database_url =
        env::var(database_env).unwrap_or_else(|_| panic!("{} must be set", database_env));

    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!());

    builder
        .connect(&database_url)
        .await
        .map_err(|e| DbError::Transaction(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn toasty_connect_and_open_transaction() {
        let mut db = establish_test_toasty_db()
            .await
            .expect("toasty db should connect");
        let tx = db.transaction().await.expect("toasty tx should start");
        drop(tx);
    }
}
