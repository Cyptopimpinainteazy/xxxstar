// Single source of truth for all data-driven content in the booklet.
// Reads findings.json and feature-matrix.csv directly so no finding is
// ever hand-retyped into prose (eliminates transcription drift).

#let findings-data = json("../findings.json")
#let all-findings = findings-data.findings

#let by-severity(sev) = all-findings.filter(f => f.severity == sev)

#let sev-order = ("Critical", "High", "Medium", "Low", "Informational")
#let sev-counts = sev-order.map(s => (sev: s, count: by-severity(s).len()))

#let feature-rows = csv("../feature-matrix.csv", row-type: dictionary)

// Feature-status tally, computed from feature-matrix.csv status text at
// build time (bucketed into the 7 status vocabulary terms used across the
// 7 source domain audits), not hand-typed — see README.md for the exact
// bucketing rule this mirrors.
#let status-bucket(s) = {
  let u = upper(s)
  if u.contains("DISCONNECTED") { "Disconnected" }
  else if u.contains("PLACEHOLDER") { "Placeholder" }
  else if u.contains("BLOCKED") { "Blocked" }
  else if u.contains("INFORMATIONAL") { "Informational" }
  else if u.contains("PARTIAL") or u.contains("FINDING") { "Partial" }
  else if u.contains("UNVERIFIED") { "Implemented, unverified" }
  else { "Verified" }
}

#let feature-status-order = ("Verified", "Partial", "Implemented, unverified", "Placeholder", "Disconnected", "Blocked", "Informational")
#let feature-status-counts = feature-status-order.map(st => (
  status: st,
  count: feature-rows.filter(r => status-bucket(r.status) == st).len(),
))

// ---- 16-category weighted scoring model --------------------------------
// Weight and score derivation: see Chapter 2 (Scoring Methodology) for the
// full worked rationale per category. Weights sum to 100. Each score is an
// integer 0-100 derived from the evidence cited in that category's row of
// Chapter 2's table and the underlying findings/domain-audit files.
#let subsystem-scores = (
  (name: "Consensus safety", weight: 13, score: 55),
  (name: "Cryptography & key management", weight: 10, score: 38),
  (name: "Test quality", weight: 8, score: 58),
  (name: "Transaction correctness", weight: 8, score: 70),
  (name: "Tokenomics & economic safety", weight: 7, score: 50),
  (name: "State integrity", weight: 6, score: 60),
  (name: "Smart-contract / VM safety", weight: 6, score: 62),
  (name: "Cross-chain safety", weight: 6, score: 45),
  (name: "Operational readiness", weight: 6, score: 50),
  (name: "Networking resilience", weight: 5, score: 55),
  (name: "Governance & upgrade safety", weight: 5, score: 40),
  (name: "Proof-gate enforcement", weight: 5, score: 65),
  (name: "Observability", weight: 4, score: 28),
  (name: "Deployment reproducibility", weight: 4, score: 30),
  (name: "Performance evidence", weight: 4, score: 35),
  (name: "Documentation accuracy", weight: 3, score: 45),
)

#let total-weight = subsystem-scores.fold(0, (acc, r) => acc + r.weight)
#let weighted-sum = subsystem-scores.fold(0, (acc, r) => acc + r.weight * r.score)
#let overall-score = calc.round(weighted-sum / total-weight)
