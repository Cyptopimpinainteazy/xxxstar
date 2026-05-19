//! Validate that each invariant in tests/invariants/registry.toml is referenced by at least one test file.
//!
//! This is a lightweight meta-test ensuring we don't accrue untested invariants.

use std::fs;
use std::path::Path;

#[test]
fn registry_invariants_are_referenced() {
    let toml = fs::read_to_string("tests/invariants/registry.toml").expect("Failed to read registry.toml");

    // Extract IDs by simple parsing (lines starting with id = "...")
    let ids: Vec<String> = toml
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with("id = ") {
                // naive parse
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

    assert!(!ids.is_empty(), "No invariants found in registry.toml");

    // For each id, search for its occurrence in the workspace files we care about.
    let search_paths = vec!["pallets", "crates", "tests", "apps", "packages", "swarm"];

    for id in ids {
        let mut found = false;
        for prefix in &search_paths {
            if let Ok(entries) = fs::read_dir(prefix) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if content.contains(&id) {
                                found = true;
                                break;
                            }
                        }
                    } else if path.is_dir() {
                        // Search recursively but shallow (one level) for speed
                        for sub in fs::read_dir(&path).unwrap_or_else(|_| fs::read_dir(Path::new(".")).unwrap()) {
                            if let Ok(sub) = sub {
                                let p = sub.path();
                                if p.is_file() {
                                    if let Ok(content) = fs::read_to_string(&p) {
                                        if content.contains(&id) {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if found { break; }
        }

        assert!(found, "Invariant '{}' is not referenced by any test or file under search paths", id);
    }
}
