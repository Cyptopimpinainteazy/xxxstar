# BMAD Method Integration

This repository includes a developer wrapper for the BMAD Method (`bmad-method`) located at `crates/vibe-bmad`.

What this does:

- Provides a convenient thin wrapper to install `bmad-method` via `npx`.

- Offers a `workflow-init` script to analyze the repository and recommend the right workflow track.

Usage:

```sh
# Install (alpha recommended):
npm run bmad:install

# Initialize a workspace analysis to get recommendations:
npm run bmad:workflow-init
```

Notes:
 - BMAD is a third-party NPM package (MIT). Please follow the license and usage guidelines in the upstream repo.
- Node.js >= 20 is required.
