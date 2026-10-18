use serde_json::json;
use x3_gateway::funding_proposals::ProposalInput;

fn input() -> serde_json::Value {
    json!({"expected_version":0,"source_snapshot_id":"snapshot-fixture",
        "project_title":"Reproducible scheduling research",
        "research_question":"Can deterministic scheduling reduce verified queue latency?",
        "technical_approach":"Compare the implementation with a sequential reference.",
        "evaluation_plan":"Publish executable benchmarks and failure cases.",
        "organization_facts":"Legal identity and eligibility not yet verified.",
        "requested_usd_cents":null})
}

#[test]
fn proposal_contract_rejects_empty_unbounded_and_ambiguous_inputs() {
    let valid: ProposalInput = serde_json::from_value(input()).unwrap();
    assert!(valid.validate().is_ok());
    for (field, value) in [
        ("project_title", json!(" ")),
        ("research_question", json!("x".repeat(16385))),
        ("expected_version", json!(-1)),
        ("requested_usd_cents", json!(-1)),
        ("requested_usd_cents", json!(9_007_199_254_740_992_i64)),
        ("source_snapshot_id", json!("")),
    ] {
        let mut data = input();
        data[field] = value;
        assert!(serde_json::from_value::<ProposalInput>(data)
            .map(|v| v.validate().is_err())
            .unwrap_or(true));
    }
    for extra in [
        "submit",
        "eligibility_verified",
        "approved",
        "source_document",
    ] {
        let mut data = input();
        data[extra] = json!(true);
        assert!(serde_json::from_value::<ProposalInput>(data).is_err());
    }
    let mut data = input();
    data["requested_usd_cents"] = json!(1.5);
    assert!(serde_json::from_value::<ProposalInput>(data).is_err());
}
