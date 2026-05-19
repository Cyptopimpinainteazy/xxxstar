//! Stale doc detector — compares doc mtime vs. linked code evidence mtime.

use std::path::Path;
use std::time::SystemTime;

use crate::models::{DriftSeverity, FeatureMatrixItem, StaleDoc};

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

pub fn detect(root: &Path, items: &[FeatureMatrixItem], stale_doc_days: u64) -> Vec<StaleDoc> {
    let mut out = Vec::new();
    let stale_window = std::time::Duration::from_secs(stale_doc_days.saturating_mul(24 * 60 * 60));

    for it in items {
        if it.code_evidence.is_empty() {
            continue;
        }
        let doc_path = root.join(&it.source_file);
        let doc_mtime = match mtime(&doc_path) {
            Some(m) => m,
            None => continue,
        };
        let mut newer: Vec<String> = Vec::new();
        for c in &it.code_evidence {
            let cpath = root.join(c);
            if let Some(cm) = mtime(&cpath) {
                if cm > doc_mtime {
                    if let Ok(diff) = cm.duration_since(doc_mtime) {
                        if diff > stale_window {
                            newer.push(c.clone());
                        }
                    }
                }
            }
        }
        if !newer.is_empty() {
            out.push(StaleDoc {
                file: it.source_file.clone(),
                linked_code: newer,
                severity: DriftSeverity::Medium,
                reason: format!("Doc older than linked code by more than {stale_doc_days} days"),
            });
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out.dedup_by(|a, b| a.file == b.file);
    out
}
