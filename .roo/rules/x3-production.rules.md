# X3 Production Rules

- Ship real runtime-wired code only.
- Never use placeholders/stubs for production paths.
- Never skip, weaken, or delete tests to force green.
- Critical path changes require tests + invariant coverage.
- Mainnet readiness requires all release gates passing.

Run:

```bash
make guard
make test
make audit
make mainnet-check
make fresh-machine-check
```