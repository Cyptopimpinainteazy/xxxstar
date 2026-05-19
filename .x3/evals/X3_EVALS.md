# X3 Evals

A patch passes only if:

- the relevant build/check command passes
- relevant tests pass
- no new stubs are introduced
- no new unsafe unwrap/panic path is introduced in P0 surfaces
- feature registry is updated when features change
- risk register is updated when risks change
- PATCH_LOG.md is updated
- drift detector does not flag missing tests or docs/code mismatch
- mutation gate is satisfied or explicitly documented as blocked
