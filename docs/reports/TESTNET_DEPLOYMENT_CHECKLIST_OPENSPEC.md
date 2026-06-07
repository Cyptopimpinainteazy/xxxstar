# X3 Chain Testnet v1 - OpenSpec Deployment Checklist

**OpenSpec Change ID: osc-testnet-0001**  
**Use this checklist to track OpenSpec-compliant testnet deployment progress.**

---

## 📋 OpenSpec Pre-Deployment Phase

### OpenSpec Validation
- [ ] 1.1 Validate OpenSpec proposal: `openspec validate osc-testnet-0001 --strict`
- [ ] 1.2 Review delta specifications in `openspec/changes/osc-testnet-0001/specs/testnet-deployment/spec.md`
- [ ] 1.3 Confirm all scenarios have GIVEN-WHEN-THEN format
- [ ] 1.4 Ensure stakeholder approval for change proposal

### Build & Testing (OpenSpec Compliant)
- [ ] 2.1 Build release binary: `cargo build --release`
- [ ] 2.2 Verify binary runs: `./target/release/x3-chain-node --version`
- [ ] 2.3 Run all unit tests: `cargo test --all`
- [ ] 2.4 Run integration tests: `./RUN_ALL_TESTS.sh`
- [ ] 2.5 Test local node startup (dev mode): `./run-dev-node.sh`
- [ ] 2.6 Verify RPC methods work locally (see `docs/reports/TESTNET_QUICKSTART.md`)
- [ ] 2.7 Test X3 language runtime: Execute examples from `x3-lang/examples/`

### Chain Specification (OpenSpec Enhanced)
- [ ] 3.1 Generate dev chain spec: `x3-chain-node build-spec --disable-default-bootnode --chain dev > x3-testnet-plain.json`
- [ ] 3.2 Edit chain spec (name, id, bootnodes, validators)
- [ ] 3.3 Convert to raw format: `x3-chain-node build-spec --chain x3-testnet-plain.json --raw > x3-testnet-raw.json`
- [ ] 3.4 Validate JSON syntax: `jq . x3-testnet-raw.json`
- [ ] 3.5 Commit chain spec to repository
- [ ] 3.6 Document chain spec changes in OpenSpec

### X3 DNS Server Integration (NEW)
- [ ] 4.1 Configure X3 DNS server for testnet domains:
  - [ ] 4.1.1 `rpc.testnet.x3` → RPC load balancer IP
  - [ ] 4.1.2 `rpc2.testnet.x3` → Backup RPC node IP
  - [ ] 4.1.3 `bootnode.testnet.x3` → Bootnode IP
  - [ ] 4.1.4 `faucet.testnet.x3` → Faucet service IP
  - [ ] 4.1.5 `metrics.testnet.x3` → Grafana apps/dash-legacy-2-legacy-2board IP
- [ ] 4.2 Test DNS resolution: `dig rpc.testnet.x3`
- [ ] 4.3 Verify DNS server health: `cargo run --bin x3-dns-server`
- [ ] 4.4 Update DNS server configuration with testnet zones

### Infrastructure Preparation (Enhanced)
- [ ] 5.1 Provision 3-5 validator VMs (4GB RAM, 2 vCPU, 50GB SSD minimum)
- [ ] 5.2 Provision 2+ RPC VMs (8GB RAM, 4 vCPU, 100GB SSD)
- [ ] 5.3 Provision 1 bootnode VM (2GB RAM, 1 vCPU, 20GB SSD)
- [ ] 5.4 Provision monitoring server (Prometheus + Grafana)
- [ ] 5.5 Set up DNS records (using X3 DNS server)
- [ ] 5.6 Configure firewall rules:
  - [ ] 5.6.1 Allow 30333 (P2P) from all
  - [ ] 5.6.2 Allow 9944 (RPC) from load balancer only
  - [ ] 5.6.3 Allow 9615 (Metrics) from monitoring server only
  - [ ] 5.6.4 Allow 22 (SSH) from admin IPs only

### Key Generation (OpenSpec Compliant)
- [ ] 6.1 Generate validator keys (Aura + GRANDPA) for each validator
  - [ ] 6.1.1 Validator 1: `subkey generate --scheme Sr25519` (Aura), `subkey generate --scheme Ed25519` (GRANDPA)
  - [ ] 6.1.2 Validator 2: (repeat)
  - [ ] 6.1.3 Validator 3: (repeat)
  - [ ] 6.1.4 Validator 4: (optional)
  - [ ] 6.1.5 Validator 5: (optional)
- [ ] 6.2 Generate bootnode key: `x3-chain-node key generate-node-key --file /var/lib/x3/bootnode-key`
- [ ] 6.3 Record all keys in secure vault (1Password, HashiCorp Vault)
- [ ] 6.4 Generate sudo key for chain spec
- [ ] 6.5 Share public keys with team (Aura SR25519 pubkeys, GRANDPA ED25519 pubkeys)
- [ ] 6.6 Document key management in OpenSpec change

---

## 🚀 OpenSpec Deployment Phase

### Deploy Bootnode (Enhanced with DNS)
- [ ] 7.1 Copy binary to bootnode server: `/usr/local/bin/x3-chain-node`
- [ ] 7.2 Copy node key: `/var/lib/x3/bootnode-key`
- [ ] 7.3 Create systemd service: `/etc/systemd/system/x3-bootnode.service`
- [ ] 7.4 Start bootnode: `systemctl start x3-bootnode`
- [ ] 7.5 Verify bootnode running: `systemctl status x3-bootnode`
- [ ] 7.6 Get bootnode peer ID from logs
- [ ] 7.7 Confirm bootnode listening on port 30333: `netstat -tulnp | grep 30333`
- [ ] 7.8 Update X3 DNS server with bootnode.testnet.x3 record
- [ ] 7.9 Test bootnode DNS resolution: `dig bootnode.testnet.x3`

### Deploy Validator Nodes (After Bootnode - OpenSpec Enhanced)
For each validator (repeat 3-5 times):
- [ ] 8.1 Copy binary to validator server
- [ ] 8.2 Copy chain spec: `/home/x3/x3-testnet-raw.json`
- [ ] 8.3 Create data directory: `/home/x3/data`
- [ ] 8.4 Create systemd service with `--validator` flag
- [ ] 8.5 Start validator: `systemctl start x3-validator`
- [ ] 8.6 Insert Aura key: 
  ```bash
  curl http://localhost:9944 -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["aura","<seed>","<pubkey>"]}'
  ```
- [ ] 8.7 Insert GRANDPA key:
  ```bash
  curl http://localhost:9944 -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["gran","<seed>","<pubkey>"]}'
  ```
- [ ] 8.8 Verify keys inserted: Check logs for "Loaded authority keys"
- [ ] 8.9 Verify node syncing: `curl -X POST http://localhost:9944 -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'`
- [ ] 8.10 Monitor logs: `journalctl -u x3-validator -f`

**Validator Details (Enhanced with DNS):**
- [ ] 8.11 Validator 1: IP `______`, DNS: `validator1.testnet.x3`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] 8.12 Validator 2: IP `______`, DNS: `validator2.testnet.x3`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] 8.13 Validator 3: IP `______`, DNS: `validator3.testnet.x3`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] 8.14 Validator 4: IP `______`, DNS: `validator4.testnet.x3`, Aura Pubkey `______`, GRANDPA Pubkey `______` (optional)
- [ ] 8.15 Validator 5: IP `______`, DNS: `validator5.testnet.x3`, Aura Pubkey `______`, GRANDPA Pubkey `______` (optional)

### Deploy RPC Nodes (After Validators Running - OpenSpec Enhanced)
For each RPC node (repeat 2+ times):
- [ ] 9.1 Copy binary to RPC server
- [ ] 9.2 Copy chain spec: `/home/x3/x3-testnet-raw.json`
- [ ] 9.3 Create data directory: `/home/x3/data`
- [ ] 9.4 Create systemd service with `--rpc-external --rpc-methods Safe`
- [ ] 9.5 Start RPC node: `systemctl start x3-rpc`
- [ ] 9.6 Verify node syncing: `curl http://localhost:9944 -X POST -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'`
- [ ] 9.7 Verify RPC accessible externally (from outside network)
- [ ] 9.8 Monitor logs: `journalctl -u x3-rpc -f`

**RPC Node Details (Enhanced with DNS):**
- [ ] 9.9 RPC 1: IP `______`, DNS: `rpc.testnet.x3`, Public URL `http://rpc.testnet.x3:9944`
- [ ] 9.10 RPC 2: IP `______`, DNS: `rpc2.testnet.x3`, Public URL `http://rpc2.testnet.x3:9944`

### Configure Load Balancer (Enhanced)
- [ ] 10.1 Set up Nginx/HAProxy for RPC load balancing
- [ ] 10.2 Configure health checks (system_health endpoint)
- [ ] 10.3 Set rate limiting: 1000 req/min per IP
- [ ] 10.4 Enable CORS for web apps
- [ ] 10.5 Test load balancer: `curl http://rpc.testnet.x3:9944`
- [ ] 10.6 Verify failover works (stop one RPC node, test again)
- [ ] 10.7 Document load balancer configuration in OpenSpec

---

## 📊 OpenSpec Monitoring Phase

### Prometheus Setup (Enhanced)
- [ ] 11.1 Install Prometheus on monitoring server
- [ ] 11.2 Configure scrape targets (all validator + RPC nodes on port 9615)
- [ ] 11.3 Verify metrics collection: `http://prometheus:9090/targets`
- [ ] 11.4 Set up alerts:
  - [ ] 11.4.1 Node down alert
  - [ ] 11.4.2 High memory usage (>80%)
  - [ ] 11.4.3 High disk usage (>70%)
  - [ ] 11.4.4 Slow block production (>10s)
  - [ ] 11.4.5 Low peer count (<3)
  - [ ] 11.4.6 DNS resolution failures
  - [ ] 11.4.7 X3 DNS server health

### Grafana Dashboards (Enhanced)
- [ ] 12.1 Install Grafana on monitoring server
- [ ] 12.2 Add Prometheus data source
- [ ] 12.3 Import Substrate node apps/dash-legacy-2-legacy-2board
- [ ] 12.4 Create custom X3 Kernel apps/dash-legacy-2-legacy-2board
- [ ] 12.5 Create DNS server monitoring apps/dash-legacy-2-legacy-2board
- [ ] 12.6 Configure alert notifications (Discord, Email, PagerDuty)
- [ ] 12.7 Make apps/dash-legacy-2-legacy-2board public: `http://metrics.testnet.x3`

### GPU Swarm Integration (NEW - OpenSpec Enhanced)
- [ ] 13.1 Deploy GPU swarm coordinator: `cargo run --bin swarm-coordinator`
- [ ] 13.2 Deploy GPU swarm nodes: `cargo run --bin swarm-node`
- [ ] 13.3 Verify GPU node registration: Check coordinator logs
- [ ] 13.4 Test GPU job submission: Submit sample compute job
- [ ] 13.5 Monitor GPU swarm metrics: Track job completion rates
- [ ] 13.6 Integrate GPU swarm with X3 Chain validators
- [ ] 13.7 Document GPU swarm integration in OpenSpec

### Health Checks (OpenSpec Enhanced)
- [ ] 14.1 Verify all nodes syncing: `system_health` on each node
- [ ] 14.2 Verify block production: `chain_getBlock` returns recent blocks
- [ ] 14.3 Verify finalization: Check GRANDPA finalizing blocks
- [ ] 14.4 Verify peer discovery: All nodes have 5+ peers
- [ ] 14.5 Test X3 Kernel RPC methods (see `docs/reports/TESTNET_QUICKSTART.md`)
- [ ] 14.6 Monitor logs for errors/warnings
- [ ] 14.7 Test DNS resolution for all testnet domains
- [ ] 14.8 Verify X3 DNS server health: `curl http://localhost:8080/health`
- [ ] 14.9 Test GPU swarm connectivity and job routing

---

## 💰 OpenSpec Faucet Deployment

### Faucet Service (Enhanced)
- [ ] 15.1 Deploy faucet backend (Node.js/Python service)
- [ ] 15.2 Configure faucet account with 10,000+ tATLAS
- [ ] 15.3 Set rate limits: 100 tATLAS per request, 1 req/24h per address
- [ ] 15.4 Add captcha (hCaptcha/reCAPTCHA)
- [ ] 15.5 Deploy frontend: `https://faucet.testnet.x3`
- [ ] 15.6 Test faucet: Request tokens, verify balance increases
- [ ] 15.7 Monitor faucet account balance (alert if <1000 tATLAS)
- [ ] 15.8 Integrate faucet with X3 DNS server
- [ ] 15.9 Document faucet security measures in OpenSpec

### Discord Bot (Optional - Enhanced)
- [ ] 16.1 Deploy faucet Discord bot
- [ ] 16.2 Configure bot permissions in Discord server
- [ ] 16.3 Test `!faucet <address>` command
- [ ] 16.4 Add cooldown tracking per Discord user
- [ ] 16.5 Monitor Discord bot metrics and abuse attempts

---

## 🧪 X3 Language Runtime Testing (NEW - OpenSpec Enhanced)

### X3 Runtime Validation
- [ ] 17.1 Test X3 compiler: `cargo build --bin x3c`
- [ ] 17.2 Execute sample scripts:
  - [ ] 17.2.1 `jit_lp.x3` - Linear programming optimization
  - [ ] 17.2.2 `mev_smooth.x3` - MEV detection algorithm
  - [ ] 17.2.3 `flash.x3` - Flash loan simulation
  - [ ] 17.2.4 `arb.x3` - Arbitrage detection
- [ ] 17.3 Verify bytecode compilation and execution
- [ ] 17.4 Test deterministic execution across VM environments
- [ ] 17.5 Validate cross-VM integration (EVM + SVM)
- [ ] 17.6 Document X3 runtime testing in OpenSpec

### X3 REPL Testing
- [ ] 18.1 Launch X3 REPL: `cargo run --bin x3c -- repl`
- [ ] 18.2 Test basic arithmetic and logic operations
- [ ] 18.3 Test blockchain integration calls
- [ ] 18.4 Test cross-VM atomic operations
- [ ] 18.5 Verify deterministic execution results

---

## 📢 OpenSpec Public Launch

### Documentation Review (OpenSpec Compliant)
- [ ] 19.1 Review `docs/reports/TESTNET_ANNOUNCEMENT.md` for accuracy
- [ ] 19.2 Update RPC endpoints with DNS URLs
- [ ] 19.3 Update faucet URL: `https://faucet.testnet.x3`
- [ ] 19.4 Update bootnode multiaddr with DNS
- [ ] 19.5 Add actual validator count
- [ ] 19.6 Add actual network stats (block height, uptime)
- [ ] 19.7 Include OpenSpec change documentation
- [ ] 19.8 Document DNS server integration

### Community Preparation (Enhanced)
- [ ] 20.1 Create Discord channels: #testnet-announcements, #testnet-support, #testnet-feedback
- [ ] 20.2 Create Telegram group for testnet
- [ ] 20.3 Prepare Twitter announcement thread
- [ ] 20.4 Draft Medium/blog post with technical details
- [ ] 20.5 Create quick start video tutorial (optional)
- [ ] 20.6 Highlight OpenSpec-driven development process
- [ ] 20.7 Showcase X3 DNS server and GPU swarm features

### Developer Resources (OpenSpec Enhanced)
- [ ] 21.1 Publish `docs/reports/TESTNET_QUICKSTART.md` to docs site
- [ ] 21.2 Create Postman/Insomnia collection for RPC methods
- [ ] 21.3 Write example scripts (Python, JavaScript) for common tasks
- [ ] 21.4 Create X3 language tutorial and examples
- [ ] 21.5 Document X3 DNS server API
- [ ] 21.6 Provide GPU swarm integration examples
- [ ] 21.7 Set up developer Discord
