use axum::{routing::post, Json, Router};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Deserialize)]
struct ScreenRequest {
    tx_hash: String,
    sender: String,
    recipient: String,
}

#[derive(Serialize)]
struct ScreenResponse {
    tx_hash: String,
    decision: String,
    risk_score: i32,
    reasons: Vec<String>,
}

struct AppState {
    db: PgPool,
    redis: redis::Client,
}

const SANCTIONS_SET_KEY: &str = "sanctioned_addresses";

async fn load_sanctions_into_redis(db: &PgPool, redis_client: &redis::Client) -> eyre::Result<usize> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;

    // Clear any stale cache, then repopulate from Postgres (source of truth)
    let _: () = conn.del(SANCTIONS_SET_KEY).await.unwrap_or(());

    let rows: Vec<(String,)> = sqlx::query_as("SELECT address FROM address_attributions")
        .fetch_all(db)
        .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let addresses: Vec<String> = rows.into_iter().map(|(a,)| a).collect();
    let count = addresses.len();

    let _: () = conn.sadd(SANCTIONS_SET_KEY, addresses).await?;

    Ok(count)
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    let redis_client = redis::Client::open(redis_url).expect("failed to create Redis client");

    let loaded = load_sanctions_into_redis(&pool, &redis_client)
        .await
        .expect("failed to warm Redis sanctions cache");
    println!("Loaded {} sanctioned addresses into Redis cache", loaded);

    let state = Arc::new(AppState {
        db: pool,
        redis: redis_client,
    });

    let app = Router::new()
        .route("/screen", post(screen_transaction))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await.unwrap();
    println!("Compliance engine listening on http://127.0.0.1:3001");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn screen_transaction(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<ScreenRequest>,
) -> Json<ScreenResponse> {
    let sender_lower = req.sender.to_lowercase();
    let recipient_lower = req.recipient.to_lowercase();

    let mut redis_conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let sender_hit: bool = redis_conn
        .sismember(SANCTIONS_SET_KEY, &sender_lower)
        .await
        .unwrap_or(false);

    let recipient_hit: bool = redis_conn
        .sismember(SANCTIONS_SET_KEY, &recipient_lower)
        .await
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let mut decision = "ALLOW".to_string();
    let mut risk_score = 0;

    if sender_hit {
        reasons.push("SANCTIONED_SENDER".to_string());
        decision = "BLOCK".to_string();
        risk_score = 98;
    }
    if recipient_hit {
        reasons.push("SANCTIONED_RECIPIENT".to_string());
        decision = "BLOCK".to_string();
        risk_score = 98;
    }

    let response = ScreenResponse {
        tx_hash: req.tx_hash.clone(),
        decision: decision.clone(),
        risk_score,
        reasons: reasons.clone(),
    };

    let _ = sqlx::query(
        "INSERT INTO compliance_decisions (tx_hash, sender, recipient, decision, risk_score, reason_codes, policy_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (tx_hash) DO NOTHING",
    )
    .bind(&req.tx_hash)
    .bind(&req.sender)
    .bind(&req.recipient)
    .bind(&decision)
    .bind(risk_score)
    .bind(&reasons)
    .bind("v1")
    .execute(&state.db)
    .await;

    Json(response)
}
