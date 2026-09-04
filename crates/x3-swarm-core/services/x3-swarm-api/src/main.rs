use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::Mutex;

const MAX_MEMORY_ENTRIES: usize = 1_000;
const MAX_EVENT_ENTRIES: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum AgentStatus {
    Online,
    Offline,
    Busy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum TaskStatus {
    Pending,
    Approved,
    Running,
    Passed,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ApprovalMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum PermissionTier {
    #[serde(rename = "read-only", alias = "ReadOnly")]
    ReadOnly,
    #[serde(rename = "constrained", alias = "Constrained")]
    Constrained,
    DocsTestsReports,
    TauriServiceWiring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentRecord {
    id: String,
    kind: String,
    status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRecord {
    id: String,
    title: String,
    feature: String,
    agent: String,
    permission_tier: PermissionTier,
    allowed_paths: Vec<String>,
    forbidden_paths: Vec<String>,
    required_commands: Vec<String>,
    status: TaskStatus,
    approval_required: ApprovalMode,
    risk: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryRecord {
    id: String,
    agent: String,
    feature: String,
    finding: String,
    severity: String,
    test_added: Option<String>,
    fix_commit: Option<String>,
    result: String,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventRecord {
    id: String,
    event_type: String,
    message: String,
    timestamp: String,
}

type AppState = Arc<Mutex<StateData>>;

struct StateData {
    agents: HashMap<String, AgentRecord>,
    tasks: HashMap<String, TaskRecord>,
    memory: VecDeque<MemoryRecord>,
    events: VecDeque<EventRecord>,
    kill_switch: bool,
    next_task_id: u64,
    next_memory_id: u64,
    next_event_id: u64,
}

impl Default for StateData {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            tasks: HashMap::new(),
            memory: VecDeque::new(),
            events: VecDeque::new(),
            kill_switch: false,
            next_task_id: 1,
            next_memory_id: 1,
            next_event_id: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NewTask {
    title: String,
    feature: String,
    agent: String,
    permission_tier: PermissionTier,
    allowed_paths: Option<Vec<String>>,
    forbidden_paths: Option<Vec<String>>,
    required_commands: Option<Vec<String>>,
    approval_required: ApprovalMode,
    risk: RiskLevel,
}

#[derive(Debug, Deserialize)]
struct MemoryPayload {
    agent: String,
    feature: String,
    finding: String,
    severity: String,
    test_added: Option<String>,
    fix_commit: Option<String>,
    result: String,
}

#[derive(Debug, Deserialize)]
struct KillSwitchPayload {
    confirm: String,
    actor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventPayload {
    event_type: String,
    message: String,
}

#[tokio::main]
async fn main() {
    let state: AppState = Arc::new(Mutex::new(StateData::default()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/agents", get(list_agents))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id/start", post(start_task))
        .route("/tasks/:id/complete", post(complete_task))
        .route("/tasks/:id/fail", post(fail_task))
        .route("/tasks/:id/approve", post(approve_task))
        .route("/tasks/:id/reject", post(reject_task))
        .route("/scoreboard", get(scoreboard))
        .route("/memory", get(list_memory).post(create_memory))
        .route("/events", get(list_events).post(create_event))
        .route("/kill-switch", post(kill_switch))
        .route("/kill-switch/disengage", post(kill_switch_disengage))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    println!("Starting x3-swarm-api on http://{}", addr);
    if let Err(error) = axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        eprintln!("failed to bind/serve x3-swarm-api on {}: {}", addr, error);
        std::process::exit(1);
    }
}

async fn health(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    Json(serde_json::json!({
        "service": "x3-swarm-api",
        "status": "ok",
        "mode": "GUARDED_TESTNET",
        "agents_enabled": true,
        "kill_switch": state.kill_switch,
    }))
}

async fn status(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    Json(serde_json::json!({
        "service": "x3-swarm-api",
        "status": "ok",
        "tasks": state.tasks.len(),
        "agents": state.agents.len(),
        "kill_switch": state.kill_switch,
    }))
}

async fn list_agents(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    let agents: Vec<_> = state.agents.values().cloned().collect();
    Json(agents)
}

async fn list_tasks(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    let tasks: Vec<_> = state.tasks.values().cloned().collect();
    Json(tasks)
}

#[axum::debug_handler]
async fn create_task(
    state: State<AppState>,
    Json(payload): Json<NewTask>,
) -> Result<Json<TaskRecord>, (StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(existing) = state.tasks.values().find(|task| {
        task.title == payload.title
            && task.feature == payload.feature
            && task.agent == payload.agent
    }) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"duplicate task", "task": existing})),
        ));
    }

    let id = format!("x3-task-{:04}", state.next_task_id);
    state.next_task_id += 1;
    let task = TaskRecord {
        id,
        title: payload.title,
        feature: payload.feature,
        agent: payload.agent,
        permission_tier: payload.permission_tier,
        allowed_paths: payload.allowed_paths.unwrap_or_default(),
        forbidden_paths: payload.forbidden_paths.unwrap_or_default(),
        required_commands: payload.required_commands.unwrap_or_default(),
        status: TaskStatus::Pending,
        approval_required: payload.approval_required,
        risk: payload.risk,
    };
    state.tasks.insert(task.id.clone(), task.clone());
    Ok(Json(task))
}

async fn get_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let state = state.lock().await;
    if let Some(task) = state.tasks.get(&id) {
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn start_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(task) = state.tasks.get_mut(&id) {
        if !matches!(task.status, TaskStatus::Pending | TaskStatus::Approved) {
            return Err((
                StatusCode::CONFLICT,
                Json(
                    serde_json::json!({"error":"invalid transition", "from": task.status, "to":"Running"}),
                ),
            ));
        }
        task.status = TaskStatus::Running;
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn complete_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(task) = state.tasks.get_mut(&id) {
        if task.status != TaskStatus::Running {
            return Err((
                StatusCode::CONFLICT,
                Json(
                    serde_json::json!({"error":"invalid transition", "from": task.status, "to":"Passed"}),
                ),
            ));
        }
        task.status = TaskStatus::Passed;
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn fail_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(task) = state.tasks.get_mut(&id) {
        if task.status != TaskStatus::Running {
            return Err((
                StatusCode::CONFLICT,
                Json(
                    serde_json::json!({"error":"invalid transition", "from": task.status, "to":"Failed"}),
                ),
            ));
        }
        task.status = TaskStatus::Failed;
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn approve_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(task) = state.tasks.get_mut(&id) {
        if task.approval_required != ApprovalMode::Manual {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"approval is only allowed for manual tasks"})),
            ));
        }
        task.status = TaskStatus::Approved;
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn reject_task(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Result<Json<TaskRecord>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut state = state.lock().await;
    if let Some(task) = state.tasks.get_mut(&id) {
        task.status = TaskStatus::Rejected;
        Ok(Json(task.clone()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ))
    }
}

async fn scoreboard(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    let total_tasks = state.tasks.len();
    let successful_tasks = state
        .tasks
        .values()
        .filter(|task| task.status == TaskStatus::Passed)
        .count();
    let success_rate = if total_tasks == 0 {
        0.0
    } else {
        successful_tasks as f64 / total_tasks as f64
    };
    Json(serde_json::json!({
        "service": "x3-swarm-api",
        "total_tasks": total_tasks,
        "success_rate": success_rate,
    }))
}

async fn list_memory(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    Json(state.memory.iter().cloned().collect::<Vec<_>>())
}

#[axum::debug_handler]
async fn create_memory(
    state: State<AppState>,
    Json(payload): Json<MemoryPayload>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    let id = format!("x3-memory-{:04}", state.next_memory_id);
    state.next_memory_id += 1;
    let entry = MemoryRecord {
        id,
        agent: payload.agent,
        feature: payload.feature,
        finding: payload.finding,
        severity: payload.severity,
        test_added: payload.test_added,
        fix_commit: payload.fix_commit,
        result: payload.result,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    state.memory.push_back(entry.clone());
    while state.memory.len() > MAX_MEMORY_ENTRIES {
        state.memory.pop_front();
    }
    Json(entry)
}

async fn list_events(state: State<AppState>) -> impl IntoResponse {
    let state = state.lock().await;
    Json(state.events.iter().cloned().collect::<Vec<_>>())
}

#[axum::debug_handler]
async fn create_event(
    state: State<AppState>,
    Json(payload): Json<EventPayload>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    let id = format!("x3-event-{:04}", state.next_event_id);
    state.next_event_id += 1;
    let event = EventRecord {
        id,
        event_type: payload.event_type,
        message: payload.message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    state.events.push_back(event.clone());
    while state.events.len() > MAX_EVENT_ENTRIES {
        state.events.pop_front();
    }
    Json(event)
}

fn authorize_admin(headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let expected = std::env::var("X3_SWARM_ADMIN_TOKEN").map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error":"X3_SWARM_ADMIN_TOKEN is not configured"})),
        )
    })?;
    let provided = headers
        .get("x-x3-swarm-token")
        .and_then(|value| value.to_str().ok());
    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"invalid swarm admin token"})),
        ))
    }
}

async fn kill_switch(
    headers: HeaderMap,
    state: State<AppState>,
    Json(payload): Json<KillSwitchPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    authorize_admin(&headers)?;
    if payload.confirm != "ENGAGE" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"confirm must be ENGAGE"})),
        ));
    }
    let mut state = state.lock().await;
    state.kill_switch = true;
    let actor = payload.actor.unwrap_or_else(|| "unknown".to_string());
    Ok(Json(
        serde_json::json!({"status": "kill switch engaged", "actor": actor}),
    ))
}

async fn kill_switch_disengage(
    headers: HeaderMap,
    state: State<AppState>,
    Json(payload): Json<KillSwitchPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    authorize_admin(&headers)?;
    if payload.confirm != "DISENGAGE" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"confirm must be DISENGAGE"})),
        ));
    }
    let mut state = state.lock().await;
    state.kill_switch = false;
    let actor = payload.actor.unwrap_or_else(|| "unknown".to_string());
    Ok(Json(
        serde_json::json!({"status": "kill switch disengaged", "actor": actor}),
    ))
}
