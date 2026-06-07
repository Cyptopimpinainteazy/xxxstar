use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::models::{ArtifactFieldDoc, ArtifactManifest, ArtifactSchemaDoc, ArtifactSchemaIndex};

const OUT_DIR: &str = ".launchops";

pub fn validate_emitted_contract(root: &Path) -> Result<usize> {
    let out_dir = root.join(OUT_DIR);
    let manifest_path = out_dir.join("artifact_manifest.json");
    let schema_path = out_dir.join("artifact_schemas.json");

    let manifest: ArtifactManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?,
    )
    .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let schema_index: ArtifactSchemaIndex = serde_json::from_str(
        &std::fs::read_to_string(&schema_path)
            .with_context(|| format!("reading {}", schema_path.display()))?,
    )
    .with_context(|| format!("parsing {}", schema_path.display()))?;

    let schema_by_kind: BTreeMap<&str, &ArtifactSchemaDoc> = schema_index
        .artifacts
        .iter()
        .map(|doc| (doc.kind.as_str(), doc))
        .collect();

    let mut validated = 0usize;
    let mut seen_files = BTreeSet::new();
    for artifact in &manifest.artifacts {
        let path = out_dir.join(&artifact.file);
        if !path.exists() {
            bail!("artifact listed in manifest is missing: {}", artifact.file);
        }
        if !seen_files.insert(artifact.file.as_str()) {
            bail!("duplicate artifact in manifest: {}", artifact.file);
        }

        if artifact.kind == "artifact_schemas" {
            validated += 1;
            continue;
        }

        let schema_ref = artifact
            .schema_ref
            .as_deref()
            .with_context(|| format!("artifact {} is missing schema_ref", artifact.file))?;
        validate_schema_ref(schema_ref, &artifact.file, &artifact.kind)?;

        let schema = schema_by_kind
            .get(artifact.kind.as_str())
            .copied()
            .with_context(|| {
                format!(
                    "artifact {} references missing schema kind {}",
                    artifact.file, artifact.kind
                )
            })?;

        if schema.file != artifact.file {
            bail!(
                "artifact {} schema file mismatch: schema expects {}",
                artifact.file,
                schema.file
            );
        }
        if schema.format != artifact.format {
            bail!(
                "artifact {} format mismatch: manifest={}, schema={}",
                artifact.file,
                artifact.format,
                schema.format
            );
        }

        match artifact.format.as_str() {
            "json" => validate_json_artifact(&path, schema)
                .with_context(|| format!("validating {}", artifact.file))?,
            "markdown" => {
                let body = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                if body.trim().is_empty() {
                    bail!("markdown artifact is empty: {}", artifact.file);
                }
            }
            other => bail!(
                "unsupported artifact format {} for {}",
                other,
                artifact.file
            ),
        }

        validated += 1;
    }

    Ok(validated)
}

fn validate_schema_ref(schema_ref: &str, file: &str, kind: &str) -> Result<()> {
    let Some((schema_file, schema_kind)) = schema_ref.split_once('#') else {
        bail!("artifact {} has invalid schema_ref {}", file, schema_ref);
    };
    if schema_file != "artifact_schemas.json" {
        bail!(
            "artifact {} has unexpected schema file {} in schema_ref {}",
            file,
            schema_file,
            schema_ref
        );
    }
    if schema_kind != kind {
        bail!(
            "artifact {} schema_ref kind mismatch: expected {}, got {}",
            file,
            kind,
            schema_kind
        );
    }
    Ok(())
}

fn validate_json_artifact(path: &Path, schema: &ArtifactSchemaDoc) -> Result<()> {
    let value: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;

    match schema.shape.as_str() {
        "object" => {
            let obj = value
                .as_object()
                .with_context(|| format!("expected top-level object in {}", path.display()))?;
            validate_object_fields(obj, &schema.fields, path, "$")?;
        }
        "array<object>" => {
            let arr = value
                .as_array()
                .with_context(|| format!("expected top-level array in {}", path.display()))?;
            for (idx, item) in arr.iter().enumerate() {
                let obj = item
                    .as_object()
                    .with_context(|| format!("expected object at {}[{}]", path.display(), idx))?;
                validate_object_fields(obj, &schema.fields, path, &format!("$[{}]", idx))?;
            }
        }
        other => bail!("unsupported schema shape {} for {}", other, schema.file),
    }
    Ok(())
}

fn validate_object_fields(
    obj: &serde_json::Map<String, Value>,
    fields: &[ArtifactFieldDoc],
    path: &Path,
    location: &str,
) -> Result<()> {
    let expected: BTreeSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();

    for field in fields {
        let Some(value) = obj.get(&field.name) else {
            if field.required {
                bail!("missing required field {}.{}", location, field.name);
            }
            continue;
        };
        validate_field_value(value, field, path, &format!("{}.{}", location, field.name))?;
    }

    for key in obj.keys() {
        if !expected.contains(key.as_str()) {
            bail!(
                "undocumented field {}.{} in {}",
                location,
                key,
                path.display()
            );
        }
    }

    Ok(())
}

fn validate_field_value(
    value: &Value,
    field: &ArtifactFieldDoc,
    path: &Path,
    location: &str,
) -> Result<()> {
    match field.type_name.as_str() {
        "string" => {
            if !value.is_string() {
                bail!("expected string at {} in {}", location, path.display());
            }
        }
        "string(datetime)" => {
            let raw = value.as_str().with_context(|| {
                format!(
                    "expected datetime string at {} in {}",
                    location,
                    path.display()
                )
            })?;
            chrono::DateTime::parse_from_rfc3339(raw).with_context(|| {
                format!(
                    "invalid RFC3339 datetime at {} in {}",
                    location,
                    path.display()
                )
            })?;
        }
        "string|null" => {
            if !(value.is_string() || value.is_null()) {
                bail!("expected string|null at {} in {}", location, path.display());
            }
        }
        "integer" => {
            if value.as_i64().is_none() && value.as_u64().is_none() {
                bail!("expected integer at {} in {}", location, path.display());
            }
        }
        "integer|null" => {
            if !(value.is_null() || value.as_i64().is_some() || value.as_u64().is_some()) {
                bail!(
                    "expected integer|null at {} in {}",
                    location,
                    path.display()
                );
            }
        }
        "number" => {
            if !value.is_number() {
                bail!("expected number at {} in {}", location, path.display());
            }
        }
        "boolean" => {
            if !value.is_boolean() {
                bail!("expected boolean at {} in {}", location, path.display());
            }
        }
        "array<string>" => {
            let arr = value.as_array().with_context(|| {
                format!(
                    "expected array<string> at {} in {}",
                    location,
                    path.display()
                )
            })?;
            if arr.iter().any(|item| !item.is_string()) {
                bail!(
                    "expected only strings in {} in {}",
                    location,
                    path.display()
                );
            }
        }
        "array<object>" => {
            let arr = value.as_array().with_context(|| {
                format!(
                    "expected array<object> at {} in {}",
                    location,
                    path.display()
                )
            })?;
            if arr.iter().any(|item| !item.is_object()) {
                bail!(
                    "expected only objects in {} in {}",
                    location,
                    path.display()
                );
            }
            if !field.nested_fields.is_empty() {
                for (idx, item) in arr.iter().enumerate() {
                    let obj = item.as_object().with_context(|| {
                        format!(
                            "expected object at {}[{}] in {}",
                            location,
                            idx,
                            path.display()
                        )
                    })?;
                    validate_object_fields(
                        obj,
                        &field.nested_fields,
                        path,
                        &format!("{}[{}]", location, idx),
                    )?;
                }
            }
        }
        "object" => {
            let obj = value.as_object().with_context(|| {
                format!("expected object at {} in {}", location, path.display())
            })?;
            if !field.nested_fields.is_empty() {
                validate_object_fields(obj, &field.nested_fields, path, location)?;
            }
        }
        "object<string,number>" => {
            let obj = value.as_object().with_context(|| {
                format!(
                    "expected object<string,number> at {} in {}",
                    location,
                    path.display()
                )
            })?;
            if obj.values().any(|item| !item.is_number()) {
                bail!(
                    "expected only numeric values in {} in {}",
                    location,
                    path.display()
                );
            }
        }
        other => bail!(
            "unsupported field type {} at {} in {}",
            other,
            location,
            path.display()
        ),
    }

    if !field.enum_values.is_empty() {
        let raw = value.as_str().with_context(|| {
            format!("expected enum string at {} in {}", location, path.display())
        })?;
        if !field.enum_values.iter().any(|candidate| candidate == raw) {
            bail!(
                "unexpected enum value {:?} at {} in {}; expected one of {:?}",
                raw,
                location,
                path.display(),
                field.enum_values
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ArtifactDescriptor, ArtifactFieldDoc, ArtifactManifest, ArtifactSchemaDoc,
        ArtifactSchemaIndex,
    };

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("launchops-contract-{}-{}", name, nanos));
        std::fs::create_dir_all(path.join(OUT_DIR)).expect("temp dir");
        path
    }

    fn write_json_file(path: &Path, value: &impl serde::Serialize) {
        std::fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    fn sample_manifest() -> ArtifactManifest {
        ArtifactManifest {
            schema_version: "launchops-artifacts/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![
                ArtifactDescriptor {
                    file: "artifact_schemas.json".to_string(),
                    kind: "artifact_schemas".to_string(),
                    format: "json".to_string(),
                    schema_version: "launchops-artifact-fields/v1".to_string(),
                    schema_ref: None,
                },
                ArtifactDescriptor {
                    file: "readiness.json".to_string(),
                    kind: "readiness".to_string(),
                    format: "json".to_string(),
                    schema_version: "launchops-artifacts/v1".to_string(),
                    schema_ref: Some("artifact_schemas.json#readiness".to_string()),
                },
            ],
        }
    }

    fn sample_schema() -> ArtifactSchemaIndex {
        ArtifactSchemaIndex {
            schema_version: "launchops-artifact-fields/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![ArtifactSchemaDoc {
                file: "readiness.json".to_string(),
                kind: "readiness".to_string(),
                format: "json".to_string(),
                shape: "object".to_string(),
                fields: vec![
                    ArtifactFieldDoc {
                        name: "status".to_string(),
                        type_name: "string".to_string(),
                        required: true,
                        description: "status".to_string(),
                        enum_values: vec!["BLOCKED".to_string(), "NOT_BLOCKED".to_string()],
                        nested_fields: Vec::new(),
                    },
                    ArtifactFieldDoc {
                        name: "totals".to_string(),
                        type_name: "object".to_string(),
                        required: true,
                        description: "totals".to_string(),
                        enum_values: Vec::new(),
                        nested_fields: vec![ArtifactFieldDoc {
                            name: "total_md_files".to_string(),
                            type_name: "integer".to_string(),
                            required: true,
                            description: "count".to_string(),
                            enum_values: Vec::new(),
                            nested_fields: Vec::new(),
                        }],
                    },
                ],
            }],
        }
    }

    fn markdown_manifest() -> ArtifactManifest {
        ArtifactManifest {
            schema_version: "launchops-artifacts/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![
                ArtifactDescriptor {
                    file: "artifact_schemas.json".to_string(),
                    kind: "artifact_schemas".to_string(),
                    format: "json".to_string(),
                    schema_version: "launchops-artifact-fields/v1".to_string(),
                    schema_ref: None,
                },
                ArtifactDescriptor {
                    file: "report.md".to_string(),
                    kind: "scan_report".to_string(),
                    format: "markdown".to_string(),
                    schema_version: "launchops-artifacts/v1".to_string(),
                    schema_ref: Some("artifact_schemas.json#scan_report".to_string()),
                },
            ],
        }
    }

    fn markdown_schema() -> ArtifactSchemaIndex {
        ArtifactSchemaIndex {
            schema_version: "launchops-artifact-fields/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![ArtifactSchemaDoc {
                file: "report.md".to_string(),
                kind: "scan_report".to_string(),
                format: "markdown".to_string(),
                shape: "text/markdown".to_string(),
                fields: vec![],
            }],
        }
    }

    #[test]
    fn validates_nested_object_fields() {
        let root = temp_dir("ok");
        let out = root.join(OUT_DIR);
        write_json_file(&out.join("artifact_manifest.json"), &sample_manifest());
        write_json_file(&out.join("artifact_schemas.json"), &sample_schema());
        write_json_file(
            &out.join("readiness.json"),
            &serde_json::json!({
                "status": "BLOCKED",
                "totals": { "total_md_files": 12 }
            }),
        );

        let validated = validate_emitted_contract(&root).unwrap();
        assert_eq!(validated, 2);
    }

    #[test]
    fn rejects_undocumented_fields() {
        let root = temp_dir("extra");
        let out = root.join(OUT_DIR);
        write_json_file(&out.join("artifact_manifest.json"), &sample_manifest());
        write_json_file(&out.join("artifact_schemas.json"), &sample_schema());
        write_json_file(
            &out.join("readiness.json"),
            &serde_json::json!({
                "status": "BLOCKED",
                "totals": { "total_md_files": 12 },
                "extra": true
            }),
        );

        let err = format!("{:#}", validate_emitted_contract(&root).unwrap_err());
        assert!(err.contains("undocumented field $.extra"));
    }

    #[test]
    fn validates_nested_array_object_fields() {
        let root = temp_dir("nested-array");
        let out = root.join(OUT_DIR);

        let manifest = ArtifactManifest {
            schema_version: "launchops-artifacts/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![
                ArtifactDescriptor {
                    file: "artifact_schemas.json".to_string(),
                    kind: "artifact_schemas".to_string(),
                    format: "json".to_string(),
                    schema_version: "launchops-artifact-fields/v1".to_string(),
                    schema_ref: None,
                },
                ArtifactDescriptor {
                    file: "matrix.json".to_string(),
                    kind: "matrix".to_string(),
                    format: "json".to_string(),
                    schema_version: "launchops-artifacts/v1".to_string(),
                    schema_ref: Some("artifact_schemas.json#matrix".to_string()),
                },
            ],
        };

        let schema = ArtifactSchemaIndex {
            schema_version: "launchops-artifact-fields/v1".to_string(),
            generated_at: "2026-04-22T00:00:00Z".to_string(),
            artifacts: vec![ArtifactSchemaDoc {
                file: "matrix.json".to_string(),
                kind: "matrix".to_string(),
                format: "json".to_string(),
                shape: "object".to_string(),
                fields: vec![ArtifactFieldDoc {
                    name: "items".to_string(),
                    type_name: "array<object>".to_string(),
                    required: true,
                    description: "items".to_string(),
                    enum_values: Vec::new(),
                    nested_fields: vec![ArtifactFieldDoc {
                        name: "name".to_string(),
                        type_name: "string".to_string(),
                        required: true,
                        description: "name".to_string(),
                        enum_values: Vec::new(),
                        nested_fields: Vec::new(),
                    }],
                }],
            }],
        };

        write_json_file(&out.join("artifact_manifest.json"), &manifest);
        write_json_file(&out.join("artifact_schemas.json"), &schema);
        write_json_file(
            &out.join("matrix.json"),
            &serde_json::json!({
                "items": [
                    { "name": "runtime_backed" }
                ]
            }),
        );

        let validated = validate_emitted_contract(&root).unwrap();
        assert_eq!(validated, 2);
    }

    #[test]
    fn rejects_missing_nested_fields() {
        let root = temp_dir("missing");
        let out = root.join(OUT_DIR);
        write_json_file(&out.join("artifact_manifest.json"), &sample_manifest());
        write_json_file(&out.join("artifact_schemas.json"), &sample_schema());
        write_json_file(
            &out.join("readiness.json"),
            &serde_json::json!({
                "status": "BLOCKED",
                "totals": {}
            }),
        );

        let err = format!("{:#}", validate_emitted_contract(&root).unwrap_err());
        assert!(err.contains("missing required field $.totals.total_md_files"));
    }

    #[test]
    fn validates_markdown_artifact_against_schema_entry() {
        let root = temp_dir("markdown");
        let out = root.join(OUT_DIR);
        write_json_file(&out.join("artifact_manifest.json"), &markdown_manifest());
        write_json_file(&out.join("artifact_schemas.json"), &markdown_schema());
        std::fs::write(out.join("report.md"), "# Report\n\nBody\n").unwrap();

        let validated = validate_emitted_contract(&root).unwrap();
        assert_eq!(validated, 2);
    }

    #[test]
    fn rejects_schema_ref_kind_mismatch() {
        let root = temp_dir("schema-ref");
        let out = root.join(OUT_DIR);
        let mut manifest = markdown_manifest();
        manifest.artifacts[1].schema_ref = Some("artifact_schemas.json#wrong_kind".to_string());
        write_json_file(&out.join("artifact_manifest.json"), &manifest);
        write_json_file(&out.join("artifact_schemas.json"), &markdown_schema());
        std::fs::write(out.join("report.md"), "# Report\n").unwrap();

        let err = format!("{:#}", validate_emitted_contract(&root).unwrap_err());
        assert!(err.contains("schema_ref kind mismatch"));
    }
}
