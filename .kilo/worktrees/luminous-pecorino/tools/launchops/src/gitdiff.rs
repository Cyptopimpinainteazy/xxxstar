//! Thin wrapper around git for diffing against a baseline branch.

use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub struct GitDiff {
    pub baseline: String,
    pub changed_files: Vec<String>,
    pub diff_text: String,
    #[allow(dead_code)]
    pub warning: Option<String>,
}

fn git(root: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git").current_dir(root).args(args).output()
}

pub fn diff_against(root: &Path, baseline: &str) -> Result<GitDiff> {
    // If this is not a git repo, short-circuit with an empty diff + warning.
    let is_repo = git(root, &["rev-parse", "--is-inside-work-tree"])
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !is_repo {
        return Ok(GitDiff {
            baseline: baseline.to_string(),
            changed_files: vec![],
            diff_text: String::new(),
            warning: Some("not a git repository; drift detection skipped".into()),
        });
    }

    // Try `baseline...HEAD` first; if it fails (branch missing) fall back to `baseline..HEAD`
    // and finally to plain HEAD (diff working tree vs HEAD).
    let specs = [
        format!("{baseline}...HEAD"),
        format!("{baseline}..HEAD"),
        "HEAD".to_string(),
    ];

    for spec in &specs {
        let names = git(root, &["diff", "--name-only", spec]);
        if let Ok(o) = names {
            if o.status.success() {
                let files: Vec<String> = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect();
                let diff = git(root, &["diff", spec])
                    .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                    .unwrap_or_default();
                return Ok(GitDiff {
                    baseline: baseline.to_string(),
                    changed_files: files,
                    diff_text: diff,
                    warning: if spec == "HEAD" {
                        Some(format!(
                            "baseline `{baseline}` not reachable; diffed working tree vs HEAD"
                        ))
                    } else {
                        None
                    },
                });
            }
        }
    }

    Ok(GitDiff {
        baseline: baseline.to_string(),
        changed_files: vec![],
        diff_text: String::new(),
        warning: Some(format!("git diff against `{baseline}` failed")),
    })
}
