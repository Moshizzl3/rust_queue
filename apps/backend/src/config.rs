use envy::from_env;
use serde::Deserialize;

fn default_port() -> i32 {
    8000
}

fn deserialize_cors_origins<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: i32,
    pub database_url: String,
    pub password_pepper: String,
    pub running_in_cloud: bool,
    pub jwt_secret: String,
    pub jwt_access_expiry_mins: Option<i64>,
    pub jwt_refresh_expiry_days: Option<i64>,
    #[serde(deserialize_with = "deserialize_cors_origins")]
    pub cors_origins: Vec<String>,
}

pub fn load_config() -> Config {
    dotenv::dotenv().ok();
    from_env().expect("Failed to load config from environment")
}
