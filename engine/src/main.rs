use axum::{routing::post, Json, Router};
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
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    let state = Arc::new(AppState { db: pool });

    let app = Router::new()
        .route("/screen", post(screen_transaction))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await.unwrap();
    println!("Compliance engine listening on http://127.0.0.1:3001");
    axum::serve(listener, app).await.unwrap();
}

async fn screen_transaction(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<ScreenRequest>,
) -> Json<ScreenResponse> {
    let sender_lower = req.sender.to_lowercase();
    let recipient_lower = req.recipient.to_lowercase();

    let sender_hit: Option<(String,)> = sqlx::query_as(
        "SELECT address FROM address_attributions WHERE address = $1",
    )
    .bind(&sender_lower)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let recipient_hit: Option<(String,)> = sqlx::query_as(
        "SELECT address FROM address_attributions WHERE address = $1",
    )
    .bind(&recipient_lower)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let mut reasons = Vec::new();
    let mut decision = "ALLOW".to_string();
    let mut risk_score = 0;

    if sender_hit.is_some() {
        reasons.push("SANCTIONED_SENDER".to_string());
        decision = "BLOCK".to_string();
        risk_score = 98;
    }
    if recipient_hit.is_some() {
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
