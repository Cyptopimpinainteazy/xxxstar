# 🗺️ X3 CHAIN - CARTOGRAPHER'S REPORT
## Deep System Architecture Map (YOLO FINISHER v3.0)

**Date:** 2026-01-14
**Status:** INITIAL CARTOGRAPHIC SCAN
**Purpose:** Complete execution and data lifecycle mapping for gap detection

---

## 1️⃣ SYSTEM EXECUTION MAP

### Layer 0: AI/GPU Infrastructure
**Entry Point:** `run-everything.sh` → `start_ollama()`
```
OLLAMA (Multi-GPU)
├── Port: 11434
├── Health Check: GET /api/tags
├── Healthz Endpoint: HTTP GET
├── Recovery: systemctl restart ollama
├── Failure Mode: Missing → WARN + Continue
└── Dependency: Docker socket / systemd
```

**Assumptions Found:**
- ❌ ASSUMPTION: systemctl exists and Ollama is managed by it
- ❌ ASSUMPTION: Ollama listens on localhost:11434 OR systemd-configured host
- ❌ ASSUMPTION: Resolution works (docker0 fallback, 172.17.0.1 candidate)
- ❌ ASSUMPTION: First HTTP /api/tags succeeds means Ollama is ready (not just listening)

**Data Flow:**
```
GPU Hardware → Ollama Daemon → HTTP API
                    ↓
         OLLAMA_URL env var (resolved)
                    ↓
         Passed to: swarm/api_server.py, UIs, others
```

**Gap Found:** ⚠️ OLLAMA_URL resolution can fail silently, but downstream services DON'T validate it exists at startup.

---

### Layer 1: Blockchain Node
**Entry Point:** `run-everything.sh` → `start_blockchain()`
```
BLOCKCHAIN NODE (Substrate)
├── Binary Discovery:
│   ├── target/release/x3-chain-node
│   ├── target/$TRIPLE/release/x3-chain-node
│   ├── find first matching under target/*/release/
├── Port: 9944 (WS)
├── RPC Options: --rpc-cors all --rpc-methods Unsafe
├── Mode: --dev (development)
└── Health: Port in use = healthy (no /health endpoint checked)
```

**Assumptions Found:**
- ❌ ASSUMPTION: Binary exists somewhere in target/
- ❌ ASSUMPTION: --dev mode is correct for all contexts
- ❌ ASSUMPTION: Port 9944 being in use = node is healthy (could be zombie)
- ❌ ASSUMPTION: First startup succeeds (no recovery if fails mid-init)

**Data Flow:**
```
$ x3-chain-node --dev
         ↓
BLOCKCHAIN_PORT env → 9944
         ↓
BLOCKCHAIN_WS="ws://localhost:9944"
         ↓
Passed to: swarm/api_server.py, UIs
         ↓
RPC Calls (JSON-RPC over WebSocket)
```

**Gap Found:** ⚠️ Node startup failure not caught. No verify that WS API is actually responding.

---

### Layer 2: Swarm API Server
**Entry Point:** `run-everything.sh` → `start_swarm_server()`

```python
SWARM_API_SERVER (Python async)
├── Entry: swarm/api_server.py or swarm/unified_server.py
├── Port: 8080
├── Framework: aiohttp
├── Health Endpoint: GET /health
├── WebSocket: /ws (multicast hub)
├── Routes:
│   ├── /api/agents/* (management)
│   ├── /api/tasks/* (execution)
│   ├── /api/gpu/* (GPU tasks)
│   ├── /api/swarm/* (swarm state)
│   ├── /metrics (prometheus)
│   └── /ws (real-time updates)
├── Dependencies:
│   ├── asyncio
│   ├── aiohttp
│   ├── BLOCKCHAIN_WS_URL (runtime inject)
│   ├── OLLAMA_URL (runtime inject)
├── Environment Variables:
│   ├── SWARM_HOST="0.0.0.0"
│   ├── SWARM_PORT=8080
│   ├── BLOCKCHAIN_WS_URL
│   ├── OLLAMA_URL
│   ├── TOTAL_GPUS=100
│   └── LOG_LEVEL=INFO
└── Startup Checks: NONE EXPLICIT (just starts listening)
```

**Assumptions Found:**
- ❌ ASSUMPTION: BLOCKCHAIN_WS_URL will be set and reachable
- ❌ ASSUMPTION: OLLAMA_URL will be set and reachable
- ❌ ASSUMPTION: /health endpoint works immediately
- ❌ ASSUMPTION: Orchestrator and GPUManager initialize successfully
- ❌ ASSUMPTION: No prior state corrupted (fresh DB assumed)

**Data Flow:**
```
swarm/api_server.py
       ↓
    main() async
       ↓
    SwarmAPIServer.start()
       ↓
    ├─ WebSocketManager init
    ├─ GPUOrchestrator init
    ├─ AgentJobDistributionManager init
    └─ aiohttp.frontend/frontend/web.run_app()
       ↓
    Listen on 0.0.0.0:8080
       ↓
    ├─ REST endpoints
    ├─ WebSocket hub
    └─ Status routes
```

**Gap Found:** ⚠️ No startup validation. Swarm API can listen but orchestrator be broken.

---

### Layer 3: Frontend Applications (Next.js)

```
APPS:
├── explorer (X3OS)        @ :3001
├── wallet                 @ :3002
├── dex                    @ :3003
├── apps/apps/next-solana-main-legacy-2-legacy-2       @ :3000
├── apps/apps/swarm-apps/apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2        @ :3100
└── quantum-apps/apps/dash-legacy-2-legacy-2board      @ (Tauri)

Environment Injection:
├── NEXT_PUBLIC_SWARM_API_URL="http://localhost:8080"
├── NEXT_PUBLIC_SWARM_WS_URL="ws://localhost:8080/ws"
├── NEXT_PUBLIC_GPU_WS_URL="ws://localhost:8080/ws/gpu"
├── NEXT_PUBLIC_BLOCKCHAIN_WS_URL="ws://localhost:9944"
└── NEXT_PUBLIC_OLLAMA_URL="${OLLAMA_URL}"

Startup:
1. npm install (if node_modules missing)
2. npx next dev --port $PORT
3. Listen on port $PORT
4. Expose /health? (check if implemented)
```

**Assumptions Found:**
- ❌ ASSUMPTION: env vars injected at bfrontend/uild time or runtime
- ❌ ASSUMPTION: npm install succeeds
- ❌ ASSUMPTION: Next.js dev server doesn't crash on bad env vars
- ❌ ASSUMPTION: WS connections to swarm/blockchain won't fail
- ❌ ASSUMPTION: UI handles API unavailability gracefully

**Data Flow:**
```
Next.js App (page/component)
       ↓
    useEffect/client-side init
       ↓
    Connect to ws://localhost:8080/ws
    Connect to ws://localhost:9944
       ↓
    Subscribe to streams
    Render UI
```

**Gap Found:** ⚠️ No startup validation that APIs are reachable. UIs can start "successfully" while blind.

---

### Layer 4: Desktop (Tauri) Applications

```
X3OS Tauri Desktop
├── Binary: apps/x3os/src-tauri/target/release/x3os
├── Entry: GUI window
├── Framework: Tauri + React
└── Same env var injection as Next.js

Quantum Dashboard Tauri
├── Binary: apps/quantum-apps/apps/dash-legacy-2-legacy-2board/src-tauri/target/release/x3-quantum-apps/apps/dash-legacy-2-legacy-2board
├── Entry: GUI window
└── Same env var injection as Next.js
```

**Gaps:**
- ❌ No startup validation
- ❌ Desktop apps can launch but API connections fail silently

---

## 2️⃣ DATA LIFECYCLE MAP

### Event Flow Architecture

```mermaid
┌─────────────────────────────────────────────────────────────┐
│ CHAIN EVENTS (Blockchain → Swarm)                          │
├─────────────────────────────────────────────────────────────┤
│ ┌─ Block finalized (blockchain)                            │
│ ├─ JSON-RPC call (WebSocket)                               │
│ ├─ ChainEvent dataclass created                            │
│ ├─ Broadcast via ws_manager.broadcast('chain-events', ...) │
│ └─ Subscribed clients receive update                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ SWARM EVENTS (Agent lifecycle)                             │
├─────────────────────────────────────────────────────────────┤
│ ┌─ Agent spawned/died/mutated                              │
│ ├─ SwarmEvent dataclass created                            │
│ ├─ Persisted to DB? (UNKNOWN)                              │
│ ├─ Broadcast via ws_manager.broadcast('swarm-events', ...) │
│ └─ Subscribed clients receive update                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ AGENT EVENTS (Task execution)                              │
├─────────────────────────────────────────────────────────────┤
│ ┌─ Agent started/completed/slashed/quarantined             │
│ ├─ AgentEvent dataclass created                            │
│ ├─ Persisted to DB? (UNKNOWN)                              │
│ ├─ Broadcast via ws_manager.broadcast('agent-events', ...) │
│ └─ Subscribed clients receive update                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ GOVERNANCE EVENTS (Freezes, overrides, disputes)           │
├─────────────────────────────────────────────────────────────┤
│ ┌─ Governance action triggered                             │
│ ├─ GovernanceEvent dataclass created                       │
│ ├─ Persisted to DB? (UNKNOWN)                              │
│ ├─ Broadcast via ws_manager.broadcast(..., ...)            │
│ └─ Subscribed clients receive update                       │
└─────────────────────────────────────────────────────────────┘
```

**Data Persistence:**
- ❌ **UNKNOWN:** Do events persist to DB or just broadcast?
- ❌ **UNKNOWN:** What DB? SQL? MongoDB? Memory-only?
- ❌ **UNKNOWN:** If server restarts, are events replayed?
- ❌ **UNKNOWN:** If broadcast fails, is event lost?

---

### Agent Data Lifecycle

```
Agent Creation:
┌─ POST /api/agents/register (from UI or external)
├─ Agent record created (in-memory? DB?)
├─ Broadcast to /ws subscribers
├─ Assigned to GPUManager work queue
└─ Start execution

Agent Execution:
┌─ Fetch task from queue
├─ Execute (GPU? CPU? where?)
├─ Capture metrics
├─ Update state (in-memory? DB?)
├─ Broadcast status updates
└─ Handle success/failure

Agent Destruction:
┌─ Trigger: completion, timeout, slashing, or manual
├─ Save final metrics (persisted? where?)
├─ Broadcast death event
├─ Cleanup resources
└─ Archive state (yes/no?)
```

**Critical Gaps:**
- ❌ Agent state storage layer is **undefined**
- ❌ Persistence model is **unknown**
- ❌ Restart behavior is **unknown**
- ❌ Data recovery is **not documented**

---

## 3️⃣ TRUST BOUNDARY MAP

```
┌──────────────────────────────────────────────────────────┐
│ PUBLIC INTERNET (UNTRUSTED)                              │
├──────────────────────────────────────────────────────────┤
│  • UIs (could be compromised)                            │
│  • External agents                                        │
│  • Possible MEV attackers (if DeFi involved)             │
└──────────────────────────────────────────────────────────┘
        ↓↓ (HTTP/WS, NO AUTH?)
┌──────────────────────────────────────────────────────────┐
│ SWARM API SERVER (Python)                                │
├──────────────────────────────────────────────────────────┤
│  • API routes: no auth tokens seen                        │
│  • WebSocket: no subscription validation                 │
│  • Requests: no rate limiting visible                    │
│  • Handlers: direct access to orchestrator               │
└──────────────────────────────────────────────────────────┘
        ↓↓ (no explicit auth)
┌──────────────────────────────────────────────────────────┐
│ BLOCKCHAIN NODE (Substrate)                              │
├──────────────────────────────────────────────────────────┤
│  • RPC: --rpc-methods Unsafe                             │
│  • CORS: --rpc-cors all                                  │
│  • Dev mode: all privs available                         │
│  • State: can be manipulated                             │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│ GPU/OLLAMA (Untrusted execution environment)             │
├──────────────────────────────────────────────────────────┤
│  • Can run arbitrary models                              │
│  • Can access host network (via Ollama daemon)           │
│  • Can consume arbitrary resources                       │
│  • Can exit/crash without cleanup                        │
└──────────────────────────────────────────────────────────┘
```

**Security Gaps (CRITICAL):**
- ❌ No authentication between Swarm API and blockchain
- ❌ No rate limiting on API endpoints
- ❌ No authorization checks on handlers
- ❌ GPU tasks can run untrusted code
- ❌ No sandbox isolation

---

## 4️⃣ CRITICAL GAPS & ASSUMPTIONS

### Startup Assumptions (ALL BREAKING):
1. **Ollama Resolution**
   - Assumption: `resolve_ollama_url()` works correctly
   - Actual: Can fail, fallback to localhost which may not exist
   - **Impact:** Swarm API gets wrong OLLAMA_URL, all GPU tasks fail

2. **Node Binary Discovery**
   - Assumption: Binary exists in one of several expected paths
   - Actual: Binary may be missing or in unexpected location
   - **Impact:** Blockchain doesn't start, but script continues

3. **Port Availability**
   - Assumption: Ports 9944, 8080, 3001, 3002, 3003 are free
   - Actual: Ports could be in use by zombie processes
   - **Impact:** Services fail to bind but script shows success

4. **Health Check Reliability**
   - Assumption: Port in use = service healthy
   - Actual: Port in use could mean service crashed mid-startup
   - **Impact:** Services fail silently, downstream connects fail

5. **Environment Variable Propagation**
   - Assumption: Env vars set in parent shell available to child processes
   - Actual: Subprocess isolation could prevent access
   - **Impact:** Services start without reqfrontend/uired config

### Runtime Assumptions (ALL BREAKING):
1. **API Availability at Startup**
   - Assumption: Backend APIs reachable when UIs load
   - Actual: Network, DNS, or service delays could cause failures
   - **Impact:** UIs fail to initialize, show blank/error

2. **WebSocket Connection Stability**
   - Assumption: WS connections remain open and resilient
   - Actual: Network issues cause drops, no reconnect logic visible
   - **Impact:** Real-time updates stop, UI becomes stale

3. **Agent State Persistence**
   - Assumption: Agent state survives process restart
   - Actual: State storage mechanism is unclear/missing
   - **Impact:** Restart loses agent registry, tasks get orphaned

4. **Database Schema Initialization**
   - Assumption: DB tables exist and are initialized
   - Actual: Migration/init logic not visible in startup
   - **Impact:** First query fails, no graceful fallback

5. **Graceful Degradation**
   - Assumption: Missing dependency doesn't crash system
   - Actual: Most services hard-fail if dependency missing
   - **Impact:** One failure cascades to entire stack

---

## 5️⃣ LIFECYCLE GAPS

### Startup Sequence Issues:
```
Current:
1. Start Ollama (may fail)
2. Start Blockchain (may fail)
3. Start Swarm (doesn't validate deps)
4. Start UIs (don't validate swarm)
5. Return success

Problems:
├─ No cross-validation of dependencies
├─ No startup order enforcement
├─ No initialization checks
├─ No rollback on failure
└─ No recovery actions defined
```

### Shutdown Sequence Issues:
```
Current:
1. Trap EXIT/INT
2. Kill all PIDs from .x3-pids
3. Kill ports 9944, 8080, 3001, 3002, 3003

Problems:
├─ Could miss processes if PID tracking failed
├─ Could kill wrong PIDs if port reused
├─ No graceful shutdown signals (SIGTERM → SIGKILL)
├─ No state flush before kill (transactions incomplete?)
├─ No cleanup validation
```

### Restart Recovery Issues:
```
Missing:
├─ If Ollama starts but HTTP fails, no recovery
├─ If blockchain starts but can't generate blocks, no recovery
├─ If swarm connects to wrong blockchain, no detection
├─ If database is corrupted, no migration
├─ If state is inconsistent, no reconciliation
```

---

## 6️⃣ UNCHARTED TERRITORY

### Unknown Subsystems:

1. **Database Layer**
   - Location: unknown
   - Type: unknown
   - Schema: unknown
   - Migrations: unknown
   - Initialization: unknown
   - Backup: unknown

2. **Metrics/Observability**
   - Where does `/metrics` endpoint live?
   - What metrics are exposed?
   - Where is Prometheus scraping from?
   - What alerts are configured?

3. **GPU Task Execution**
   - Where do GPU tasks run?
   - What is the execution model?
   - How is resource allocation handled?
   - What is the failure recovery?

4. **Agent Registry**
   - Where is `agent_registry` defined?
   - What data is stored?
   - How is it persisted?
   - How is it reconciled on restart?

5. **Cross-Chain Integration**
   - What chains are supported?
   - How is state synchronized?
   - What is the consensus model?
   - How is reorg handled?

---

## 7️⃣ NEXT STEPS FOR OTHER AGENTS

### For THE BREAKER 💣:
- [ ] Force Ollama URL resolution to fail
- [ ] Kill blockchain mid-startup
- [ ] Corrupt swarm environment variables
- [ ] Drop WebSocket connections
- [ ] Restart services mid-transaction
- [ ] Fill disk while services running
- [ ] Corrupt database files

### For THE AUDITOR 🕵️:
- [ ] Review all API endpoints for auth
- [ ] Check rate limiting
- [ ] Verify privilege minimization
- [ ] Test malicious input handling
- [ ] Review smart contract interactions
- [ ] Check for reentrancy vulnerabilities
- [ ] Test flashloan scenarios

### For THE INTEGRATOR 🔩:
- [ ] Find all unused API endpoints
- [ ] Find all orphaned event types
- [ ] Find all dangling database tables
- [ ] Verify end-to-end traceability
- [ ] Remove dead code

### For THE FIXER 🛠️:
- [ ] Add startup validation
- [ ] Add health checks
- [ ] Add graceful degradation
- [ ] Add recovery logic
- [ ] Add state reconciliation
- [ ] Add logging/tracing

### For THE VERIFIER ✅:
- [ ] Test cold start from clean OS
- [ ] Test agent spawn → execution → death
- [ ] Test blockchain transaction finality
- [ ] Test WebSocket reconnection
- [ ] Test database persistence
- [ ] Test multi-service restart recovery

---

## SCORING: System Completeness

| Aspect | Score | Status |
|--------|-------|--------|
| Execution Map | 40% | Documented but gaps found |
| Data Lifecycle | 20% | Largely unknown |
| Error Recovery | 10% | Minimal/missing |
| Startup Validation | 5% | None visible |
| Shutdown Safety | 15% | Basic implementation |
| Observability | 25% | Partial (/metrics exists) |
| Security | 5% | Many holes |
| **OVERALL** | **16%** | **INCOMPLETE** |

---

## DECISION LOG

**Decision:** Treat all unknown assumptions as **BROKEN** until proven otherwise.

**Rationale:** Production systems cannot tolerate silent failures. Better to be paranoid than to deploy broken code.

---

*Generated by CARTOGRAPHER for YOLO FINISHER v3.0*
*Next: BREAKER chaos testing*

---

## ✅ Immediate Fixes Implemented (2026-01-20)

- **Robust startup validation**: `run-everything.sh` now performs strict checks and retries:
  - Verifies Ollama HTTP API reachability more reliably and logs detailed errors
  - Verifies Blockchain *JSON‑RPC* responds (uses `system_health` JSON-RPC probe)
  - Uses Swarm `/ready` readiness probe (not just `/health`) to ensure dependencies connected
  - Added `--strict` mode to fail-fast on critical startup errors (blockchain/swarm)
  - Improved graceful shutdown (SIGTERM then SIGKILL) and safer PID cleanup
  - Added better logging and exit codes for automation and CI

- **Persistent Agent Registry**: Added a lightweight SQLite persistence layer at `swarm/storage/sqlite_store.py` and integrated it into `swarm/telemetry/agent_registry.py`.
  - Persists births, deaths, and agent snapshots
  - Loads persisted agents at startup for reconciliation
  - Adds tests to verify persistence behavior (`swarm/storage/tests/test_sqlite_store.py`)

- **Startup Tests & CI**:
  - Added `tests/startup_smoke.sh` for local startup validation (Ollama, blockchain RPC, Swarm readiness)
  - Added a manual GitHub Action `Startup Smoke Check` (workflow_dispatch) to run smoke checks on-demand

---

## 🔜 Next recommended work (priority order)

1. **Data lifecycle & reconciliation (IN PROGRESS)** — Complete persistent schema for task/events, add migrations, and implement reconciliation logic on Swarm API startup (re-queue unfinished tasks, replay important events). ✅ *I can implement this next.*
2. **Security hardening** — Add authentication middleware, rate limiting, and remove `--rpc-methods Unsafe` from Substrate runs in non-dev environments.
3. **Observability & Alerts** — Expose Prometheus metrics for startup/readiness, add alerts and apps/apps/dash-legacy-2-legacy-2boards, and instrument more internal metrics.
4. **Chaos & Recovery tests** — Implement BREAKER test sfrontend/uite to exercise failures (Ollama/dns, node crash, corrupted DB) and add CI gating for recovery behavior.

---

If you'd like, I can open a PR now containing the changes for startup validation, persistence integration, and the smoke tests, and then continue with item #1 (persistence/reconciliation) next. Let me know which item to do next or if you want me to open the PR first.

