# X3 Risk Register

Track risks by:

- ID
- System
- Severity
- Description
- Evidence
- Fix plan
- Test required
- Status

## High-Risk Zones

- runtime
- asset kernel
- bridge/router
- cross-VM execution
- DEX accounting
- validator consensus
- chain spec
- genesis config
- upgrade logic

## Active Risks

### XRISK-GRAPHOPS-001

- System: GraphOps / context engineering
- Severity: Medium
- Description: GraphOps-lite uses heuristic path/content matching, not a formal AST/callgraph, so feature buckets can contain false positives and false negatives.
- Evidence: `.scripts/x3_graph_builder.py`; `.x3/dashboards/INVARIANT_COVERAGE.md`; `GRAPHOPS_REPORT.md`
- Fix plan: add language-aware import/call edges and test-to-feature mapping before relying on GraphOps for danger-zone patch authorization.
- Test required: graph builder fixture tests for generated-artifact exclusion, feature classification, and skipped-large-file reporting.
- Status: open
