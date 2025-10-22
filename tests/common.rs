use std::process::Command;

use anyhow::Result;

pub async fn setup() -> Result<()> {
    let status = Command::new("sh")
        .arg("./setup_test_db.sh")
        .status()
        .expect("Failed to execute setup_test_db.sh");

    if !status.success() {
        panic!("setup_test_db.sh exit code: {:?}", &status.success());
    }

    Ok(())
}
