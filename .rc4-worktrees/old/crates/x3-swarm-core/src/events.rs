use crate::{AgentKind, TaskStatus};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Swarm events for logging and reactivity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SwarmEvent {
    TaskEnqueued { task_id: String, agent: AgentKind },
    TaskStarted { task_id: String },
    TaskCompleted { task_id: String, status: TaskStatus },
    ApprovalRequired { task_id: String },
    BlockerDetected { task_id: String, blocker: String },
    MemoryRecorded { entry_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimestampedEvent {
    pub event: SwarmEvent,
    pub timestamp: String,
}

impl TimestampedEvent {
    pub fn new(event: SwarmEvent) -> Self {
        Self {
            event,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Event bus (channel-based stub).
pub struct EventBus;

impl EventBus {
    pub fn emit(event: SwarmEvent) {
        tracing::info!("Event: {:?}", event);
    }

    pub fn subscribe() -> Vec<SwarmEvent> {
        vec![]
    }
}
