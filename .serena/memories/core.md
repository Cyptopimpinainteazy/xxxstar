# X3 Atomic Star / X3 Chain

- Substrate-based multi-VM chain: X3Native, X3Evm, X3Svm.
- Source of truth is executable code plus passing verification; status/roadmap markdown is not proof.
- Atomic route invariant: parse -> semantic check -> typed IR -> validate -> preflight -> reserve/lock -> execute -> verify receipts/proofs -> commit or rollback -> settle -> assert invariants.
- Security-critical properties: replay protection, expiry/deadlines, rollback/refund, receipt/proof verification, canonical supply invariant.
- External bridges are disabled at genesis and governance/audit gated.
- Primary navigation: README.md, AGENTS.md, LAUNCH_SCOPE.md, docs/current/README.md, docs/current/FAILURES_AND_TODOS.md.
- Language note: authoritative x3-lang implementation is the Python/pipeline track under x3-lang/; crates/x3-compiler is experimental.
- Module memories: read `mem:tech_stack` for toolchains and package managers; `mem:suggested_commands` for common commands; `mem:conventions` for coding patterns; `mem:task_completion` for completion gates.