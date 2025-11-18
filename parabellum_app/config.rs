use dotenvy::dotenv;
use std::env;

pub struct Config {
    pub world_size: i16,
    pub speed: i8,
    pub auth_cookie_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let world_size = match env::var("PARABELLUM_WORLD_SIZE") {
            Ok(val) => val.parse::<i16>().unwrap_or(100),
            Err(_) => 100,
        };

        let speed = match env::var("PARABELLUM_SERVER_SPEED") {
            Ok(val) => val.parse::<i8>().unwrap_or(1).clamp(1, 5),
            Err(_) => 1,
        };

        let auth_cookie_secret = match env::var("PARABELLUM_COOKIE_SECRET") {
            Ok(val) => val,
            Err(_) => panic!("You need to set env PARABELLUM_COOKIE_SECRET"),
        };

        Self {
            world_size,
            speed,
            auth_cookie_secret,
        }
    }
}
