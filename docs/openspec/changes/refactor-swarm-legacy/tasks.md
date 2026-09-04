## 1. Deprecation Boundaries
- [ ] 1.1 Identify production paths that reference `swarm/` or `crates/gpu-swarm/`.
- [ ] 1.2 Add deprecation markers and disable default build/run usage (keep code for reference).
- [ ] 1.3 Gate reusable components behind explicit feature flags.

## 2. Documentation Updates
- [ ] 2.1 Update docs to clarify swarm is deprecated for production launch.
- [ ] 2.2 Document approved reuse points for Inferstructor.

## 3. Safety Checks
- [ ] 3.1 Add CI guard to prevent new dependencies on deprecated modules.
