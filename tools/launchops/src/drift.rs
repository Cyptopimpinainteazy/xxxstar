//! Classify changed files into X3 categories using configured globs.

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::models::{AuditPathConfig, ChangedFileSet};

fn set(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(Glob::new(p)?);
    }
    Ok(b.build()?)
}

fn cross_vm_globs() -> Vec<String> {
    vec![
        "crates/**/cross-vm*/**/*.rs".into(),
        "crates/**/cross_vm*/**/*.rs".into(),
        "crates/**/x3-atomic*/**/*.rs".into(),
        "crates/**/atomic-swap*/**/*.rs".into(),
        "crates/**/cross-chain-coordinator/**/*.rs".into(),
    ]
}
fn dex_globs() -> Vec<String> {
    vec![
        "crates/**/x3-dex/**/*.rs".into(),
        "crates/**/x3-swap*/**/*.rs".into(),
    ]
}
fn gpu_globs() -> Vec<String> {
    vec![
        "crates/**/gpu*/**/*.rs".into(),
        "crates/**/x3-gpu*/**/*.rs".into(),
        "crates/**/cross-chain-gpu*/**/*.rs".into(),
    ]
}
fn ops_globs() -> Vec<String> {
    vec![
        ".github/**/*.yml".into(),
        ".github/**/*.yaml".into(),
        "scripts/**/*.sh".into(),
        "testnet/**".into(),
        "deploy/**".into(),
    ]
}

pub fn classify(paths: &AuditPathConfig, changed: &[String]) -> Result<ChangedFileSet> {
    let docs = set(&paths.docs)?;
    let code = set(&paths.production_code)?;
    let tests = set(&paths.tests)?;
    let consensus = set(&paths.consensus)?;
    let bridge = set(&paths.bridge)?;
    let mainnet_config = set(&paths.mainnet_config)?;
    let cross_vm = set(&cross_vm_globs())?;
    let dex = set(&dex_globs())?;
    let gpu = set(&gpu_globs())?;
    let ops = set(&ops_globs())?;

    let mut out = ChangedFileSet::default();
    for f in changed {
        let p = std::path::Path::new(f);
        if docs.is_match(p) {
            out.docs.push(f.clone());
        }
        if code.is_match(p) {
            out.code.push(f.clone());
        }
        if tests.is_match(p) {
            out.tests.push(f.clone());
        }
        if consensus.is_match(p) {
            out.consensus.push(f.clone());
        }
        if bridge.is_match(p) {
            out.bridge.push(f.clone());
        }
        if cross_vm.is_match(p) {
            out.cross_vm.push(f.clone());
        }
        if dex.is_match(p) {
            out.dex.push(f.clone());
        }
        if gpu.is_match(p) {
            out.gpu.push(f.clone());
        }
        if ops.is_match(p) {
            out.ops.push(f.clone());
        }
        if mainnet_config.is_match(p) {
            out.mainnet_config.push(f.clone());
        }
    }
    for v in [
        &mut out.docs,
        &mut out.code,
        &mut out.tests,
        &mut out.consensus,
        &mut out.bridge,
        &mut out.cross_vm,
        &mut out.dex,
        &mut out.gpu,
        &mut out.ops,
        &mut out.mainnet_config,
    ] {
        v.sort();
        v.dedup();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bridge_and_code() {
        let p = AuditPathConfig::default();
        let set = classify(
            &p,
            &[
                "crates/x3-bridge/src/lib.rs".into(),
                "docs/bridge.md".into(),
            ],
        )
        .unwrap();
        assert!(set
            .bridge
            .contains(&"crates/x3-bridge/src/lib.rs".to_string()));
        assert!(set
            .code
            .contains(&"crates/x3-bridge/src/lib.rs".to_string()));
        assert!(set.docs.contains(&"docs/bridge.md".to_string()));
    }
}
