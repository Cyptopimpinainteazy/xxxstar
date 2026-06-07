//! Task queue — manages .md task specs, priority ordering, filesystem watching.

use super::spec::{TaskParseError, TaskPriority, TaskSpec};
use crate::score::TaskClassification;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

/// Status of a task in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Queued and waiting for approval/execution.
    Pending,
    /// Approved by core agents (minor) or jury (major).
    Approved,
    /// Currently being executed by an on-chain agent.
    Executing,
    /// Staged for jury review (major tasks).
    AwaitingJury,
    /// Jury rejected this task.
    Rejected,
    /// Execution completed successfully.
    Completed,
    /// Execution failed.
    Failed,
    /// Human deleted the .md file — task cancelled.
    Cancelled,
}

/// A task entry in the queue with tracking metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// The parsed task specification.
    pub spec: TaskSpec,
    /// Current status.
    pub status: TaskStatus,
    /// Assigned agent ID (if executing).
    pub assigned_to: Option<u32>,
    /// When this entry was added to the queue.
    pub queued_at: DateTime<Utc>,
    /// When status last changed.
    pub status_changed_at: DateTime<Utc>,
    /// Jury session ID (if awaiting/completed jury review).
    pub jury_session: Option<String>,
    /// Execution result summary (if completed or failed).
    pub result_summary: Option<String>,
}

impl QueueEntry {
    pub fn new(spec: TaskSpec) -> Self {
        let now = Utc::now();
        Self {
            spec,
            status: TaskStatus::Pending,
            assigned_to: None,
            queued_at: now,
            status_changed_at: now,
            jury_session: None,
            result_summary: None,
        }
    }

    fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.status_changed_at = Utc::now();
    }
}

/// The task queue — manages all pending, executing, and completed tasks.
pub struct TaskQueue {
    /// All tasks indexed by ID.
    entries: BTreeMap<String, QueueEntry>,
    /// Tasks directory on disk (for .md file watching).
    tasks_dir: PathBuf,
}

impl TaskQueue {
    /// Create a new task queue that watches a directory.
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self {
            entries: BTreeMap::new(),
            tasks_dir,
        }
    }

    /// Scan the tasks directory and load all .md files.
    pub fn scan_directory(&mut self) -> Result<Vec<String>, TaskParseError> {
        let mut loaded = Vec::new();

        if !self.tasks_dir.exists() {
            std::fs::create_dir_all(&self.tasks_dir).map_err(|e| {
                TaskParseError::IoError(format!(
                    "Failed to create tasks dir {}: {}",
                    self.tasks_dir.display(),
                    e
                ))
            })?;
            return Ok(loaded);
        }

        let pattern = format!("{}/**/*.md", self.tasks_dir.display());
        for entry in glob::glob(&pattern).map_err(|e| {
            TaskParseError::IoError(format!("Glob error: {}", e))
        })? {
            match entry {
                Ok(path) => match TaskSpec::load_from_file(&path) {
                    Ok(spec) => {
                        let id = spec.metadata.id.clone();
                        if !self.entries.contains_key(&id) {
                            self.entries.insert(id.clone(), QueueEntry::new(spec));
                            loaded.push(id);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {}", path.display(), e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Glob entry error: {}", e);
                }
            }
        }

        Ok(loaded)
    }

    /// Add a task to the queue programmatically.
    pub fn enqueue(&mut self, spec: TaskSpec) -> String {
        let id = spec.metadata.id.clone();
        self.entries.insert(id.clone(), QueueEntry::new(spec));
        id
    }

    /// Get a task by ID.
    pub fn get(&self, task_id: &str) -> Option<&QueueEntry> {
        self.entries.get(task_id)
    }

    /// Get a mutable reference to a task.
    pub fn get_mut(&mut self, task_id: &str) -> Option<&mut QueueEntry> {
        self.entries.get_mut(task_id)
    }

    /// Get all pending tasks, sorted by priority (high first).
    pub fn pending_tasks(&self) -> Vec<&QueueEntry> {
        let mut tasks: Vec<&QueueEntry> = self
            .entries
            .values()
            .filter(|e| e.status == TaskStatus::Pending)
            .collect();

        tasks.sort_by(|a, b| b.spec.metadata.priority.cmp(&a.spec.metadata.priority));
        tasks
    }

    /// Get next task for a given section.
    pub fn next_for_section(
        &self,
        section: crate::agent::identity::OrchestraSection,
    ) -> Option<&QueueEntry> {
        self.pending_tasks()
            .into_iter()
            .find(|e| e.spec.metadata.section == section)
    }

    /// Get all tasks awaiting jury review.
    pub fn jury_pending_tasks(&self) -> Vec<&QueueEntry> {
        self.entries
            .values()
            .filter(|e| e.status == TaskStatus::AwaitingJury)
            .collect()
    }

    /// Approve a task (core agent approves minor tasks).
    pub fn approve(&mut self, task_id: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::Pending {
                entry.update_status(TaskStatus::Approved);
                entry.spec.approved = true;
                return true;
            }
        }
        false
    }

    /// Stage a task for jury review (major tasks).
    pub fn stage_for_jury(&mut self, task_id: &str, session_id: String) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::Pending {
                entry.update_status(TaskStatus::AwaitingJury);
                entry.jury_session = Some(session_id);
                return true;
            }
        }
        false
    }

    /// Jury approves a task (majority vote passed).
    pub fn jury_approve(&mut self, task_id: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::AwaitingJury {
                entry.update_status(TaskStatus::Approved);
                entry.spec.approved = true;
                return true;
            }
        }
        false
    }

    /// Jury rejects a task.
    pub fn jury_reject(&mut self, task_id: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::AwaitingJury {
                entry.update_status(TaskStatus::Rejected);
                return true;
            }
        }
        false
    }

    /// Assign a task to an agent for execution.
    pub fn assign(&mut self, task_id: &str, agent_id: u32) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::Approved {
                entry.update_status(TaskStatus::Executing);
                entry.assigned_to = Some(agent_id);
                return true;
            }
        }
        false
    }

    /// Mark a task as completed.
    pub fn complete(&mut self, task_id: &str, summary: String) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::Executing {
                entry.update_status(TaskStatus::Completed);
                entry.result_summary = Some(summary);
                return true;
            }
        }
        false
    }

    /// Mark a task as failed.
    pub fn fail(&mut self, task_id: &str, reason: String) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            if entry.status == TaskStatus::Executing {
                entry.update_status(TaskStatus::Failed);
                entry.result_summary = Some(reason);
                return true;
            }
        }
        false
    }

    /// Cancel a task (human .md deletion).
    pub fn cancel(&mut self, task_id: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(task_id) {
            entry.update_status(TaskStatus::Cancelled);
            return true;
        }
        false
    }

    /// Check for .md files that have been deleted (human intervention).
    pub fn check_deleted_files(&mut self) -> Vec<String> {
        let mut cancelled = Vec::new();
        for (id, entry) in self.entries.iter_mut() {
            if let Some(ref path) = entry.spec.source_path {
                if !Path::new(path).exists()
                    && entry.status != TaskStatus::Completed
                    && entry.status != TaskStatus::Cancelled
                    && entry.status != TaskStatus::Failed
                {
                    entry.update_status(TaskStatus::Cancelled);
                    cancelled.push(id.clone());
                }
            }
        }
        cancelled
    }

    /// Save a task spec as a .md file in the tasks directory.
    pub fn write_task_file(&self, spec: &TaskSpec) -> Result<PathBuf, std::io::Error> {
        let filename = format!("{}.md", spec.metadata.id);
        let path = self.tasks_dir.join(&filename);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, spec.to_markdown())?;
        Ok(path)
    }

    /// Get queue statistics.
    pub fn stats(&self) -> QueueStats {
        let mut stats = QueueStats::default();
        for entry in self.entries.values() {
            stats.total += 1;
            match entry.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Approved => stats.approved += 1,
                TaskStatus::Executing => stats.executing += 1,
                TaskStatus::AwaitingJury => stats.awaiting_jury += 1,
                TaskStatus::Rejected => stats.rejected += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed => stats.failed += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }
        }
        stats
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Queue statistics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub total: usize,
    pub pending: usize,
    pub approved: usize,
    pub executing: usize,
    pub awaiting_jury: usize,
    pub rejected: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::OrchestraSection;
    use crate::task::spec::{TaskMetadata, TaskPriority, TaskType};
    use chrono::Utc;

    fn make_spec(id: &str, priority: TaskPriority, task_type: TaskType) -> TaskSpec {
        TaskSpec {
            metadata: TaskMetadata {
                id: id.into(),
                priority,
                section: OrchestraSection::Strings,
                proposer: 1,
                timestamp: Utc::now(),
                task_type,
            },
            body: "Test task body".into(),
            simulation_output: None,
            source_path: None,
            approved: false,
            classification: task_type.classification(),
        }
    }

    #[test]
    fn enqueue_and_retrieve() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = TaskQueue::new(dir.path().to_path_buf());

        let spec = make_spec("t1", TaskPriority::High, TaskType::Execution);
        queue.enqueue(spec);

        assert_eq!(queue.len(), 1);
        assert!(queue.get("t1").is_some());
    }

    #[test]
    fn priority_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = TaskQueue::new(dir.path().to_path_buf());

        queue.enqueue(make_spec("low", TaskPriority::Low, TaskType::Execution));
        queue.enqueue(make_spec("high", TaskPriority::High, TaskType::Execution));
        queue.enqueue(make_spec("med", TaskPriority::Medium, TaskType::Execution));

        let pending = queue.pending_tasks();
        assert_eq!(pending[0].spec.metadata.id, "high");
        assert_eq!(pending[1].spec.metadata.id, "med");
        assert_eq!(pending[2].spec.metadata.id, "low");
    }

    #[test]
    fn full_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = TaskQueue::new(dir.path().to_path_buf());

        queue.enqueue(make_spec("t1", TaskPriority::High, TaskType::Execution));

        // Approve → Assign → Complete
        assert!(queue.approve("t1"));
        assert!(queue.assign("t1", 42));
        assert!(queue.complete("t1", "All checks passed".into()));

        let entry = queue.get("t1").unwrap();
        assert_eq!(entry.status, TaskStatus::Completed);
        assert_eq!(entry.assigned_to, Some(42));
    }

    #[test]
    fn jury_flow() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = TaskQueue::new(dir.path().to_path_buf());

        queue.enqueue(make_spec("law-1", TaskPriority::High, TaskType::Law));

        // Law tasks should be staged for jury
        assert!(queue.stage_for_jury("law-1", "session-001".into()));
        assert_eq!(queue.jury_pending_tasks().len(), 1);

        // Jury approves
        assert!(queue.jury_approve("law-1"));
        let entry = queue.get("law-1").unwrap();
        assert_eq!(entry.status, TaskStatus::Approved);
    }

    #[test]
    fn queue_stats() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = TaskQueue::new(dir.path().to_path_buf());

        queue.enqueue(make_spec("t1", TaskPriority::High, TaskType::Execution));
        queue.enqueue(make_spec("t2", TaskPriority::Low, TaskType::Simulation));
        queue.approve("t1");

        let stats = queue.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.approved, 1);
        assert_eq!(stats.pending, 1);
    }
}
