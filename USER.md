# USER.md - User Model

Store stable user preferences and profile facts as directives that can guide future sessions.

Use one directive per entry:

```md
<!-- observed: YYYY-MM-DD | status: active -->

- Prefer concise progress updates during implementation work.
```

- Begin each directive with an imperative such as `Always`, `Never`, or `Prefer`.
- Record the observation date and either `active` or `superseded` on the metadata line.
- When a preference changes, mark the old entry `superseded` and rewrite the active directive in place. Never append a contradictory active directive.
- Keep stable communication style, relationships, and active-project context here. Put durable non-profile facts and decisions in `MEMORY.md`.

## Directives

<!-- observed: YYYY-MM-DD | status: active -->

- Prefer ...

<!-- observed: 2026-09-04 | status: active -->
- When asked to compare an external repo against this workspace, evaluate strictly by whether it adds real, non-deprecated code we lack; do not import scaffolding, deprecated crates, or redundant alternate implementations of features we already own in another layer. <!-- project: path:/home/lojak/Desktop/xxxstar-main -->

## Related

- [Agent workspace](/concepts/agent-workspace)
