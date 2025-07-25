// src/paywall.rs
use crate::auth; // Для проверки токена
use crate::config::Config;
use crate::db;
use crate::ml; // Для ML анализа
use crate::models::{PurchaseRequest, Subscription, UserBehavior}; // Убран Content
use actix_web::{HttpRequest, HttpResponse, get, post, web}; // Убраны неиспользуемые
use chrono::Utc;
use moka::future::Cache;
use serde_json::json;
use uuid::Uuid;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_content);
    cfg.service(purchase_subscription);
    cfg.service(get_user_profile);
}

#[get("/content/{content_id}")]
pub async fn get_content(
    pool: web::Data<sqlx::PgPool>,
    ml_model: web::Data<ml::PaywallModel>,
    cache: web::Data<Cache<String, serde_json::Value>>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let content_id = path.into_inner();

    let user_id = match auth::get_user_id_from_request(&req) {
        Some(id) => id,
        None => return Ok(HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}))),
    };

    let cache_key = format!("content_{}_user_{}", content_id, user_id);
    if let Some(cached_response) = cache.get(&cache_key).await {
        tracing::info!("Cache hit for key: {}", cache_key);
        return Ok(HttpResponse::Ok().json(cached_response));
    }

    // Получение контента с обработкой ошибок
    let content = match db::get_content_by_id(&pool, content_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(HttpResponse::NotFound().json(json!({"error": "Content not found"}))),
        Err(e) => {
            tracing::error!("Database error fetching content: {}", e);
            return Ok(
                HttpResponse::InternalServerError().json(json!({"error": "Internal server error"}))
            );
        }
    };

    // Проверка подписки с обработкой ошибок
    let subscription = match db::get_active_subscription(&pool, user_id).await {
        Ok(sub_opt) => sub_opt,
        Err(e) => {
            tracing::error!("Database error fetching subscription: {}", e);
            return Ok(
                HttpResponse::InternalServerError().json(json!({"error": "Internal server error"}))
            );
        }
    };

    let has_access = match &subscription {
        Some(sub) => match content.required_plan.as_str() {
            "free" => true,
            "basic" => sub.plan_id == "basic" || sub.plan_id == "premium",
            "premium" => sub.plan_id == "premium",
            _ => false,
        },
        None => content.required_plan == "free",
    };

    let response = if has_access {
        let behavior = UserBehavior {
            user_id,
            content_id,
            view_time_seconds: 0,
            scroll_depth_percent: 0.0,
            interaction_score: 0.0,
            timestamp: Utc::now(),
        };
        // Логируем поведение, но не прерываем запрос при ошибке
        if let Err(e) = db::log_user_behavior(&pool, &behavior).await {
            tracing::warn!("Failed to log user behavior: {}", e);
        }

        json!({
            "content": content,
            "access_granted": true,
        })
    } else {
        // Извлечение признаков с обработкой ошибок
        let features_result = ml::extract_features(&pool, user_id, content_id).await;
        let ml_decision = match features_result {
            Ok(features) => ml_model.predict(&features),
            Err(e) => {
                tracing::error!("Feature extraction error: {}", e);
                false // В случае ошибки ML, доступ не предоставляется
            }
        };

        if ml_decision {
            json!({
                "content": content,
                "access_granted": false,
                "ml_suggestion": "Access can be granted with a discount or trial",
            })
        } else {
            json!({
                "content": content,
                "access_granted": false,
                "message": "Upgrade your subscription to access this content",
            })
        }
    };

    cache.insert(cache_key.clone(), response.clone()).await;
    tracing::info!("Cached response for key: {}", cache_key);

    Ok(HttpResponse::Ok().json(response))
}

// Фиктивная интеграция с платежной системой
async fn process_payment(
    _config: &Config,
    _token: &str,
    _amount: f64,
    _currency: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    tracing::info!(
        "Processing payment: token={}, amount={}, currency={}",
        _token,
        _amount,
        _currency
    );
    Ok(true) // Всегда успешно для демонстрации
}

#[post("/subscription/purchase")]
pub async fn purchase_subscription(
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<Config>,
    req: HttpRequest,
    purchase_req: web::Json<PurchaseRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match auth::get_user_id_from_request(&req) {
        Some(id) => id,
        None => return Ok(HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}))),
    };

    let (amount, duration_days) = match purchase_req.plan_id.as_str() {
        "basic" => (9.99, 30),
        "premium" => (19.99, 30),
        _ => return Ok(HttpResponse::BadRequest().json(json!({"error": "Invalid plan"}))),
    };

    match process_payment(&config, &purchase_req.payment_token, amount, "USD").await {
        Ok(true) => {
            let new_subscription = Subscription {
                id: Uuid::new_v4(),
                user_id,
                plan_id: purchase_req.plan_id.clone(),
                started_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::days(duration_days),
                is_active: true,
            };

            // Обработка ошибки создания подписки
            match db::create_subscription(&pool, &new_subscription).await {
                Ok(()) => Ok(HttpResponse::Ok().json(json!({
                    "message": "Subscription purchased successfully",
                    "subscription_id": new_subscription.id,
                    "expires_at": new_subscription.expires_at,
                }))),
                Err(e) => {
                    tracing::error!("Subscription creation error: {}", e);
                    Ok(HttpResponse::InternalServerError()
                        .json(json!({"error": "Internal server error"})))
                }
            }
        }
        Ok(false) => Ok(HttpResponse::PaymentRequired().json(json!({"error": "Payment failed"}))),
        Err(e) => {
            tracing::error!("Payment processing error: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(json!({"error": "Payment processing error"})))
        }
    }
}

#[get("/user/profile")]
pub async fn get_user_profile(
    pool: web::Data<sqlx::PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match auth::get_user_id_from_request(&req) {
        Some(id) => id,
        None => return Ok(HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}))),
    };

    // Обработка ошибки получения профиля
    match db::get_user_profile(&pool, user_id).await {
        Ok(profile) => Ok(HttpResponse::Ok().json(profile)),
        Err(e) => {
            tracing::error!("Database error fetching user profile: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({"error": "Internal server error"})))
        }
    }
}
