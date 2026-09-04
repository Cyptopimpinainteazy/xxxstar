# X3 Swarm Scripts

This folder contains local swarm orchestration helpers for X3 Atomic Star.

- `approve_task.sh`: manage manual swarm tasks and generate a task summary report.
- `swarm_health.sh`: verify the swam API and worker health.
- `swarm_self_test.sh`: run a local functional smoke test, including `approve_task.sh report`.

## `approve_task.sh report`

Run:

```bash
scripts/swarm/approve_task.sh report reports/swarm_task_summary.md
```

This command fetches current swarm tasks from the API and writes a markdown task summary to `reports/swarm_task_summary.md`.
