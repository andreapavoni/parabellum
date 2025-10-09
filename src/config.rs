use dotenvy::dotenv;
use std::env;

pub struct Config {
    pub world_size: i16,
    pub speed: i8,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let world_size = match env::var("PARABELLUM_WORLD_SIZE") {
            Ok(val) => val.parse::<i16>().unwrap_or(100),
            Err(_) => 100,
        };

        let speed = match env::var("PARABELLUM_SERVER_SPEED") {
            Ok(val) => {
                let speed = val.parse::<i8>().unwrap_or(1);

                if speed < 1 {
                    1
                } else if speed > 5 {
                    5
                } else {
                    speed
                }
            }
            Err(_) => 1,
        };

        Self { world_size, speed }
    }
}
