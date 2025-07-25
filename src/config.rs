// src/config.rs
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub payment_api_key: String,
    pub payment_api_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env() // Используем envy напрямую
    }
}
