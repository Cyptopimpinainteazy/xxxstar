#[test]
fn gateway_binary_wires_the_http_service() {
    let source = include_str!("../src/main.rs");

    for required in [
        "Database::connect",
        "create_schema",
        "create_router",
        "axum::serve",
    ] {
        assert!(
            source.contains(required),
            "gateway entrypoint is missing {required}"
        );
    }
}

#[test]
fn gateway_migrations_are_contiguous() {
    let migrations = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
        .expect("gateway migrations directory must exist");
    let mut names: Vec<String> = migrations
        .map(|entry| {
            entry
                .expect("migration entry must be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();

    assert_eq!(
        names,
        vec![
            "0001_benchmark_reports.sql",
            "0002_benchmark_report_workload_profile.sql",
            "0003_benchmark_jobs.sql",
            "0004_orchestra_workflows.sql",
            "0005_vote_window_tally.sql",
            "0006_funding_swarm_public_ledger.sql",
        ]
    );
}
