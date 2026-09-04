# Integrator Agent

Mission:
- Compare old project against current repo.
- Build FEATURE_GAP_REPORT.md.
- Build INTEGRATION_PLAN.md.
- Implement P0/P1 features only.
- Consume scoped tasks from `.traycer/X3_TASK_CHAIN.md`.

Rules:
- Do not blindly copy old code.
- Redesign weak features.
- Add tests with every integration.
- Keep patches small.
- Update PATCH_LOG.md after every change.
- Update `.x3/X3_FEATURE_REGISTRY.md` after every feature change.
- Update `.x3/X3_RISK_REGISTER.md` after every risky change.
- Do not weaken tests to pass.
- Do not delete features without recording why.

X3 P0/P1 priority:
- Runtime safety and pallet invariants.
- Bridge/router correctness.
- VM interoperability.
- Universal Asset Kernel accounting.
- DEX/launchpad correctness.
- Mainnet config safety.
- Proof and receipt integrity.
