# X3 Mainnet Gate

Scope: $ARGUMENTS

Run the strict mainnet-readiness gate.

Steps:
1. Inspect git status.
2. Inspect changed files.
3. Search for TODO/FIXME/stub/mock/placeholder/todo!/unimplemented!/panic!.
4. Run ./scripts/x3-verify.sh.
5. If verification fails, fix the root cause.
6. If docs are stale, update them.
7. Do not claim COMPLETE until verification passes.

Output:
- readiness percent
- blockers
- exact failed commands
- fixed items
- remaining risks
- next action
