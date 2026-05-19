# X3 Chain Testnet v1 - Deployment Checklist

**Use this checklist to track testnet deployment progress.**

---

## 📋 Pre-Deployment Phase

### Build & Testing
- [ ] Build release binary: `cargo build --release`
- [ ] Verify binary runs: `./target/release/x3-chain-node --version`
- [ ] Run all unit tests: `cargo test --all`
- [ ] Run integration tests: `./RUN_ALL_TESTS.sh`
- [ ] Test local node startup (dev mode): `./run-dev-node.sh`
- [ ] Verify RPC methods work locally (see `docs/reports/TESTNET_QUICKSTART.md`)

### Chain Specification
- [ ] Generate dev chain spec: `x3-chain-node build-spec --disable-default-bootnode --chain dev > x3-testnet-plain.json`
- [ ] Edit chain spec (name, id, bootnodes, validators)
- [ ] Convert to raw format: `x3-chain-node build-spec --chain x3-testnet-plain.json --raw > x3-testnet-raw.json`
- [ ] Validate JSON syntax: `jq . x3-testnet-raw.json`
- [ ] Commit chain spec to repository

### Infrastructure Preparation
- [ ] Provision 3-5 validator VMs (4GB RAM, 2 vCPU, 50GB SSD minimum)
- [ ] Provision 2+ RPC VMs (8GB RAM, 4 vCPU, 100GB SSD)
- [ ] Provision 1 bootnode VM (2GB RAM, 1 vCPU, 20GB SSD)
- [ ] Provision monitoring server (Prometheus + Grafana)
- [ ] Set up DNS records:
  - [ ] `rpc.testnet.x3-chain.io` → RPC load balancer
  - [ ] `rpc2.testnet.x3-chain.io` → Backup RPC node
  - [ ] `bootnode.testnet.x3-chain.io` → Bootnode IP
  - [ ] `faucet.testnet.x3-chain.io` → Faucet service
  - [ ] `metrics.testnet.x3-chain.io` → Grafana apps/dash-legacy-2-legacy-2board
- [ ] Configure firewall rules:
  - [ ] Allow 30333 (P2P) from all
  - [ ] Allow 9944 (RPC) from load balancer only
  - [ ] Allow 9615 (Metrics) from monitoring server only
  - [ ] Allow 22 (SSH) from admin IPs only

### Key Generation
- [ ] Generate validator keys (Aura + GRANDPA) for each validator
  - [ ] Validator 1: `subkey generate --scheme Sr25519` (Aura), `subkey generate --scheme Ed25519` (GRANDPA)
  - [ ] Validator 2: (repeat)
  - [ ] Validator 3: (repeat)
  - [ ] Validator 4: (optional)
  - [ ] Validator 5: (optional)
- [ ] Generate bootnode key: `x3-chain-node key generate-node-key --file /var/lib/x3/bootnode-key`
- [ ] Record all keys in secure vault (1Password, HashiCorp Vault)
- [ ] Generate sudo key for chain spec
- [ ] Share public keys with team (Aura SR25519 pubkeys, GRANDPA ED25519 pubkeys)

---

## 🚀 Deployment Phase

### Deploy Bootnode (First!)
- [ ] Copy binary to bootnode server: `/usr/local/bin/x3-chain-node`
- [ ] Copy node key: `/var/lib/x3/bootnode-key`
- [ ] Create systemd service: `/etc/systemd/system/x3-bootnode.service`
- [ ] Start bootnode: `systemctl start x3-bootnode`
- [ ] Verify bootnode running: `systemctl status x3-bootnode`
- [ ] Get bootnode peer ID from logs
- [ ] Confirm bootnode listening on port 30333: `netstat -tulnp | grep 30333`

### Deploy Validator Nodes (After Bootnode)
For each validator (repeat 3-5 times):
- [ ] Copy binary to validator server
- [ ] Copy chain spec: `/home/x3/x3-testnet-raw.json`
- [ ] Create data directory: `/home/x3/data`
- [ ] Create systemd service with `--validator` flag
- [ ] Start validator: `systemctl start x3-validator`
- [ ] Insert Aura key: 
  ```bash
  curl http://localhost:9944 -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["aura","<seed>","<pubkey>"]}'
  ```
- [ ] Insert GRANDPA key:
  ```bash
  curl http://localhost:9944 -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["gran","<seed>","<pubkey>"]}'
  ```
- [ ] Verify keys inserted: Check logs for "Loaded authority keys"
- [ ] Verify node syncing: `curl -X POST http://localhost:9944 -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'`
- [ ] Monitor logs: `journalctl -u x3-validator -f`

**Validator Details:**
- [ ] Validator 1: IP `______`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] Validator 2: IP `______`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] Validator 3: IP `______`, Aura Pubkey `______`, GRANDPA Pubkey `______`
- [ ] Validator 4: IP `______`, Aura Pubkey `______`, GRANDPA Pubkey `______` (optional)
- [ ] Validator 5: IP `______`, Aura Pubkey `______`, GRANDPA Pubkey `______` (optional)

### Deploy RPC Nodes (After Validators Running)
For each RPC node (repeat 2+ times):
- [ ] Copy binary to RPC server
- [ ] Copy chain spec: `/home/x3/x3-testnet-raw.json`
- [ ] Create data directory: `/home/x3/data`
- [ ] Create systemd service with `--rpc-external --rpc-methods Safe`
- [ ] Start RPC node: `systemctl start x3-rpc`
- [ ] Verify node syncing: `curl http://localhost:9944 -X POST -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'`
- [ ] Verify RPC accessible externally (from outside network)
- [ ] Monitor logs: `journalctl -u x3-rpc -f`

**RPC Node Details:**
- [ ] RPC 1: IP `______`, Public URL `http://rpc.testnet.x3-chain.io:9944`
- [ ] RPC 2: IP `______`, Public URL `http://rpc2.testnet.x3-chain.io:9944`

### Configure Load Balancer
- [ ] Set up Nginx/HAProxy for RPC load balancing
- [ ] Configure health checks (system_health endpoint)
- [ ] Set rate limiting: 1000 req/min per IP
- [ ] Enable CORS for web apps
- [ ] Test load balancer: `curl http://rpc.testnet.x3-chain.io:9944`
- [ ] Verify failover works (stop one RPC node, test again)

---

## 📊 Monitoring Phase

### Prometheus Setup
- [ ] Install Prometheus on monitoring server
- [ ] Configure scrape targets (all validator + RPC nodes on port 9615)
- [ ] Verify metrics collection: `http://prometheus:9090/targets`
- [ ] Set up alerts:
  - [ ] Node down alert
  - [ ] High memory usage (>80%)
  - [ ] High disk usage (>70%)
  - [ ] Slow block production (>10s)
  - [ ] Low peer count (<3)

### Grafana Dashboards
- [ ] Install Grafana on monitoring server
- [ ] Add Prometheus data source
- [ ] Import Substrate node apps/dash-legacy-2-legacy-2board
- [ ] Create custom X3 Kernel apps/dash-legacy-2-legacy-2board
- [ ] Configure alert notifications (Discord, Email, PagerDuty)
- [ ] Make apps/dash-legacy-2-legacy-2board public: `http://metrics.testnet.x3-chain.io`

### Health Checks
- [ ] Verify all nodes syncing: `system_health` on each node
- [ ] Verify block production: `chain_getBlock` returns recent blocks
- [ ] Verify finalization: Check GRANDPA finalizing blocks
- [ ] Verify peer discovery: All nodes have 5+ peers
- [ ] Test X3 Kernel RPC methods (see `docs/reports/TESTNET_QUICKSTART.md`)
- [ ] Monitor logs for errors/warnings

---

## 💰 Faucet Deployment

### Faucet Service
- [ ] Deploy faucet backend (Node.js/Python service)
- [ ] Configure faucet account with 10,000+ tATLAS
- [ ] Set rate limits: 100 tATLAS per request, 1 req/24h per address
- [ ] Add captcha (hCaptcha/reCAPTCHA)
- [ ] Deploy frontend: `https://faucet.testnet.x3-chain.io`
- [ ] Test faucet: Request tokens, verify balance increases
- [ ] Monitor faucet account balance (alert if <1000 tATLAS)

### Discord Bot (Optional)
- [ ] Deploy faucet Discord bot
- [ ] Configure bot permissions in Discord server
- [ ] Test `!faucet <address>` command
- [ ] Add cooldown tracking per Discord user

---

## 📢 Public Launch

### Documentation Review
- [ ] Review `docs/reports/TESTNET_ANNOUNCEMENT.md` for accuracy
- [ ] Update RPC endpoints with actual URLs
- [ ] Update faucet URL
- [ ] Update bootnode multiaddr
- [ ] Add actual validator count
- [ ] Add actual network stats (block height, uptime)

### Community Preparation
- [ ] Create Discord channels: #testnet-announcements, #testnet-support, #testnet-feedback
- [ ] Create Telegram group for testnet
- [ ] Prepare Twitter announcement thread
- [ ] Draft Medium/blog post with technical details
- [ ] Create quick start video tutorial (optional)

### Developer Resources
- [ ] Publish `docs/reports/TESTNET_QUICKSTART.md` to docs site
- [ ] Create Postman/Insomnia collection for RPC methods
- [ ] Write example scripts (Python, JavaScript) for common tasks
- [ ] Set up developer Discord office hours

### Launch Communications
- [ ] Post announcement to Discord (#testnet-announcements)
- [ ] Post announcement to Telegram
- [ ] Tweet launch announcement (pin to profile)
- [ ] Cross-post to Reddit (r/substrate, r/rust, etc.)
- [ ] Post to Hacker News (Show HN)
- [ ] Email newsletter to early supporters
- [ ] Update GitHub README with testnet badge

---

## ✅ Post-Launch Phase (First 24 Hours)

### Immediate Monitoring
- [ ] Monitor block production (every 5 minutes for first hour)
- [ ] Watch for validator errors/warnings
- [ ] Monitor RPC request patterns
- [ ] Check faucet request volume
- [ ] Respond to community questions in Discord/Telegram

### Performance Validation
- [ ] Verify block time averaging ~6 seconds
- [ ] Check finalization delay (<30 seconds typical)
- [ ] Monitor validator equivocation (should be zero)
- [ ] Check RPC response times (<200ms average)
- [ ] Verify no memory leaks (stable memory usage)

### Community Engagement
- [ ] Welcome first developers in Discord
- [ ] Help troubleshoot first faucet issues
- [ ] Collect initial feedback on RPC usability
- [ ] Document common issues → add to `docs/reports/TESTNET_QUICKSTART.md`
- [ ] Thank validators for running nodes

---

## 🔄 Week 1 Tasks

### Stability Monitoring
- [ ] Daily check on validator uptime (target: 99%+)
- [ ] Daily check on RPC node uptime (target: 99.9%+)
- [ ] Review Grafana apps/dash-legacy-2-legacy-2boards for anomalies
- [ ] Check disk usage growth rate
- [ ] Monitor network peer count (should grow daily)

### Developer Support
- [ ] Daily check of Discord #testnet-support
- [ ] Respond to GitHub issues within 24 hours
- [ ] Host first developer Q&A session
- [ ] Collect feature requests for v2
- [ ] Document first community-built apps

### Bug Tracking
- [ ] Create GitHub issues for reported bugs
- [ ] Prioritize critical issues (network halts, RPC failures)
- [ ] Hot-patch security issues immediately
- [ ] Plan non-critical fixes for v1.1 release

---

## 📈 Success Metrics (First Week)

Track these metrics daily:

- [ ] **Uptime**: Validator _____%, RPC _____% (target: >99%)
- [ ] **Block Height**: Current _______ (expected: ~14,400 blocks/day)
- [ ] **Active Developers**: _______ (target: 10+ in first week)
- [ ] **Faucet Requests**: _______ (indicates developer interest)
- [ ] **RPC Requests**: _______ per day (indicates active usage)
- [ ] **Community Growth**: Discord _______ members, Twitter _______ followers
- [ ] **GitHub Activity**: _______ stars, _______ issues, _______ PRs

---

## 🚨 Incident Response

### If Network Halts (No New Blocks)
1. [ ] Check all validators online: `systemctl status x3-validator`
2. [ ] Check validator logs for errors: `journalctl -u x3-validator -n 100`
3. [ ] Verify GRANDPA finality not stalled
4. [ ] If 1+ validators down, restart them immediately
5. [ ] If all validators up but no blocks, check for consensus bug → contact core team
6. [ ] Post status update to Discord within 15 minutes

### If RPC Node Down
1. [ ] Verify load balancer redirecting to backup RPC
2. [ ] Check down node: `systemctl status x3-rpc`
3. [ ] Restart if needed: `systemctl restart x3-rpc`
4. [ ] Check disk space: `df -h`
5. [ ] Check logs: `journalctl -u x3-rpc -n 100`
6. [ ] If persistent issue, provision emergency RPC node

### If Faucet Exploited
1. [ ] Pause faucet service immediately
2. [ ] Review faucet transaction logs
3. [ ] Identify exploit pattern (rate limit bypass, Sybil attack)
4. [ ] Patch faucet code
5. [ ] Deploy new faucet instance
6. [ ] Refill faucet account if drained
7. [ ] Post-mortem report to Discord

---

## 📝 Notes & Lessons Learned

**Team Notes:**
- _________________________________________
- _________________________________________
- _________________________________________

**Issues Encountered:**
- _________________________________________
- _________________________________________
- _________________________________________

**What Went Well:**
- _________________________________________
- _________________________________________
- _________________________________________

**What to Improve for v2:**
- _________________________________________
- _________________________________________
- _________________________________________

---

## ✅ Sign-Off

**Deployment Team:**
- [ ] Lead Engineer: ______________ (Date: ______)
- [ ] DevOps Engineer: ______________ (Date: ______)
- [ ] QA Engineer: ______________ (Date: ______)
- [ ] Community Manager: ______________ (Date: ______)

**Deployment Date:** _______________  
**Network Launch Block:** _______________  
**Status:** [ ] Pre-Launch  [ ] Live  [ ] Post-Launch  

---

**🎉 Congratulations on launching X3 Chain Testnet v1! 🎉**
