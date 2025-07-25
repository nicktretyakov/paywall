// src/main.rs
use actix_web::{App, HttpServer, middleware::Logger, web};
use moka::future::Cache;
use sqlx::PgPool;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod auth;
mod config;
mod db;
mod ml;
mod models;
mod paywall;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    tracing::info!("Starting advanced paywall service");

    dotenv::dotenv().ok();
    let config = config::Config::from_env().expect("Failed to load config from environment");

    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    let cache: Cache<String, serde_json::Value> = Cache::new(1000);

    let ml_model = ml::initialize_model(&pool)
        .await
        .expect("Failed to initialize ML model");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(ml_model.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .configure(auth::init_routes)
            .configure(paywall::init_routes)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
