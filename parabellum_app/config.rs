use dotenvy::dotenv;
use std::env;

pub struct Config {
    pub world_size: i16,
    pub speed: i8,
    pub auth_cookie_secret: String,
    pub medal_period_type: String,
    pub medal_period_interval: i32,
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
        
        let medal_period_type = match env::var("PARABELLUM_MEDAL_PERIOD_TYPE") {
            Ok(val) => val,
            Err(_) => "Week".to_string(),
        };
        
        let medal_period_interval = match env::var("PARABELLUM_MEDAL_PERIOD_INTERVAL") {
            Ok(val) => val.parse::<i32>().unwrap_or(1),
            Err(_) => 1,
        };

        Self {
            world_size,
            speed,
            auth_cookie_secret,
            medal_period_type,
            medal_period_interval,
        }
    }
}
