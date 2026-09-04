PASS 1: Scanner / free-scan
- Load .roo/rules.md and .legion/SCANNER.md.
- Run .scripts/x3_repomix_pack.sh when a fresh context pack is needed.
- Run .scripts/x3_full_scan.sh.
- Run .scripts/x3_smell_scan.sh.
- Create CODE_COVERAGE_TRACKER.md.
- Create OLD_PROJECT_FEATURE_INVENTORY.md.
- Create CURRENT_PROJECT_FEATURE_INVENTORY.md.
- Create FILE_INDEX.md.
- Create .x3/X3_SYSTEM_MAP.md.
- Process every file from .cache/x3_full_file_list.txt.
- Do not integrate yet.

PASS 2: Integrator / cheap-coder
- Load .roo/rules.md and .legion/INTEGRATOR.md.
- Use Traycer tasks from .traycer/X3_TASK_CHAIN.md when available.
- Use CODE_COVERAGE_TRACKER.md, OLD_PROJECT_FEATURE_INVENTORY.md, CURRENT_PROJECT_FEATURE_INVENTORY.md, .repomix/*.md, and .reports/x3_smells.txt.
- Create FEATURE_GAP_REPORT.md.
- Create INTEGRATION_PLAN.md.
- Implement P0/P1 features only.
- Run tests after each patch.
- Update PATCH_LOG.md.
- Update .x3/X3_FEATURE_REGISTRY.md and .x3/X3_RISK_REGISTER.md.

PASS 3: Auditor / heavy-claude
- Load .roo/rules.md and .legion/AUDITOR.md.
- Audit completed integration.
- Create DEEP_AUDIT_REPORT.md.
- Create MAINNET_READINESS_DELTA.md.
- Create FINAL_AUDIT_REPORT.md.
- Rank blockers P0/P1/P2.
- Do not make broad rewrites unless required.

PASS 4: Architect / cheap-coder or heavy-claude
- Load .roo/rules.md and .legion/ARCHITECT.md.
- Create NEW_IDEAS_REPORT.md.
- Create X3_COMPETITIVE_ADVANTAGE.md.
- Create FASTEST_MAINNET_PLAN.md.
- Provide better architecture ideas, competitor features worth copying, X3-only differentiators, mainnet launch shortcuts, what to cut, and what to build next.

PASS 5: Fixer / cheap-coder or local-ollama
- Load .roo/agents/X3_AGENT.md, .roo/rules.md, and .legion/FIXER.md.
- Fix current failing build/test only.
- Do not add features.
- Do not refactor unrelated code.
- Stop when the failing command passes or blocker is documented.
