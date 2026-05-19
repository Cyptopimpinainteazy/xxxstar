//! Task spec — .md file parser with YAML front-matter.
//!
//! Each task is a markdown file with YAML front-matter containing:
//! ```yaml
//! ---
//! id: <unique-task-id>
//! priority: <high|medium|low>
//! section: <Strings|Brass|Percussion|Woodwinds>
//! proposer: <agent-id>
//! timestamp: <ISO-8601>
//! task-type: <law|execution|simulation>
//! ---
//! ```
//! Followed by a markdown body with description, reasoning, constraints,
//! and optional simulation outputs for jury review.

use crate::agent::identity::OrchestraSection;
use crate::score::TaskClassification;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Task type — determines the execution pathway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    /// Law proposal — changes to system rules (always Major → requires jury).
    Law,
    /// Execution — computational task (Minor or Major based on impact).
    Execution,
    /// Simulation — test scenario with no side effects.
    Simulation,
}

impl TaskType {
    /// Classify a task type for Score enforcement.
    pub fn classification(&self) -> TaskClassification {
        match self {
            TaskType::Law => TaskClassification::Major,       // Laws always need jury
            TaskType::Execution => TaskClassification::Minor,  // Default; caller may override
            TaskType::Simulation => TaskClassification::Minor, // Sims are always minor
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "law" => Ok(TaskType::Law),
            "execution" => Ok(TaskType::Execution),
            "simulation" => Ok(TaskType::Simulation),
            other => Err(format!("Unknown task type: {}", other)),
        }
    }
}

/// Priority level for task scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl std::str::FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(TaskPriority::Low),
            "medium" => Ok(TaskPriority::Medium),
            "high" => Ok(TaskPriority::High),
            other => Err(format!("Unknown priority: {}", other)),
        }
    }
}

/// YAML front-matter metadata from a task .md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Unique task identifier.
    pub id: String,
    /// Priority level.
    pub priority: TaskPriority,
    /// Orchestra section this task belongs to.
    pub section: OrchestraSection,
    /// Agent ID of the proposer.
    pub proposer: u32,
    /// Timestamp of task creation.
    pub timestamp: DateTime<Utc>,
    /// Type of task.
    #[serde(rename = "task-type")]
    pub task_type: TaskType,
}

/// A complete task specification parsed from a .md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Parsed YAML metadata.
    pub metadata: TaskMetadata,
    /// Markdown body — description, reasoning, constraints.
    pub body: String,
    /// Optional simulation output (for jury review).
    pub simulation_output: Option<String>,
    /// Source file path (if loaded from disk).
    pub source_path: Option<String>,
    /// Whether this task has been approved for execution.
    pub approved: bool,
    /// Task classification (derived from task_type, may be overridden).
    pub classification: TaskClassification,
}

/// Intermediate struct for YAML front-matter deserialization.
#[derive(Debug, Deserialize)]
struct RawFrontMatter {
    id: String,
    priority: String,
    section: String,
    proposer: u32,
    timestamp: String,
    #[serde(rename = "task-type")]
    task_type: String,
}

impl TaskSpec {
    /// Parse a task spec from a markdown string with YAML front-matter.
    pub fn parse(content: &str) -> Result<Self, TaskParseError> {
        // Split front-matter from body
        let content = content.trim();
        if !content.starts_with("---") {
            return Err(TaskParseError::MissingFrontMatter);
        }

        let after_first = &content[3..];
        let end_idx = after_first
            .find("---")
            .ok_or(TaskParseError::MalformedFrontMatter(
                "No closing --- found".into(),
            ))?;

        let yaml_str = &after_first[..end_idx].trim();
        let body = after_first[end_idx + 3..].trim().to_string();

        // Parse YAML front-matter
        let raw: RawFrontMatter = serde_yaml::from_str(yaml_str).map_err(|e| {
            TaskParseError::MalformedFrontMatter(format!("YAML parse error: {}", e))
        })?;

        // Parse individual fields
        let priority: TaskPriority = raw
            .priority
            .parse()
            .map_err(|e: String| TaskParseError::InvalidField("priority".into(), e))?;

        let section = match raw.section.to_lowercase().as_str() {
            "strings" => OrchestraSection::Strings,
            "brass" => OrchestraSection::Brass,
            "percussion" => OrchestraSection::Percussion,
            "woodwinds" => OrchestraSection::Woodwinds,
            other => {
                return Err(TaskParseError::InvalidField(
                    "section".into(),
                    format!("Unknown section: {}", other),
                ))
            }
        };

        let task_type: TaskType = raw
            .task_type
            .parse()
            .map_err(|e: String| TaskParseError::InvalidField("task-type".into(), e))?;

        let timestamp = DateTime::parse_from_rfc3339(&raw.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                // Try ISO-8601 without timezone
                chrono::NaiveDateTime::parse_from_str(&raw.timestamp, "%Y-%m-%dT%H:%M:%S")
                    .map(|ndt| ndt.and_utc())
            })
            .map_err(|e| {
                TaskParseError::InvalidField("timestamp".into(), format!("Invalid timestamp: {}", e))
            })?;

        let classification = task_type.classification();

        // Extract simulation output if present (delimited by ## Simulation Output)
        let simulation_output = if body.contains("## Simulation Output") {
            let idx = body.find("## Simulation Output").unwrap();
            Some(body[idx..].to_string())
        } else {
            None
        };

        Ok(TaskSpec {
            metadata: TaskMetadata {
                id: raw.id,
                priority,
                section,
                proposer: raw.proposer,
                timestamp,
                task_type,
            },
            body,
            simulation_output,
            source_path: None,
            approved: false,
            classification,
        })
    }

    /// Load a task spec from a .md file on disk.
    pub fn load_from_file(path: &Path) -> Result<Self, TaskParseError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            TaskParseError::IoError(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let mut spec = Self::parse(&content)?;
        spec.source_path = Some(path.to_string_lossy().to_string());
        Ok(spec)
    }

    /// Serialize this task spec back to a .md string with YAML front-matter.
    pub fn to_markdown(&self) -> String {
        let section_str = match self.metadata.section {
            OrchestraSection::Strings => "Strings",
            OrchestraSection::Brass => "Brass",
            OrchestraSection::Percussion => "Percussion",
            OrchestraSection::Woodwinds => "Woodwinds",
        };

        let task_type_str = match self.metadata.task_type {
            TaskType::Law => "law",
            TaskType::Execution => "execution",
            TaskType::Simulation => "simulation",
        };

        let priority_str = match self.metadata.priority {
            TaskPriority::Low => "low",
            TaskPriority::Medium => "medium",
            TaskPriority::High => "high",
        };

        format!(
            "---\nid: {}\npriority: {}\nsection: {}\nproposer: {}\ntimestamp: {}\ntask-type: {}\n---\n\n{}",
            self.metadata.id,
            priority_str,
            section_str,
            self.metadata.proposer,
            self.metadata.timestamp.to_rfc3339(),
            task_type_str,
            self.body,
        )
    }

    /// Whether this task requires jury approval.
    pub fn requires_jury(&self) -> bool {
        matches!(self.classification, TaskClassification::Major)
    }

    /// Override classification (e.g., escalate an execution task to Major).
    pub fn escalate_to_major(&mut self) {
        self.classification = TaskClassification::Major;
    }
}

/// Errors encountered during task spec parsing.
#[derive(Debug, thiserror::Error)]
pub enum TaskParseError {
    #[error("Missing YAML front-matter (must start with ---)")]
    MissingFrontMatter,
    #[error("Malformed front-matter: {0}")]
    MalformedFrontMatter(String),
    #[error("Invalid field '{0}': {1}")]
    InvalidField(String, String),
    #[error("I/O error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TASK: &str = r#"---
id: task-001
priority: high
section: Strings
proposer: 42
timestamp: 2026-02-07T12:00:00Z
task-type: execution
---

# Deploy Updated Smart Contract

## Description
Deploy the updated lending pool contract with new interest rate model.

## Reasoning
Current interest rate model doesn't account for market volatility.

## Constraints
- Must maintain backwards compatibility
- Cannot exceed 500k gas limit
- Must pass all e2e tests before deployment
"#;

    #[test]
    fn parse_task_spec() {
        let spec = TaskSpec::parse(SAMPLE_TASK).unwrap();
        assert_eq!(spec.metadata.id, "task-001");
        assert_eq!(spec.metadata.priority, TaskPriority::High);
        assert_eq!(spec.metadata.section, OrchestraSection::Strings);
        assert_eq!(spec.metadata.proposer, 42);
        assert_eq!(spec.metadata.task_type, TaskType::Execution);
        assert!(!spec.requires_jury()); // execution is Minor by default
        assert!(spec.body.contains("Deploy Updated Smart Contract"));
    }

    #[test]
    fn law_tasks_require_jury() {
        let md = r#"---
id: law-001
priority: high
section: Brass
proposer: 1
timestamp: 2026-02-07T12:00:00Z
task-type: law
---
Propose new fee structure.
"#;
        let spec = TaskSpec::parse(md).unwrap();
        assert!(spec.requires_jury());
    }

    #[test]
    fn escalate_execution_to_major() {
        let mut spec = TaskSpec::parse(SAMPLE_TASK).unwrap();
        assert!(!spec.requires_jury());
        spec.escalate_to_major();
        assert!(spec.requires_jury());
    }

    #[test]
    fn round_trip_markdown() {
        let spec = TaskSpec::parse(SAMPLE_TASK).unwrap();
        let md = spec.to_markdown();
        let reparsed = TaskSpec::parse(&md).unwrap();
        assert_eq!(reparsed.metadata.id, spec.metadata.id);
        assert_eq!(reparsed.metadata.priority, spec.metadata.priority);
    }

    #[test]
    fn missing_frontmatter_rejected() {
        let result = TaskSpec::parse("no front matter here");
        assert!(result.is_err());
    }
}
