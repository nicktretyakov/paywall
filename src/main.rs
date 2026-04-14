// src/main.rs
// ИСПРАВЛЕННАЯ + УСЛОЖНЁННАЯ (SOLID DI + multithreading + ML anomaly validator)
use actix_web::{App, HttpServer, middleware::Logger, web};
use moka::future::Cache;
use sqlx::PgPool;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod auth;
mod config;
mod db;
mod middleware;
mod ml;
mod models;
mod paywall;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing");

    tracing::info!("🚀 ULTRA-FAST paywall v3.0 (SOLID + ML + MULTITHREADING + EITHERBODY + RAYON DB RETRAIN)");

    dotenv::dotenv().ok();
    let config = config::Config::from_env().expect("Failed to load config");

    let pool = PgPool::connect(&config.database_url).await.expect("Postgres connect failed");

    let cache: Cache<String, serde_json::Value> = Cache::new(10000);

    let ml_model = ml::initialize_model(&pool).await.expect("ML init failed");

    // SOLID: можно переключить на MLAnomalyValidator без изменения middleware
    let jwt_validator: Arc<middleware::JwtValidator> = Arc::new(middleware::JwtValidator::new(Arc::new(config.clone())));

    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(16);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(ml_model.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .configure(auth::init_routes)
            .service(
                web::scope("/")
                    .wrap(middleware::JwtMiddleware::new(jwt_validator.clone()))
                    .configure(paywall::init_routes)
            )
    })
    .workers(workers)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}