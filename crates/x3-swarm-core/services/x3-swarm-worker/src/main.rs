use anyhow::Context;
use serde::Deserialize;
use std::{env, path::PathBuf, time::Duration};
use tokio::process::Command;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:8787";
const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Maximum wall-clock seconds a single agent command may run before SIGKILL.
const COMMAND_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum TaskStatus {
    Pending,
    Approved,
    Running,
    Passed,
    Failed,
    Rejected,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ApprovalRequired {
    Auto,
    Manual,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
enum PermissionTier {
    #[serde(rename = "read-only", alias = "ReadOnly")]
    ReadOnly,
    #[serde(rename = "constrained", alias = "Constrained")]
    Constrained,
    DocsTestsReports,
    TauriServiceWiring,
}

#[derive(Debug, Deserialize)]
struct TaskRecord {
    id: String,
    title: String,
    feature: String,
    agent: String,
    permission_tier: PermissionTier,
    status: TaskStatus,
    approval_required: ApprovalRequired,
    risk: RiskLevel,
    required_commands: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("failed to build swarm worker HTTP client")?;
    let api_base_url =
        env::var("X3_SWARM_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string());
    println!("Starting x3-swarm-worker...");

    loop {
        let tasks = match fetch_tasks(&client, &api_base_url).await {
            Ok(tasks) => tasks,
            Err(error) => {
                eprintln!("failed to fetch swarm tasks: {error:#}");
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        let pending: Vec<_> = tasks
            .into_iter()
            .filter(|task| {
                if task.status == TaskStatus::Approved {
                    return true;
                }
                if task.status == TaskStatus::Pending
                    && task.approval_required != ApprovalRequired::Manual
                {
                    return true;
                }
                false
            })
            .collect();

        if pending.is_empty() {
            println!("No pending swarm tasks available, sleeping...");
        } else {
            println!("Found {} pending task(s).", pending.len());
            for task in pending {
                if let Err(error) = process_task(&client, &api_base_url, task).await {
                    eprintln!("failed to process swarm task: {error:#}");
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn fetch_tasks(
    client: &reqwest::Client,
    api_base_url: &str,
) -> anyhow::Result<Vec<TaskRecord>> {
    let url = format!("{}/tasks", api_base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .send()
        .await
        .context("failed to request tasks from swarm API")?
        .error_for_status()?;
    let tasks = response.json::<Vec<TaskRecord>>().await?;
    Ok(tasks)
}

async fn process_task(
    client: &reqwest::Client,
    api_base_url: &str,
    task: TaskRecord,
) -> anyhow::Result<()> {
    println!(
        "Starting task {}: {} (agent={})",
        task.id, task.title, task.agent
    );
    post_task_action(client, api_base_url, &task.id, "start").await?;

    let outcome = dispatch_agent(&task).await;

    match outcome {
        Ok(()) => {
            let completed = post_task_action(client, api_base_url, &task.id, "complete").await?;
            println!(
                "Task {} completed with status {:?}",
                completed.id, completed.status
            );
        }
        Err(ref err) => {
            eprintln!("Task {} failed: {err:#}", task.id);
            post_task_action(client, api_base_url, &task.id, "fail").await?;
        }
    }
    outcome
}

/// Resolve the wrapper script for a named agent, if one exists.
///
/// Convention: `scripts/swarm/<agent_name>.sh` is the agent entry-point.
/// If that file is absent the agent falls back to executing `required_commands`
/// directly (for agents that have no dedicated wrapper yet).
fn agent_script(agent_name: &str, swarm_scripts_dir: &PathBuf) -> Option<PathBuf> {
    let path = swarm_scripts_dir.join(format!("{agent_name}.sh"));
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Reject commands that exceed the task's permission tier.
///
/// Returns `Err` if a command would violate the declared tier so the task is
/// failed before any subprocess is launched — preventing silent privilege
/// escalation.
fn check_permission(cmd: &str, tier: &PermissionTier) -> anyhow::Result<()> {
    // Patterns whose presence in a command string indicates a potentially
    // destructive or write-heavy operation.
    const WRITE_PATTERNS: &[&str] = &[
        "rm ",
        "rm\t",
        "rm\n",
        "> /",
        ">> /",
        "dd ",
        "mkfs",
        "fdisk",
        "parted",
        "chmod 777",
        "sudo ",
        "curl -X POST",
        "curl -X PUT",
        "curl -X DELETE",
    ];
    if matches!(tier, PermissionTier::ReadOnly) {
        for pat in WRITE_PATTERNS {
            if cmd.contains(pat) {
                anyhow::bail!(
                    "command '{cmd}' violates ReadOnly permission tier (matches '{pat}')"
                );
            }
        }
    }
    Ok(())
}

/// Run a single shell command under `bash -c` with a hard timeout.
async fn run_command(cmd: &str) -> anyhow::Result<()> {
    let mut child = Command::new("bash")
        .args(["-c", cmd])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn command: {cmd}"))?;

    let result =
        tokio::time::timeout(Duration::from_secs(COMMAND_TIMEOUT_SECS), child.wait()).await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => anyhow::bail!("command exited with {status}: {cmd}"),
        Ok(Err(err)) => Err(err).with_context(|| format!("command wait error: {cmd}")),
        Err(_) => {
            // Timed out — kill the child so it doesn't linger.
            let _ = child.kill().await;
            anyhow::bail!("command timed out after {COMMAND_TIMEOUT_SECS}s: {cmd}")
        }
    }
}

/// Core dispatch: runs the agent wrapper script (if present) then any
/// `required_commands` declared on the task.
async fn dispatch_agent(task: &TaskRecord) -> anyhow::Result<()> {
    // Critical-risk tasks require an explicit opt-in env flag to prevent
    // accidental execution in environments where a human hasn't reviewed them.
    if task.risk == RiskLevel::Critical {
        let allowed = env::var("SWARM_ALLOW_CRITICAL")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !allowed {
            anyhow::bail!(
                "task {} has risk=Critical; set SWARM_ALLOW_CRITICAL=1 to permit",
                task.id
            );
        }
    }

    // Locate the swarm scripts directory relative to the binary's working dir.
    let swarm_scripts_dir = {
        let mut p = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        p.push("scripts/swarm");
        p
    };

    // --- 1. Optional agent wrapper script ---
    if let Some(script) = agent_script(&task.agent, &swarm_scripts_dir) {
        let script_str = script.to_string_lossy();
        println!("[{}] running agent wrapper: {script_str}", task.agent);
        let cmd = format!(
            "TASK_ID={} TASK_TITLE={} FEATURE={} {}",
            shlex_escape(&task.id),
            shlex_escape(&task.title),
            shlex_escape(&task.feature),
            script_str,
        );
        check_permission(&cmd, &task.permission_tier)?;
        run_command(&cmd)
            .await
            .with_context(|| format!("agent wrapper script failed for task {}", task.id))?;
    }

    // --- 2. required_commands ---
    let commands = task.required_commands.as_deref().unwrap_or(&[]);
    if commands.is_empty() && agent_script(&task.agent, &swarm_scripts_dir).is_none() {
        println!(
            "[{}] no commands and no agent script — task is a no-op",
            task.agent
        );
        return Ok(());
    }

    for cmd in commands {
        println!("[{}] $ {cmd}", task.agent);
        check_permission(cmd, &task.permission_tier)?;
        run_command(cmd)
            .await
            .with_context(|| format!("required_command failed for task {}: {cmd}", task.id))?;
    }

    Ok(())
}

/// Minimal shell-safe quoting: wrap in single quotes, escape embedded `'`.
fn shlex_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

async fn post_task_action(
    client: &reqwest::Client,
    api_base_url: &str,
    task_id: &str,
    action: &str,
) -> anyhow::Result<TaskRecord> {
    let url = format!(
        "{}/tasks/{}/{}",
        api_base_url.trim_end_matches('/'),
        task_id,
        action
    );
    let response = client
        .post(&url)
        .send()
        .await
        .with_context(|| format!("failed to post {} for task {}", action, task_id))?
        .error_for_status()?;

    let task = response.json::<TaskRecord>().await?;
    Ok(task)
}
