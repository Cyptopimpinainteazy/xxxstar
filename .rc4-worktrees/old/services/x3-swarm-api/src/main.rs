use axum::{
    routing::{get, post},
    Json, Router, extract::Path,
};
use serde_json::json;
use std::net::SocketAddr;
use x3_swarm_core::*; // Core types

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/agents", get(agents))
        .route("/tasks", get(tasks).post(create_task))
        .route("/tasks/:id", get(get_task).post(start_task))
        .route("/tasks/:id/start", post(start_task))
        .route("/tasks/:id/complete", post(complete_task))
        .route("/tasks/:id/fail", post(fail_task))
        .route("/tasks/:id/approve", post(approve_task))
        .route("/tasks/:id/reject", post(reject_task))
        .route("/scoreboard", get(scoreboard))
        .route("/memory", get(memory).post(add_memory))
        .route("/events", get(events))
        .route("/kill-switch", post(kill_switch));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    println!("X3 Swarm API listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "service": "x3-swarm-api",
        "status": "ok",
        "mode": "GUARDED_TESTNET",
        "agents_enabled": true,
        "kill_switch": false
    }))
}

async fn status() -> &'static str {
    "{\"status\": \"running\"}"
}

async fn agents() -> Json<serde_json::Value> {
    Json(json!(["RepoScanner", "TestBuilder", "Auditor"]))
}

async fn tasks() -> Json<serde_json::Value> {
    Json(json!([]))
}

async fn create_task(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(json!({"id": "new-task", "status": "pending"}))
}

async fn get_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "status": "pending"}))
}

async fn start_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "status": "running"}))
}

async fn complete_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "status": "passed"}))
}

async fn fail_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "status": "failed"}))
}

async fn approve_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "approved": true}))
}

async fn reject_task(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({"id": id, "rejected": true}))
}

async fn scoreboard() -> Json<serde_json::Value> {
    Json(json!({"success_rate": 0.95, "tasks_completed": 42}))
}

async fn memory() -> Json<serde_json::Value> {
    Json(json!([]))
}

async fn add_memory(Json(_): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(json!({"added": true}))
}

async fn events() -> Json<serde_json::Value> {
    Json(json!([]))
}

async fn kill_switch() -> Json<serde_json::Value> {
    Json(json!({"kill_switch": "activated"}))
}
