use std::collections::BTreeSet;
use std::fs;

#[test]
fn registry_invariants_are_referenced() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest)
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let path = workspace_root.join("tests/invariants/registry.toml");
    let toml = fs::read_to_string(&path).expect(&format!(
        "Failed to read registry.toml at {}",
        path.display()
    ));
    let ids: BTreeSet<String> = toml
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with("id = ") {
                let rest = l.trim_start_matches("id = ").trim();
                if rest.starts_with('"') && rest.ends_with('"') {
                    Some(rest.trim_matches('"').to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    assert!(!ids.is_empty());

    let mut unresolved = ids.clone();
    let root = std::path::Path::new(&workspace_root);
    for entry in walkdir::WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "target" | ".git" | "node_modules" | ".next" | "dist" | "build"
            )
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if unresolved.is_empty() {
            break;
        }

        let p = entry.path();
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            // only check common source file types to speed up the search
            match ext {
                "rs" | "toml" | "py" | "ts" | "tsx" | "md" | "yaml" | "yml" | "json" => {}
                _ => continue,
            }
        } else {
            continue;
        }

        if let Ok(content) = fs::read_to_string(p) {
            unresolved.retain(|id| !content.contains(id));
        }
    }

    assert!(
        unresolved.is_empty(),
        "Invariants not referenced by any file in workspace: {:?}",
        unresolved
    );
}
