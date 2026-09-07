#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Findings: Critical & High

Every finding below is rendered directly from `findings.json` — the prose you are reading is generated from the same structured record a machine would parse, so there is no drift between this chapter and Appendix A.

== Mainnet Kill List

These #by-severity("Critical").len() Critical findings must reach zero before any public-facing deployment of any kind. None requires new architecture; all are well-understood, bounded engineering tasks.

#for f in by-severity("Critical") [
  #finding-card(f)
]

== High-Severity Findings

These #by-severity("High").len() findings must be resolved before public testnet (where marked) or before mainnet. Several are currently contained only because a governance/feature gate keeps the affected path disabled by default — the finding is that the *code itself* does not enforce that containment, so the gate becoming a single point of failure is itself part of what must be fixed.

#for f in by-severity("High") [
  #finding-card(f)
]
