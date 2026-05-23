# /update-docs

Run the workspace post-task documentation synchronization flow.

## What this command does
1. Read `.prompt.md`.
2. Collect evidence from the just-completed task (test results, build logs, UTC timestamp, metrics).
3. Identify and update all related markdown files.
4. Validate cross-file consistency.
5. Produce a session summary report in the required format.

## Required evidence
- Task name
- Test results
- Build logs
- UTC timestamp
- Metrics

If any evidence is missing, mark status as PARTIAL and list missing evidence explicitly.
