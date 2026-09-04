//! Command runner — executes configured shell commands and captures
//! status, exit code, duration, and trimmed stdout/stderr excerpts.

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::models::{CommandResult, CommandStatus};

const EXCERPT_BYTES: usize = 4096;

fn excerpt(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let trimmed = s.trim();
    if trimmed.len() <= EXCERPT_BYTES {
        trimmed.to_string()
    } else {
        let tail_start = trimmed.len() - EXCERPT_BYTES;
        // Make sure we don't slice inside a UTF-8 codepoint.
        let mut safe = tail_start;
        while safe < trimmed.len() && !trimmed.is_char_boundary(safe) {
            safe += 1;
        }
        format!("...[truncated]...\n{}", &trimmed[safe..])
    }
}

fn tool_name(cmd: &str) -> Option<&str> {
    cmd.split_whitespace().next()
}

fn tool_available(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn run_commands(
    root: &Path,
    commands: &BTreeMap<String, String>,
) -> Result<Vec<CommandResult>> {
    let mut out = Vec::new();
    for (name, cmd) in commands {
        let tool = tool_name(cmd).unwrap_or("");
        // For cargo_* commands the tool is "cargo"; for cargo_audit/cargo_deny the subcommand may be
        // missing even if cargo exists. We still run and let cargo report; `cargo audit` w/o plugin
        // exits non-zero which we capture as Failed (not MissingTool) — but if top-level tool is
        // absent we short-circuit with MissingTool.
        if !tool.is_empty() && !tool_available(tool) {
            out.push(CommandResult {
                name: name.clone(),
                command: cmd.clone(),
                status: CommandStatus::MissingTool,
                exit_code: None,
                duration_ms: 0,
                stdout_excerpt: String::new(),
                stderr_excerpt: format!("tool `{tool}` not found in PATH"),
            });
            continue;
        }
        let start = Instant::now();
        let result = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(root)
            .output();
        let duration_ms = start.elapsed().as_millis();
        match result {
            Ok(o) => {
                let code = o.status.code();
                let status = if o.status.success() {
                    CommandStatus::Passed
                } else {
                    CommandStatus::Failed
                };
                out.push(CommandResult {
                    name: name.clone(),
                    command: cmd.clone(),
                    status,
                    exit_code: code,
                    duration_ms,
                    stdout_excerpt: excerpt(&o.stdout),
                    stderr_excerpt: excerpt(&o.stderr),
                });
            }
            Err(e) => {
                out.push(CommandResult {
                    name: name.clone(),
                    command: cmd.clone(),
                    status: CommandStatus::Failed,
                    exit_code: None,
                    duration_ms,
                    stdout_excerpt: String::new(),
                    stderr_excerpt: format!("failed to spawn: {e}"),
                });
            }
        }
    }
    Ok(out)
}
