# X3 Ralph-Style Completion Loop

Task: $ARGUMENTS

Operate like Ralph loop inside Claude Code.

Rules:
- Keep iterating until the task is actually done or a real blocker is proven.
- Every claim must be backed by code, tests, or command output.
- Fix root causes.
- Never weaken tests.
- Run verification before completion.

Loop:
1. Inspect.
2. Plan.
3. Implement.
4. Test.
5. Fix.
6. Document.
7. Verify.
8. Report.
9. Repeat if incomplete.

Completion gate:
Only output <promise>COMPLETE</promise> after ./scripts/x3-verify.sh passes.
