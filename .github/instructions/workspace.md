# Workspace Instructions

## Standard Post-Completion Step

After any task is completed, run the post-task documentation synchronization flow defined in `.github/prompts/post-task-docs.md`.

Required evidence to include in every documentation update:
- Task name
- Test results
- Build logs
- UTC timestamp
- Metrics

Minimum actions required:
1. Find all related markdown files.
2. Update status/readiness/QA/index files with evidence-backed changes.
3. Run a consistency pass for dates, status labels, and cross-references.
4. Produce a session summary in the format shown in `.github/prompts/post-task-docs.md`.

If evidence is incomplete, mark documentation status as PARTIAL and list what is missing.

## Companion Agent

Use `.github/agents/docs-sync-agent.md` as the companion agent configuration that invokes `.github/prompts/post-task-docs.md` for post-task documentation updates.