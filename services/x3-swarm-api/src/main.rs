use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use x3_swarm_core::{
    AgentKind, AgentTask, ApprovalRequirement, RiskLevel, SwarmMemoryEntry, TaskStatus,
};

const MAX_MEMORY: usize = 1_000;
const MAX_EVENTS: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwarmEvent {
    id: String,
    event_type: String,
    message: String,
    timestamp: String,
}

#[derive(Debug, Clone)]
struct AppState {
    tasks: BTreeMap<String, AgentTask>,
    memory: VecDeque<SwarmMemoryEntry>,
    events: VecDeque<SwarmEvent>,
    kill_switch: bool,
    task_seq: u64,
    event_seq: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tasks: BTreeMap::new(),
            memory: VecDeque::new(),
            events: VecDeque::new(),
            kill_switch: false,
            task_seq: 7,
            event_seq: 1,
        }
    }
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut initial = AppState::default();
    for task in default_tasks() {
        initial.tasks.insert(task.id.clone(), task);
    }

    let state: SharedState = Arc::new(Mutex::new(initial));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/agents", get(agents))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id/start", post(start_task))
        .route("/tasks/:id/complete", post(complete_task))
        .route("/tasks/:id/fail", post(fail_task))
        .route("/tasks/:id/approve", post(approve_task))
        .route("/tasks/:id/reject", post(reject_task))
        .route("/scoreboard", get(scoreboard))
        .route("/memory", get(list_memory).post(create_memory))
        .route("/events", get(list_events))
        .route("/kill-switch", post(kill_switch))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:8787")
        .await
        .expect("bind x3-swarm-api listener");
    println!("x3-swarm-api listening on http://127.0.0.1:8787");
    axum::serve(listener, app).await.expect("serve x3-swarm-api");
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn emit_event(state: &mut AppState, event_type: &str, message: String) {
    let event = SwarmEvent {
        id: format!("evt_{:06}", state.event_seq),
        event_type: event_type.to_string(),
        message,
        timestamp: now(),
    };
    state.event_seq += 1;
    state.events.push_front(event);
    while state.events.len() > MAX_EVENTS {
        state.events.pop_back();
    }
}

fn default_tasks() -> Vec<AgentTask> {
    vec![
        AgentTask {
            id: "x3-task-0001".to_string(),
            title: "Add Atomic Router six-route tests".to_string(),
            feature: "atomic_router".to_string(),
            agent: AgentKind::TestBuilder,
            permission_tier: x3_swarm_core::AgentPermissionTier::DocsTestsReports,
            allowed_paths: vec!["tests/".to_string(), "reports/".to_string()],
            forbidden_paths: vec!["mainnet/".to_string(), "keys/".to_string()],
            required_commands: vec![],
            approval_required: ApprovalRequirement::HumanReview,
            status: TaskStatus::Pending,
            risk: RiskLevel::High,
        },
        AgentTask {
            id: "x3-task-0002".to_string(),
            title: "Add Tauri SwarmCommand screen updates".to_string(),
            feature: "tauri_os".to_string(),
            agent: AgentKind::Integrator,
            permission_tier: x3_swarm_core::AgentPermissionTier::TauriServiceWiring,
            allowed_paths: vec!["apps/tauri-os/".to_string(), "reports/".to_string()],
            forbidden_paths: vec!["mainnet/".to_string()],
            required_commands: vec![],
            approval_required: ApprovalRequirement::HumanReview,
            status: TaskStatus::Pending,
            risk: RiskLevel::Medium,
        },
    ]
}

async fn health(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    Json(json!({
        "service": "x3-swarm-api",
        "status": "ok",
        "mode": "GUARDED_TESTNET",
        "agents_enabled": true,
        "kill_switch": state.kill_switch
    }))
}

async fn status(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    Json(json!({
        "service": "x3-swarm-api",
        "status": "running",
        "mode": "GUARDED_TESTNET",
        "kill_switch": state.kill_switch,
        "tasks": state.tasks.len(),
        "memory_entries": state.memory.len(),
    }))
}

async fn agents() -> Json<Vec<AgentKind>> {
    Json(vec![
        AgentKind::RepoScanner,
        AgentKind::FeatureMapper,
        AgentKind::TestBuilder,
        AgentKind::Integrator,
        AgentKind::BuildFixer,
        AgentKind::WiringInspector,
        AgentKind::Auditor,
        AgentKind::Breaker,
        AgentKind::Fixer,
        AgentKind::ReadinessReporter,
        AgentKind::Benchmark,
        AgentKind::Marketing,
        AgentKind::Grant,
        AgentKind::ApprovalGate,
    ])
}

async fn list_tasks(State(state): State<SharedState>) -> Json<Vec<AgentTask>> {
    let state = state.lock().await;
    Json(state.tasks.values().cloned().collect())
}

#[derive(Debug, Deserialize)]
struct NewTaskRequest {
    title: String,
    feature: String,
    agent: AgentKind,
    permission_tier: x3_swarm_core::AgentPermissionTier,
    #[serde(default)]
    allowed_paths: Vec<String>,
    #[serde(default)]
    forbidden_paths: Vec<String>,
    #[serde(default)]
    required_commands: Vec<String>,
    approval_required: ApprovalRequirement,
    risk: String,
}

/// `AgentTask::risk` is a `RiskLevel`, and an unknown value is a client error
/// rather than something to guess at.
fn parse_risk(raw: &str) -> Result<RiskLevel, (StatusCode, String)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(RiskLevel::Low),
        "medium" => Ok(RiskLevel::Medium),
        "high" => Ok(RiskLevel::High),
        "critical" => Ok(RiskLevel::Critical),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unknown risk level {other:?}; expected low|medium|high|critical"),
        )),
    }
}

/// Append one JSON line to an agent-memory file.
///
/// `x3_swarm_core` used to export `append_memory_entry_file`; it no longer does
/// (the crate keeps only the in-memory helpers), so this service writes the
/// line itself: append-only, one JSON object per line, creating the directory on
/// first write.
fn append_memory_entry_file(path: &str, entry: &SwarmMemoryEntry) -> std::io::Result<()> {
    use std::io::Write;

    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(entry)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

async fn create_task(
    State(state): State<SharedState>,
    Json(req): Json<NewTaskRequest>,
) -> Result<(StatusCode, Json<AgentTask>), (StatusCode, String)> {
    let mut state = state.lock().await;
    let task_id = format!("x3-task-{:04}", state.task_seq);
    state.task_seq += 1;

    // Reject an unknown risk before any state is touched.
    let risk = parse_risk(&req.risk)?;
    let task = AgentTask {
        id: task_id,
        title: req.title,
        feature: req.feature,
        agent: req.agent,
        permission_tier: req.permission_tier,
        allowed_paths: req.allowed_paths,
        forbidden_paths: req.forbidden_paths,
        required_commands: req.required_commands,
        approval_required: req.approval_required,
        status: TaskStatus::Pending,
        risk,
    };

    emit_event(&mut state, "task_created", format!("{} created", task.id));
    state.tasks.insert(task.id.clone(), task.clone());
    Ok((StatusCode::CREATED, Json(task)))
}

async fn get_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let state = state.lock().await;
    state
        .tasks
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

fn update_status(state: &mut AppState, id: &str, status: TaskStatus) -> Result<AgentTask, StatusCode> {
    let Some(task) = state.tasks.get_mut(id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    task.status = status;
    let out = task.clone();
    emit_event(state, "task_status", format!("{} -> {:?}", id, out.status));
    Ok(out)
}

async fn start_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let mut state = state.lock().await;
    if state.kill_switch {
        return Err(StatusCode::LOCKED);
    }
    update_status(&mut state, &id, TaskStatus::Running).map(Json)
}

async fn complete_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let mut state = state.lock().await;
    update_status(&mut state, &id, TaskStatus::Passed).map(Json)
}

async fn fail_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let mut state = state.lock().await;
    update_status(&mut state, &id, TaskStatus::Failed).map(Json)
}

async fn approve_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let mut state = state.lock().await;
    update_status(&mut state, &id, TaskStatus::Pending).map(Json)
}

async fn reject_task(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<AgentTask>, StatusCode> {
    let mut state = state.lock().await;
    update_status(&mut state, &id, TaskStatus::Blocked).map(Json)
}

async fn scoreboard(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    let total = state.tasks.len() as f64;
    let passed = state
        .tasks
        .values()
        .filter(|task| task.status == TaskStatus::Passed)
        .count() as f64;
    let failed = state
        .tasks
        .values()
        .filter(|task| task.status == TaskStatus::Failed)
        .count();

    let success_rate = if total > 0.0 { passed / total } else { 0.0 };
    Json(json!({
        "service": "x3-swarm-api",
        "tasks_total": state.tasks.len(),
        "tasks_passed": passed as usize,
        "tasks_failed": failed,
        "success_rate": success_rate,
        "kill_switch": state.kill_switch,
    }))
}

async fn list_memory(State(state): State<SharedState>) -> Json<Vec<SwarmMemoryEntry>> {
    let state = state.lock().await;
    Json(state.memory.iter().cloned().collect())
}

async fn create_memory(
    State(state): State<SharedState>,
    Json(mut entry): Json<SwarmMemoryEntry>,
) -> Result<(StatusCode, Json<SwarmMemoryEntry>), (StatusCode, String)> {
    let mut state = state.lock().await;
    if entry.id.trim().is_empty() {
        entry.id = format!("mem_{:06}", state.memory.len() + 1);
    }
    if entry.timestamp.trim().is_empty() {
        entry.timestamp = now();
    }

    let file_path = match entry.agent {
        AgentKind::TestBuilder => "data/agent-memory/builder.jsonl",
        AgentKind::Auditor => "data/agent-memory/auditor.jsonl",
        AgentKind::Breaker => "data/agent-memory/breaker.jsonl",
        AgentKind::Fixer => "data/agent-memory/fixer.jsonl",
        AgentKind::Benchmark => "data/agent-memory/benchmark.jsonl",
        AgentKind::Marketing => "data/agent-memory/marketing.jsonl",
        AgentKind::Grant => "data/agent-memory/grants.jsonl",
        _ => "data/agent-memory/lessons.jsonl",
    };

    append_memory_entry_file(file_path, &entry)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    state.memory.push_front(entry.clone());
    while state.memory.len() > MAX_MEMORY {
        state.memory.pop_back();
    }
    emit_event(&mut state, "memory_recorded", format!("{} recorded", entry.id));
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn list_events(State(state): State<SharedState>) -> Json<Vec<SwarmEvent>> {
    let state = state.lock().await;
    Json(state.events.iter().cloned().collect())
}

#[derive(Debug, Deserialize)]
struct KillSwitchRequest {
    enabled: bool,
    reason: Option<String>,
}

async fn kill_switch(
    State(state): State<SharedState>,
    Json(req): Json<KillSwitchRequest>,
) -> Json<serde_json::Value> {
    let mut state = state.lock().await;
    state.kill_switch = req.enabled;
    // Build the message before taking the mutable borrow for `emit_event`:
    // reading `state.kill_switch` inside the call is what made this a
    // borrow-check error.
    let enabled = state.kill_switch;
    let reason = req
        .reason
        .unwrap_or_else(|| "no reason provided".to_string());
    emit_event(
        &mut state,
        "kill_switch",
        format!("kill switch set to {enabled} ({reason})"),
    );
    Json(json!({ "kill_switch": state.kill_switch }))
}
