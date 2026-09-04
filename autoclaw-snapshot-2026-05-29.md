# AutoClaw Health Snapshot

- Date:    2026-05-29
- Version: 3.1.2
- Workspace: /home/x3star/Desktop/xxxstar-main

---

## Doctor Report

AutoClaw Doctor — Health Report
Generated: 2026-05-29T23:07:05.241Z
Extension: /home/x3star/.vscode/extensions/zippytechnologiesllc.autoclaw-3.1.2
Today: 2026-05-29

## Workspace
  workspace: /home/x3star/Desktop/xxxstar-main
  .autoclaw/ exists: yes
  .gitignore: .autoclaw/ ignored

## KDream State
  status:    running
  tick:      1
  started:   2026-05-27T08:49:48Z
  lastDream: (unset)

## MEMORY.md
  lines:               18
  open follow-ups:     0
  done follow-ups:     0
  ## Follow-ups:       present
  ## Facts:            present
  ## Observations:     present

## Logs
  total log files:        1
  today's log:            not present

## Compilation
  out/ present:        yes
  out/extension.js:    present
  stale:               no
  message: no source files to compare

## Adapter Schema
  status: ok
  - antigravity: skills=[kdream, autobuild, mateam]
  - claude-code: skills=[kdream, autobuild, mateam]
  - cline: skills=[kdream, autobuild, mateam]
  - continue: skills=[kdream, autobuild, mateam]
  - cursor: skills=[kdream, autobuild, mateam]
  - kilocode: skills=[(custom layout)]
  - kiro: skills=[kdream, autobuild, mateam]
  - windsurf: skills=[kdream, autobuild, mateam]
  - zippymesh: skills=[(custom layout)]

## Adapter Drift
  status:  skipped
  message: adapter drift check skipped — run `npm run adapters:compile` first

## Adapter Installation

| Host | Extension Installed | Destination Exists | Expected Files | Destination |
| --- | --- | --- | --- | --- |
| claude-code | yes | yes | 3/3 | /home/x3star/.claude/skills |
| kilocode | no | no | (missing) | /home/x3star/Desktop/xxxstar-main/.kilocodemodes |
| cline | yes | yes | 3/3 | /home/x3star/Desktop/xxxstar-main/.clinerules |
| cursor | no | no | 0/3 | /home/x3star/Desktop/xxxstar-main/.cursor/rules |
| antigravity | no | no | 0/3 | /home/x3star/Desktop/xxxstar-main/.agent/rules |
| windsurf | no | no | 0/3 | /home/x3star/Desktop/xxxstar-main/.windsurf/rules |
| kiro | no | no | 0/3 | /home/x3star/Desktop/xxxstar-main/.kiro/steering |
| continue | no | no | 0/3 | /home/x3star/Desktop/xxxstar-main/.continue/prompts |

## Git Health
  branch:               main
  upstream:             (none)
  ahead/behind:         0/0
  uncommitted files:    566
  untracked files:      66
  last commit:          133h ago
  note: no upstream tracking branch (push with --set-upstream to enable ahead/behind)

## ZippyMesh LLM Router
  status:  warning
  details: Not detected — start ZippyMesh on localhost:20128

## Skills Source (VSIX sanity)
  kdream/SKILL.md: present
  autobuild/SKILL.md: present
  mateam/SKILL.md: present

## KG Daemon
  enabled:        no (autoclaw.kg.enabled = false)
  port:           9877
  deps installed: no — cd packages/kg-daemon && npm install
  entry:          /home/x3star/.vscode/extensions/zippytechnologiesllc.autoclaw-3.1.2/packages/kg-daemon/dist/server.js (MISSING — npm run build)
  child pid:      (not running)
  last /health:   (not probed this session)

## AutoBuild
  workflows dir:    /home/x3star/Desktop/xxxstar-main/.autoclaw/autobuild/workflows
  workflow files:   0
  registry.json:    absent
  (no workflows scheduled)

---

## KDream state.json

```json
{
  "status": "running",
  "started": "2026-05-27T08:49:48Z",
  "tick": 1,
  "lastDream": null,
  "todos": [
    "crates/x3-rpc/src/validator_rpc.rs",
    "crates/x3-bridge/src/cross_chain_proofs.rs",
    "crates/x3-bridge-adapters/src/solana.rs",
    "crates/x3-bridge-adapters/src/ethereum.rs",
    "crates/x3-bridge-adapters/src/bitcoin.rs",
    "runtime/src/lib.rs"
  ]
}
```

## Recent Log (last 30 lines of 2026-05-29.md)

_no log for today — `.autoclaw/kdream/logs/<today>.md` not present_

## Open Follow-ups (from MEMORY.md)

_no open follow-ups_
