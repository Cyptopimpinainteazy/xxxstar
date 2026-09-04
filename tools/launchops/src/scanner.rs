//! Markdown scanner — walks configured include globs, applies excludes,
//! returns a list of markdown files to parse.

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::ScanConfig;

pub struct ScanInputs {
    pub files: Vec<PathBuf>,
}

fn build_set(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let g = Glob::new(p).with_context(|| format!("invalid glob: {p}"))?;
        b.add(g);
    }
    b.build().context("failed building globset")
}

pub fn scan_markdown(root: &Path, cfg: &ScanConfig) -> Result<ScanInputs> {
    let include = build_set(&cfg.include)?;
    let exclude = build_set(&cfg.exclude)?;

    let mut files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // skip target/, .git/, node_modules/ early
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "target" | ".git" | "node_modules" | "dist" | "build"
            )
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        // Only markdown
        if rel.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if exclude.is_match(rel) {
            continue;
        }
        if !include.is_match(rel) {
            continue;
        }
        files.push(entry.path().to_path_buf());
    }

    files.sort();
    Ok(ScanInputs { files })
}

pub fn collect_by_globs(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let set = build_set(patterns)?;
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "target" | ".git" | "node_modules" | "dist" | "build"
            )
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if set.is_match(rel) {
            out.push(entry.path().to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}
