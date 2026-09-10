use std::fs;
use std::path::Path;

#[cfg(test)]
mod golden_fixtures {
    use super::*;

    /// Generate golden JSON files for all fixture programs.
    /// Run this test to update the golden files when the AST changes.
    ///
    /// #[ignore] is required: this test overwrites the checked-in golden
    /// fixture files as a side effect. Without it, `cargo test` runs this
    /// concurrently with `test_golden_fixtures` (both are `#[test]` in the
    /// same binary and run in parallel by default), creating a data race
    /// where this test can truncate/rewrite a golden file while the other
    /// test is reading it, causing spurious failures. Run explicitly with
    /// `cargo test -p x3-parser --test golden -- --ignored` to regenerate.
    #[test]
    #[ignore]
    fn generate_golden_fixtures() {
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

        for entry in fs::read_dir(&fixture_dir).expect("Failed to read fixture directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("x3") {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let json_path = fixture_dir.join(format!("{}.json", stem));

                println!("Processing fixture: {}", stem);

                // Read the source code
                let source = fs::read_to_string(&path).expect("Failed to read fixture file");

                // Parse the module
                let module = x3_parser::parse_program(&source).expect("Failed to parse fixture");

                // Serialize to canonical JSON
                let json =
                    serde_json::to_string_pretty(&module).expect("Failed to serialize AST to JSON");

                // Write the golden file
                fs::write(&json_path, json).expect("Failed to write golden file");

                println!("Generated golden file: {}", json_path.display());
            }
        }
    }

    /// Test that parsing fixtures produces the expected AST JSON.
    #[test]
    fn test_golden_fixtures() {
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

        for entry in fs::read_dir(&fixture_dir).expect("Failed to read fixture directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("x3") {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let json_path = fixture_dir.join(format!("{}.json", stem));

                // Read the source code
                let source = fs::read_to_string(&path).expect("Failed to read fixture file");

                // Read the expected JSON
                let expected_json =
                    fs::read_to_string(&json_path).expect("Failed to read golden file");

                // Parse the module
                let module = x3_parser::parse_program(&source).expect("Failed to parse fixture");

                // Serialize to JSON
                let actual_json =
                    serde_json::to_string_pretty(&module).expect("Failed to serialize AST to JSON");

                // Compare
                assert_eq!(
                    actual_json, expected_json,
                    "Fixture {} failed golden test",
                    stem
                );
            }
        }
    }
}
