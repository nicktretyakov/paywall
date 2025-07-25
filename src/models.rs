// src/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid; // Добавлен импорт

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)] // Добавлен FromRow
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)] // Добавлен FromRow
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)] // Добавлен FromRow
pub struct Content {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub required_plan: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserBehavior {
    pub user_id: Uuid,
    pub content_id: Uuid,
    pub view_time_seconds: i32,
    pub scroll_depth_percent: f32,
    pub interaction_score: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,
}

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct PurchaseRequest {
    pub plan_id: String,
    pub payment_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)] // Clone для использования в ML
pub struct MLFeatures {
    pub user_id: Uuid,
    pub content_id: Uuid,
    pub user_subscription_days: f64, // Изменено на f64 для совместимости с SQL
    pub user_avg_view_time: f64,     // Изменено на f64
    pub content_popularity_score: f64, // Изменено на f64
    pub time_since_last_interaction: f64, // Изменено на f64
    pub user_total_interactions: f64, // Изменено на f64 для соответствия ML модели
    pub content_avg_interaction_score: f64, // Изменено на f64
}
