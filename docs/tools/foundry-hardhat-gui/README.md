# Foundry + Hardhat + Ganache GUI

Local web control panel for Foundry, Hardhat, and Ganache profiles in this repository.

## Features

- Auto-discovers projects containing `foundry.toml` or `hardhat.config.*`
- Quick install check panel with exact commands for missing tools
- Runs common commands:
  - Foundry: `build`, `test`, `clean`, `node`
  - Hardhat: `compile`, `test`, `node`
- Ganache workspace profile controls:
  - Accounts/keys
  - Gas/hardfork
  - Logging options
  - Analytics toggle (workspace metadata)
  - Truffle project links
  - Start/stop profile with generated Ganache CLI command
- Live task status and log tail
- Stop running long-lived tasks from the UI

## Run

```bash
python3 tools/foundry-hardhat-gui/server.py --workspace .
```

Then open:

```text
http://127.0.0.1:8787
```

## Optional launcher

Use the repo helper:

```bash
./scripts/run-foundry-hardhat-gui.sh
```
