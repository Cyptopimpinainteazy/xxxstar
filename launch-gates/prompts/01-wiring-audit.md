# 01: Complete Wiring Audit

## Objective
Build a complete wiring map of the X3 repo. Prove that every major module is actually connected to runtime, tests, CLI, RPC, frontend, or deployment.

## Instructions

You are a wiring architect.

**This Repomix file contains the X3 repo.**

Build a complete wiring map.

For every major module (crate, pallet, app), show:
- **Entrypoints:** How is this module called? From where? What's the call signature?
- **Dependencies:** What does it depend on? What depends on it?
- **Downstream calls:** What runtime calls, storage mutations, or state changes does it trigger?
- **Storage touched:** What pallet storage items does it read/write?
- **Events emitted:** What events does it emit? When?
- **Errors returned:** What error types can it return? Are they all reachable?
- **Tests covering it:** Unit tests? Integration tests? E2E tests? Missing coverage?
- **Missing integration points:** Should it be wired somewhere but isn't?

**Prove it by showing:**
1. Source file and function signature
2. Where it's imported/called
3. What extrinsic or RPC endpoint exposes it
4. What test exercises it

**Flag anything that:**
- Exists as a file/module but is not reachable from runtime, CLI, RPC, frontend, tests, or deployment scripts
- Has a TODO/FIXME saying "wire this later"
- Is behind a feature flag but the flag is never enabled
- Was moved/renamed but old references still exist
- Has dangling imports or unused dependencies

## Expected Output

**WIRING MAP**
```
Module: x3-bridge
  Entrypoints:
    - ExtrinsicA (runtime/construct_runtime)
    - RPC call bridge_status (rpc/runtime_api)
  Dependencies: [list]
  Downstream: [list]
  Storage: [list]
  Events: [list with reachability]
  Tests:
    - bridge_basic_flow ✅
    - bridge_replay_protection ❌ MISSING
  Integration: ✅ COMPLETE
  
[repeat for every major module]
```

**UNWIRED MODULES**
- List anything that exists but is not reachable

**MISSING INTEGRATIONS**
- List should-be-wired but isn't

**SCORE: [X/100]**
- 90+ = Ready
- 75-89 = Gaps identified
- <75 = Critical wiring broken
