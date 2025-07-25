// src/db.rs
use crate::models::{Content, Subscription, User, UserBehavior};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row}; // Row для доступа к полям
use uuid::Uuid;

pub async fn get_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, email, password_hash, created_at FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn create_user(pool: &PgPool, user: &User) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (id, username, email, password_hash, created_at) VALUES ($1, $2, $3, $4, $5)")
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.created_at)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_active_subscription(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT id, user_id, plan_id, started_at, expires_at, is_active FROM subscriptions WHERE user_id = $1 AND is_active = true AND expires_at > NOW()"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn create_subscription(
    pool: &PgPool,
    subscription: &Subscription,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO subscriptions (id, user_id, plan_id, started_at, expires_at, is_active) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(subscription.id)
        .bind(subscription.user_id)
        .bind(&subscription.plan_id)
        .bind(subscription.started_at)
        .bind(subscription.expires_at)
        .bind(subscription.is_active)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_content_by_id(
    pool: &PgPool,
    content_id: Uuid,
) -> Result<Option<Content>, sqlx::Error> {
    sqlx::query_as::<_, Content>(
        "SELECT id, title, body, required_plan, created_at FROM content WHERE id = $1",
    )
    .bind(content_id)
    .fetch_optional(pool)
    .await
}

pub async fn log_user_behavior(pool: &PgPool, behavior: &UserBehavior) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO user_behaviors (user_id, content_id, view_time_seconds, scroll_depth_percent, interaction_score, timestamp) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(behavior.user_id)
        .bind(behavior.content_id)
        .bind(behavior.view_time_seconds)
        .bind(behavior.scroll_depth_percent)
        .bind(behavior.interaction_score)
        .bind(behavior.timestamp)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_profile(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<serde_json::Value, sqlx::Error> {
    // Получаем информацию о подписке
    let subscription_row = sqlx::query(
        "SELECT plan_id, started_at, expires_at FROM subscriptions WHERE user_id = $1 AND is_active = true ORDER BY started_at DESC LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let subscription_info = subscription_row.map(|row| {
        serde_json::json!({
            "plan_id": row.get::<String, _>("plan_id"),
            "started_at": row.get::<DateTime<Utc>, _>("started_at"),
            "expires_at": row.get::<DateTime<Utc>, _>("expires_at"),
        })
    });

    // Получаем статистику взаимодействий
    let behavior_stats_row: (i64, Option<f64>) = sqlx::query_as(
        "SELECT COUNT(*), AVG(interaction_score) FROM user_behaviors WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let avg_score = behavior_stats_row.1.unwrap_or(0.0);

    Ok(serde_json::json!({
        "user_id": user_id,
        "subscription": subscription_info,
        "total_interactions": behavior_stats_row.0,
        "avg_interaction_score": avg_score,
    }))
}

// Функции для обучения ML модели (остаются как заглушки или для будущего использования)
pub async fn get_ml_training_data(
    _pool: &PgPool,
) -> Result<Vec<crate::models::MLFeatures>, sqlx::Error> {
    // Заглушка или реализация для получения реальных данных
    Ok(Vec::new())
}

pub async fn get_ml_targets(_pool: &PgPool) -> Result<Vec<f32>, sqlx::Error> {
    // Заглушка
    Ok(Vec::new())
}

// Исправленные функции для извлечения признаков ML
pub async fn get_user_subscription_days(pool: &PgPool, user_id: Uuid) -> Result<f64, sqlx::Error> {
    let result: Option<f64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (MAX(expires_at) - MIN(started_at))) / 86400.0 FROM subscriptions WHERE user_id = $1 AND is_active = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten(); // Обрабатываем Option<Option<f64>>
    Ok(result.unwrap_or(0.0))
}

pub async fn get_user_avg_view_time(pool: &PgPool, user_id: Uuid) -> Result<f64, sqlx::Error> {
    let result: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(view_time_seconds)::double precision FROM user_behaviors WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(result.unwrap_or(0.0))
}

pub async fn get_content_popularity_score(
    pool: &PgPool,
    content_id: Uuid,
) -> Result<f64, sqlx::Error> {
    // Простая метрика популярности: доля просмотров этого контента от общего числа просмотров
    let total_views: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_behaviors")
        .fetch_one(pool)
        .await
        .unwrap_or(1); // Избегаем деления на 0

    let content_views: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_behaviors WHERE content_id = $1")
            .bind(content_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Ok((content_views as f64) / (total_views as f64).max(1.0)) // max(1.0) для избежания 0
}

pub async fn get_time_since_last_interaction(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<f64, sqlx::Error> {
    let result: Option<f64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (NOW() - MAX(timestamp))) FROM user_behaviors WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(result.unwrap_or(0.0)) // 0 если нет взаимодействий
}

pub async fn get_user_total_interactions(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let result: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_behaviors WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(result)
}

pub async fn get_content_avg_interaction_score(
    pool: &PgPool,
    content_id: Uuid,
) -> Result<f64, sqlx::Error> {
    let result: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(interaction_score)::double precision FROM user_behaviors WHERE content_id = $1",
    )
    .bind(content_id)
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(result.unwrap_or(0.0))
}
