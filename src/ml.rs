// src/ml.rs
use crate::db;
use crate::models::MLFeatures;
use linfa::prelude::*;
use linfa_trees::DecisionTree;
use ndarray::prelude::*;
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PaywallModel {
    model: DecisionTree<f32, usize>, // f32 for features, usize for labels
}

impl PaywallModel {
    pub fn predict(&self, features: &MLFeatures) -> bool {
        // Преобразуем f64 в f32
        let feature_array = array![
            features.user_subscription_days as f32,
            features.user_avg_view_time as f32,
            features.content_popularity_score as f32,
            features.time_since_last_interaction as f32,
            features.user_total_interactions as f32,
            features.content_avg_interaction_score as f32,
        ];

        if feature_array.len() != 6 {
            tracing::warn!(
                "Feature array length mismatch: expected 6, got {}",
                feature_array.len()
            );
            return false;
        }

        let input = Array2::from_shape_vec((1, 6), feature_array.to_vec()).unwrap();

        let prediction = self.model.predict(&input);
        // Предсказание класса (0 или 1)
        if let Some(&label) = prediction.get(0) {
            label == 1
        } else {
            false
        }
    }
}

// Генерация синтетических данных для обучения
fn generate_synthetic_data(n_samples: usize) -> (Array2<f32>, Array1<usize>) {
    let mut rng = rand::thread_rng();
    let mut observations = Vec::new();
    let mut targets = Vec::new();

    for _ in 0..n_samples {
        let user_subscription_days = rng.gen_range(0.0..365.0);
        let user_avg_view_time = rng.gen_range(0.0..600.0);
        let content_popularity_score = rng.gen_range(0.0..1.0);
        let time_since_last_interaction = rng.gen_range(0.0..86400.0 * 30.0);
        let user_total_interactions = rng.gen_range(0.0..1000.0);
        let content_avg_interaction_score = rng.gen_range(0.0..1.0);

        let features = array![
            user_subscription_days,
            user_avg_view_time,
            content_popularity_score,
            time_since_last_interaction,
            user_total_interactions,
            content_avg_interaction_score,
        ];

        // Простое правило для генерации целевой переменной (1 или 0)
        let target = if user_avg_view_time > 100.0
            && content_popularity_score > 0.5
            && user_total_interactions > 10.0
        {
            1
        } else {
            0
        };

        observations.extend(features.to_vec());
        targets.push(target);
    }

    let observations_array = Array2::from_shape_vec((n_samples, 6), observations).unwrap();
    let targets_array = Array1::from_vec(targets);
    (observations_array, targets_array)
}

pub async fn initialize_model(_pool: &PgPool) -> Result<PaywallModel, Box<dyn std::error::Error>> {
    tracing::info!("Initializing ML model...");

    // Всегда используем синтетические данные для примера
    // В реальном приложении здесь будет загрузка из БД
    let (observations, targets) = generate_synthetic_data(1000);

    if observations.nrows() == 0 {
        return Err("No training data available".into());
    }

    let dataset = Dataset::new(observations, targets);

    // Исправлены параметры и убран несуществующий метод
    let model = DecisionTree::params()
        .max_depth(Some(5)) // Упрощенный способ установки параметров
        .fit(&dataset)
        .map_err(|e| format!("Failed to train model: {}", e))?;

    tracing::info!("ML model initialized successfully");
    Ok(PaywallModel { model })
}

// Исправленная функция извлечения признаков с правильной обработкой ошибок
pub async fn extract_features(
    pool: &PgPool,
    user_id: Uuid,
    content_id: Uuid,
) -> Result<MLFeatures, sqlx::Error> {
    let user_subscription_days = db::get_user_subscription_days(pool, user_id).await?;
    let user_avg_view_time = db::get_user_avg_view_time(pool, user_id).await?;
    let content_popularity_score = db::get_content_popularity_score(pool, content_id).await?;
    let time_since_last_interaction = db::get_time_since_last_interaction(pool, user_id).await?;
    let user_total_interactions = db::get_user_total_interactions(pool, user_id).await? as f64;
    let content_avg_interaction_score =
        db::get_content_avg_interaction_score(pool, content_id).await?;

    Ok(MLFeatures {
        user_id,
        content_id,
        user_subscription_days,
        user_avg_view_time,
        content_popularity_score,
        time_since_last_interaction,
        user_total_interactions,
        content_avg_interaction_score,
    })
}
