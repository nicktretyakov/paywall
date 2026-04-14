// src/ml.rs
// (без изменений — уже исправлено в v3.0, Send+Sync + background DB retrain + rayon + tokio::join!)
use crate::db;
use crate::models::MLFeatures;
use linfa::prelude::*;
use linfa_trees::DecisionTree;
use ndarray::prelude::*;
use rand::Rng;
use rayon::prelude::*;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

struct PaywallModelInner {
    model: DecisionTree<f32, usize>,
}

#[derive(Clone)]
pub struct PaywallModel {
    inner: Arc<RwLock<PaywallModelInner>>,
}

impl PaywallModel {
    pub async fn predict(&self, features: &MLFeatures) -> bool {
        let guard = self.inner.read().await;
        let feature_array = array![
            features.user_subscription_days as f32,
            features.user_avg_view_time as f32,
            features.content_popularity_score as f32,
            features.time_since_last_interaction as f32,
            features.user_total_interactions as f32,
            features.content_avg_interaction_score as f32,
        ];
        let input = Array2::from_shape_vec((1, 6), feature_array.to_vec()).unwrap();
        guard.model.predict(&input).get(0).copied().map_or(false, |l| l == 1)
    }
}

trait ModelTrainer: Send + Sync + 'static {
    fn train(&self, obs: Array2<f32>, tgt: Array1<usize>) -> Result<DecisionTree<f32, usize>, Box<dyn std::error::Error + Send + Sync>>;
}

struct DefaultTrainer;
impl ModelTrainer for DefaultTrainer {
    fn train(&self, obs: Array2<f32>, tgt: Array1<usize>) -> Result<DecisionTree<f32, usize>, Box<dyn std::error::Error + Send + Sync>> {
        let ds = Dataset::new(obs, tgt);
        DecisionTree::params().max_depth(Some(15)).fit(&ds)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

fn generate_synthetic_data(n: usize) -> (Array2<f32>, Array1<usize>) {
    let data: Vec<(Vec<f32>, usize)> = (0..n).into_par_iter().map(|_| {
        let mut rng = rand::thread_rng();
        let f = vec![
            rng.gen_range(0.0..365.0),
            rng.gen_range(0.0..600.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..86400.0 * 30.0),
            rng.gen_range(0.0..1000.0),
            rng.gen_range(0.0..1.0),
        ];
        let t = if f[1] > 100.0 && f[2] > 0.5 && f[4] > 10.0 { 1 } else { 0 };
        (f, t)
    }).collect();

    let obs: Vec<f32> = data.iter().flat_map(|(f,_)| f.clone()).collect();
    let tgt: Vec<usize> = data.iter().map(|(_,t)| *t).collect();
    (Array2::from_shape_vec((n, 6), obs).unwrap(), Array1::from_vec(tgt))
}

async fn background_retrainer(model: PaywallModel, pool: PgPool) {
    let trainer = DefaultTrainer;
    loop {
        sleep(Duration::from_secs(3600)).await;
        tracing::info!("🔥 BACKGROUND MULTITHREADED RETRAIN (rayon + tokio spawn + REAL DB data)");

        let (observations, targets) = match db::get_ml_training_data(&pool).await {
            Ok(data) if !data.is_empty() => {
                let obs_vec: Vec<f32> = data.iter().flat_map(|f| vec![
                    f.user_subscription_days as f32,
                    f.user_avg_view_time as f32,
                    f.content_popularity_score as f32,
                    f.time_since_last_interaction as f32,
                    f.user_total_interactions as f32,
                    f.content_avg_interaction_score as f32,
                ]).collect();
                let tgt_vec: Vec<usize> = data.iter().map(|_| 1usize).collect();
                (Array2::from_shape_vec((data.len(), 6), obs_vec).unwrap(), Array1::from_vec(tgt_vec))
            }
            _ => generate_synthetic_data(10000),
        };

        if let Ok(new_model) = trainer.train(observations, targets) {
            let mut g = model.inner.write().await;
            g.model = new_model;
            tracing::info!("✅ SOLID+ML retrained from DB in background thread");
        }
    }
}

pub async fn initialize_model(pool: &PgPool) -> Result<PaywallModel, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("🚀 Initializing SOLID+ML model with background DB retrainer...");
    let (o, t) = generate_synthetic_data(5000);
    let m = DefaultTrainer.train(o, t)?;
    let inner = Arc::new(RwLock::new(PaywallModelInner { model: m }));
    let clone_model = PaywallModel { inner: inner.clone() };
    tokio::spawn(background_retrainer(clone_model, pool.clone()));
    Ok(PaywallModel { inner })
}

pub async fn extract_features(pool: &PgPool, user_id: Uuid, content_id: Uuid) -> Result<MLFeatures, sqlx::Error> {
    let (a,b,c,d,e,f) = tokio::join!(
        db::get_user_subscription_days(pool, user_id),
        db::get_user_avg_view_time(pool, user_id),
        db::get_content_popularity_score(pool, content_id),
        db::get_time_since_last_interaction(pool, user_id),
        async { Ok::<f64, sqlx::Error>(db::get_user_total_interactions(pool, user_id).await? as f64) },
        db::get_content_avg_interaction_score(pool, content_id),
    );
    Ok(MLFeatures {
        user_id, content_id,
        user_subscription_days: a?, user_avg_view_time: b?,
        content_popularity_score: c?, time_since_last_interaction: d?,
        user_total_interactions: e?, content_avg_interaction_score: f?,
    })
}