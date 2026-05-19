# Hardhat Hygiene

This Hardhat project builds sources from `./contracts` only.

A legacy `generated-contracts/` directory also exists in the workspace. If old artifact files from that directory remain under `artifacts/generated-contracts/`, Hardhat can report ambiguous factories for names like `BOT` or `SimpleDEX`.

To keep test runs deterministic, `npm test` now runs `hardhat clean` first via the `pretest` script. This removes stale artifacts before compilation.

If you need to interact with generated contracts intentionally, prefer fully qualified contract names and a dedicated build flow rather than relying on mixed artifact directories.
