// src/auth.rs
use crate::config::Config;
use crate::db;
use crate::models::{Claims, LoginRequest, RegisterRequest, User};
use actix_web::{
    HttpMessage, // Для extensions() и extensions_mut()
    HttpRequest, // Убран Scope
    HttpResponse,
    post,
    web,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::encode;
use serde_json::json;
use uuid::Uuid;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(register);
}

// Получение user_id из расширений запроса
pub fn get_user_id_from_request(req: &HttpRequest) -> Option<Uuid> {
    req.extensions()
        .get::<String>()
        .and_then(|id_str| Uuid::parse_str(id_str).ok())
}

#[post("/auth/login")]
pub async fn login(
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<Config>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_result = db::get_user_by_username(&pool, &req.username).await;

    match user_result {
        Ok(Some(user)) => match verify(&req.password, &user.password_hash) {
            Ok(true) => {
                let expiration = Utc::now() + Duration::hours(24);
                let claims = Claims {
                    sub: user.id.to_string(),
                    exp: expiration.timestamp() as usize,
                };
                match encode(
                    &jsonwebtoken::Header::default(),
                    &claims,
                    &jsonwebtoken::EncodingKey::from_secret(config.jwt_secret.as_ref()),
                ) {
                    Ok(token) => Ok(HttpResponse::Ok().json(json!({
                        "token": token,
                        "user_id": user.id,
                    }))),
                    Err(e) => {
                        tracing::error!("Token generation error: {}", e);
                        Ok(HttpResponse::InternalServerError()
                            .json(json!({"error": "Internal server error"})))
                    }
                }
            }
            Ok(false) | Err(_) => {
                Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid credentials"})))
            }
        },
        Ok(None) => Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid credentials"}))),
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({"error": "Internal server error"})))
        }
    }
}

#[post("/auth/register")]
pub async fn register(
    pool: web::Data<sqlx::PgPool>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let check_result = db::get_user_by_username(&pool, &req.username).await;
    match check_result {
        Ok(Some(_)) => {
            return Ok(HttpResponse::Conflict().json(json!({"error": "Username already exists"})));
        }
        Err(e) => {
            tracing::error!("Database error during registration check: {}", e);
            return Ok(
                HttpResponse::InternalServerError().json(json!({"error": "Internal server error"}))
            );
        }
        _ => {} // User not found, proceed
    }

    let hashed_password_result = hash(&req.password, DEFAULT_COST);
    let hashed_password = match hashed_password_result {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Password hashing error: {}", e);
            return Ok(
                HttpResponse::InternalServerError().json(json!({"error": "Internal server error"}))
            );
        }
    };

    let new_user = User {
        id: Uuid::new_v4(),
        username: req.username.clone(),
        email: req.email.clone(),
        password_hash: hashed_password,
        created_at: Utc::now(),
    };

    let create_result = db::create_user(&pool, &new_user).await;
    match create_result {
        Ok(()) => Ok(HttpResponse::Created().json(json!({
            "message": "User created successfully",
            "user_id": new_user.id,
        }))),
        Err(e) => {
            tracing::error!("User creation error: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({"error": "Internal server error"})))
        }
    }
}
