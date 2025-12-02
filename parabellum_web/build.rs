use std::env;
use std::process::Command;

fn main() {
    if env::var("SKIP_FRONTEND").is_ok() {
        println!("cargo::warning=Skipping frontend build (SKIP_FRONTEND set)");
        return;
    }

    if Command::new("bun").arg("--version").output().is_err() {
        panic!("bun is not installed or not in PATH. Install it from https://bun.com/");
    }

    println!("cargo::rerun-if-changed=../frontend/src");
    println!("cargo::rerun-if-changed=../package.json");

    let status = Command::new("bun")
        .args(&["run", "build:release"])
        .current_dir("..")
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("cargo::warning=Frontend build completed");
        }
        Ok(status) => {
            panic!("Frontend build failed with exit code: {:?}", status.code());
        }
        Err(e) => panic!("Failed to execute bun: {}", e),
    }
}
