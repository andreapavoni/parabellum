use dotenvy::dotenv;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub world_size: i16,
    pub speed: i8,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub token_signing_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let world_size = match env::var("PARABELLUM_WORLD_SIZE") {
            Ok(val) => val.parse::<i16>().unwrap_or(100),
            Err(_) => 100,
        };

        let speed = match env::var("PARABELLUM_SERVER_SPEED") {
            Ok(val) => val.parse::<i8>().unwrap_or(1).clamp(1, 10),
            Err(_) => 1,
        };

        let access_token_ttl_secs = match env::var("PARABELLUM_ACCESS_TOKEN_TTL_SECS") {
            Ok(val) => val.parse::<i64>().unwrap_or(900).max(60),
            Err(_) => 900,
        };
        let refresh_token_ttl_secs = match env::var("PARABELLUM_REFRESH_TOKEN_TTL_SECS") {
            Ok(val) => val.parse::<i64>().unwrap_or(2_592_000).max(300),
            Err(_) => 2_592_000,
        };
        let token_signing_key = match env::var("PARABELLUM_TOKEN_SIGNING_KEY") {
            Ok(val) => val,
            Err(_) => {
                tracing::warn!(
                    "PARABELLUM_TOKEN_SIGNING_KEY is not set; using insecure default key for local/dev usage"
                );
                "dev-insecure-signing-key-change-me".to_string()
            }
        };

        Self {
            world_size,
            speed,
            access_token_ttl_secs,
            refresh_token_ttl_secs,
            token_signing_key,
        }
    }
}
