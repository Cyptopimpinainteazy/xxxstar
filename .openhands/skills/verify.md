---
name: verify
description: The checks a change must pass before it is reported as done, and how to report.
triggers:
  - done
  - finished
  - verify
  - ready
  - complete
---

# Before reporting a change as done

Run, for each crate you changed (see `build-on-low-disk` for the environment):

```bash
cargo fmt -p <crate> -- --check
cargo clippy -p <crate> --all-targets -- -D warnings
cargo test -p <crate>
```

`scripts/x3-verify.sh` runs the full workspace gates. Only run it when asked: it builds
the whole workspace.

If a check fails, report the failure with its output. Do not claim success, and do not
change a test or lint setting to make a check pass.

End with:

```
Files changed:
Commands run:   (each command and its result)
Proof result:
Remaining blockers:
```

Do not commit or push unless the task asks you to.
