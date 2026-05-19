//! Schema generation utilities for exporting TypeScript and GraphQL schemas.

use crate::event_schema::create_event_schema_registry;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Generate all schema files (TypeScript, GraphQL, JSON).
pub async fn generate_schemas(output_dir: &str) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let registry = create_event_schema_registry();

    // Generate TypeScript types
    let ts_content = registry.to_typescript();
    let ts_path = Path::new(output_dir).join("event_types.ts");
    fs::write(&ts_path, ts_content)?;
    info!("Generated TypeScript schema: {}", ts_path.display());

    // Generate GraphQL schema
    let graphql_content = registry.to_graphql();
    let graphql_path = Path::new(output_dir).join("events.graphql");
    fs::write(&graphql_path, graphql_content)?;
    info!("Generated GraphQL schema: {}", graphql_path.display());

    // Generate JSON registry
    let json_content = serde_json::to_string_pretty(&registry)?;
    let json_path = Path::new(output_dir).join("event_schema_registry.json");
    fs::write(&json_path, json_content)?;
    info!("Generated JSON registry: {}", json_path.display());

    // Generate pallet list
    let pallets: Vec<_> = registry.pallets.keys().collect();
    let pallet_list = format!(
        "# X3 Event Pallets\n\nTotal: {} pallets\n\n{}\n",
        pallets.len(),
        pallets
            .iter()
            .map(|p| format!("- {}", p))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let pallets_path = Path::new(output_dir).join("PALLETS.md");
    fs::write(&pallets_path, pallet_list)?;
    info!("Generated pallet list: {}", pallets_path.display());

    info!("Schema generation complete!");
    Ok(())
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
