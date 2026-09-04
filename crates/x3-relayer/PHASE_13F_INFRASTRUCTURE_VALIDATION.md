# Phase 13f Infrastructure Validation Checklist

**Purpose:** Systematic verification that all systems are ready for launch  
**Timing:** T-10d (must complete before war game T-8d)  
**Owner:** Infrastructure Lead  
**Duration:** 2-3 hours  
**Success:** All items checked and green, ready for T-0h

---

## 1. Relayer Service (90 minutes)

### 1.1 Build & Compilation

**Objective:** Relayer binary compiles and runs without errors

**Owner:** [Name]  
**Due:** [Date]  
**Location:** `/home/lojak/Desktop/x3-chain-master/crates/relayer/`

**Checklist:**

- [ ] Clone mainnet branch: `git clone ... origin/mainnet`
- [ ] Checkout correct commit: `git checkout [commit-hash]`
- [ ] Build release binary: `cargo build --release`
  - [ ] Build completes without errors
  - [ ] Build completes without warnings (or document approved warnings)
  - [ ] Time to compile: _____ minutes
- [ ] Binary location verified: `target/release/x3-relayer`
- [ ] Binary is executable: `file target/release/x3-relayer`

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 1.2 Configuration

**Objective:** Mainnet configuration is valid and ready

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Mainnet config file exists: `relayer-config-mainnet.yaml`
- [ ] Config file is readable and valid YAML
- [ ] Run relayer with `--validate-config`:
  ```bash
  ./target/release/x3-relayer \
    --config relayer-config-mainnet.yaml \
    --validate-config
  ```
  - [ ] Validation passes without errors
- [ ] All required fields are present:
  - [ ] `ethereum_rpc_urls`
  - [ ] `solana_rpc_urls`
  - [ ] `x3_runtime_urls`
  - [ ] `relayer_account`
  - [ ] `bridge_pause_address`
  - [ ] `validator_account`
- [ ] All URLs are correct:
  - [ ] EVM RPC URLs are mainnet URLs (not testnet)
  - [ ] SVM RPC URLs are mainnet URLs (not testnet)
  - [ ] X3 runtime URLs are correct
- [ ] Sensitive data is not in config:
  - [ ] No private keys in plain text
  - [ ] API keys reference environment variables
  - [ ] No credentials in git history: `git log -p --all -- relayer-config-mainnet.yaml | grep -i "password\|secret\|key" | head -5` (should be empty)

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 1.3 Runtime Execution

**Objective:** Relayer binary runs and initializes correctly

**Owner:** [Name]  
**Due:** [Date]

**Test on staging environment:**

**Checklist:**

- [ ] Set environment variables:
  ```bash
  export RUST_LOG=debug
  export X3_RELAYER_ETHEREUM_KEY="[staging-key]"  # Not production!
  ```
- [ ] Start relayer:
  ```bash
  ./target/release/x3-relayer \
    --config relayer-config-mainnet.yaml \
    2>&1 | tee relayer-startup.log
  ```
- [ ] Verify startup sequence:
  - [ ] Configuration loaded: look for "Config loaded"
  - [ ] Connecting to EVM: look for "Connecting to Ethereum RPC"
  - [ ] Connecting to SVM: look for "Connecting to Solana RPC"
  - [ ] Connecting to X3: look for "Connecting to X3 runtime"
  - [ ] Relayer running: look for "Relayer started" or similar
- [ ] Relayer stays running for 5 minutes:
  - [ ] Start time: _____ UTC
  - [ ] Uptime check at T+5m: _____ (should be running)
- [ ] No errors in logs (only warnings are OK):
  - [ ] Log level ERROR: ____ count (should be 0)
  - [ ] Log level WARN: ____ count (should be low)
- [ ] Graceful shutdown:
  - [ ] Send SIGTERM: `kill -TERM [pid]`
  - [ ] Relayer shuts down cleanly (< 10 seconds)
  - [ ] No "panic" or "crash" messages in logs

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 1.4 Systemd Service

**Objective:** Systemd can manage relayer service

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Service file exists: `/etc/systemd/system/x3-relayer.service`
- [ ] Service file is readable:
  ```bash
  cat /etc/systemd/system/x3-relayer.service
  ```
- [ ] Service file contains required sections:
  - [ ] `[Unit]` (description, dependencies)
  - [ ] `[Service]` (type, user, exec, restart policy)
  - [ ] `[Install]` (wanted-by)
- [ ] Daemon reload succeeds:
  ```bash
  sudo systemctl daemon-reload
  ```
  - [ ] No errors
- [ ] Service can be enabled:
  ```bash
  sudo systemctl enable x3-relayer
  ```
  - [ ] No errors
- [ ] Service can be started:
  ```bash
  sudo systemctl start x3-relayer
  ```
  - [ ] Service starts successfully
  - [ ] Service shows as "active (running)": `sudo systemctl status x3-relayer`
- [ ] Service can be stopped:
  ```bash
  sudo systemctl stop x3-relayer
  ```
  - [ ] Service stops gracefully
  - [ ] Service shows as "inactive (dead)"
- [ ] Service respects restart policy:
  - [ ] Restart policy: `Restart=on-failure`
  - [ ] Restart delay: check `RestartSec=` (should be reasonable, e.g., 5-10s)
- [ ] Service auto-starts on reboot:
  - [ ] Check enabled status: `sudo systemctl is-enabled x3-relayer`
  - [ ] Should output: `enabled`

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 1.5 Logging

**Objective:** Logging is configured and working

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Log directory exists: `/var/log/x3-relayer/`
- [ ] Log file is being written:
  ```bash
  tail -f /var/log/x3-relayer/relayer.log
  ```
  - [ ] File exists
  - [ ] New entries appear when relayer runs
- [ ] Log rotation is configured:
  ```bash
  cat /etc/logrotate.d/x3-relayer
  ```
  - [ ] Rotation policy: daily or weekly
  - [ ] Retention: 30 days minimum
  - [ ] Compression: enabled
- [ ] Log format is parseable:
  - [ ] Logs contain: timestamp, level, message
  - [ ] Example: `[2026-04-21T10:30:45Z] INFO Block 100 finalized`
- [ ] Sensitive data is not in logs:
  - [ ] grep for credentials: `grep -i "password\|secret\|key\|token" /var/log/x3-relayer/relayer.log | head -5`
  - [ ] Should be empty or only reference IDs, not values

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

## 2. RPC Providers (60 minutes)

### 2.1 Ethereum RPC Providers

**Objective:** All EVM RPC endpoints are responding and healthy

**Owner:** [Name]  
**Due:** [Date]

**For each provider (Alchemy, Infura, QuickNode):**

**Provider: [Provider Name]**

**Checklist:**

- [ ] API key is secured:
  - [ ] Key stored in environment variable: `echo $ETH_[PROVIDER]_KEY`
  - [ ] Key is NOT in config files: `grep -r "[key-value]" . | grep -v ".git"`
  - [ ] Key is NOT in git history
- [ ] Connection test:
  ```bash
  curl -X POST https://[provider-endpoint] \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
    -H "Authorization: Bearer $ETH_[PROVIDER]_KEY"
  ```
  - [ ] Returns valid response (hex block number)
  - [ ] No authentication errors
  - [ ] No rate limit errors
- [ ] Latency test (make 5 requests, record times):
  ```bash
  for i in {1..5}; do
    time curl -X POST https://[provider-endpoint] ... > /dev/null
  done
  ```
  - [ ] Request 1: _____ ms
  - [ ] Request 2: _____ ms
  - [ ] Request 3: _____ ms
  - [ ] Request 4: _____ ms
  - [ ] Request 5: _____ ms
  - [ ] Average: _____ ms (target: < 500ms)
  - [ ] Max: _____ ms (target: < 2000ms)
- [ ] Block header retrieval:
  ```bash
  curl -X POST https://[provider-endpoint] \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest",false],"id":1}'
  ```
  - [ ] Returns latest block
  - [ ] Block number increases on subsequent calls
  - [ ] Block timestamp is recent (< 15 seconds old)
- [ ] Rate limits understood:
  - [ ] API plan: [Plan name]
  - [ ] Requests per second: _____ rps
  - [ ] Daily requests limit: _____ requests/day
  - [ ] Team understands plan: [Yes/No]
- [ ] Failover test (will test in failover exercise):
  - [ ] [ ] Can we switch to backup provider? (test later)

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

**For Alchemy:**
- [ ] Provider: Alchemy
- [ ] Endpoint: https://eth-mainnet.alchemyapi.io/v2/[API_KEY]
- [ ] Health: ✅ / ⚠️ / ❌

**For Infura:**
- [ ] Provider: Infura
- [ ] Endpoint: https://mainnet.infura.io/v3/[API_KEY]
- [ ] Health: ✅ / ⚠️ / ❌

**For QuickNode:**
- [ ] Provider: QuickNode
- [ ] Endpoint: https://[subdomain].quiknode.pro/[API_KEY]/
- [ ] Health: ✅ / ⚠️ / ❌

---

### 2.2 Solana RPC Providers

**Objective:** All SVM RPC endpoints are responding and healthy

**Owner:** [Name]  
**Due:** [Date]

**For each provider (QuickNode, Helius, Triton):**

**Provider: [Provider Name]**

**Checklist:**

- [ ] API key is secured (same as Ethereum above)
- [ ] Connection test:
  ```bash
  curl -X POST https://[provider-endpoint] \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}'
  ```
  - [ ] Returns current slot number
  - [ ] No authentication errors
- [ ] Latency test (make 5 requests):
  - [ ] Average latency: _____ ms (target: < 500ms)
- [ ] Slot retrieval:
  ```bash
  curl -X POST https://[provider-endpoint] \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}'
  ```
  - [ ] Returns latest slot
  - [ ] Slot increases on subsequent calls

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

**For QuickNode:**
- [ ] Provider: QuickNode (Solana)
- [ ] Endpoint: https://[subdomain].solana-mainnet.quiknode.pro/[API_KEY]/
- [ ] Health: ✅ / ⚠️ / ❌

**For Helius:**
- [ ] Provider: Helius
- [ ] Endpoint: https://mainnet.helius-rpc.com/?api-key=[API_KEY]
- [ ] Health: ✅ / ⚠️ / ❌

**For Triton:**
- [ ] Provider: Triton
- [ ] Endpoint: https://api.triton.one/core/rpc/mainnet
- [ ] Health: ✅ / ⚠️ / ❌

---

### 2.3 X3 Runtime Endpoints

**Objective:** X3 runtime is accessible and accepting connections

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

**Primary Endpoint:**
- [ ] Host: [IP/Hostname]
- [ ] Port: [Port]
- [ ] Connection test:
  ```bash
  timeout 5 nc -zv [host] [port]
  ```
  - [ ] Connection succeeds
- [ ] Runtime check (query proof acceptance):
  - [ ] Can submit test proof
  - [ ] Runtime accepts proof
  - [ ] No errors in logs

**Backup Endpoint:**
- [ ] Host: [IP/Hostname]
- [ ] Port: [Port]
- [ ] Connection test:
  - [ ] Connection succeeds
- [ ] Runtime check:
  - [ ] Can submit test proof
  - [ ] Runtime accepts proof

**Network Health:**
- [ ] Packet loss: _____ % (target: < 0.1%)
  ```bash
  ping -c 100 [host] | grep "packet loss"
  ```
- [ ] No timeouts observed

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

## 3. Monitoring & Alerting (45 minutes)

### 3.1 Prometheus

**Objective:** Prometheus is collecting all metrics

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Prometheus instance is running:
  ```bash
  curl http://localhost:9090/api/v1/query?query=up
  ```
  - [ ] Returns HTTP 200
  - [ ] Shows metrics
- [ ] All scrape targets are healthy:
  - [ ] Visit: http://localhost:9090/targets
  - [ ] Relayer target: ✅ UP
  - [ ] EVM provider targets: ✅ UP
  - [ ] SVM provider targets: ✅ UP
  - [ ] X3 runtime target: ✅ UP
- [ ] Metrics are being collected:
  - [ ] Query: `x3_blocks_polled` — should return non-zero
  - [ ] Query: `x3_proofs_submitted` — should return non-zero
  - [ ] Query: `up{job="relayer"}` — should return 1
- [ ] Data retention is configured:
  - [ ] Config location: `/etc/prometheus/prometheus.yml`
  - [ ] Retention period: _____ days (target: 30 days)
  - [ ] Check: `grep "retention" /etc/prometheus/prometheus.yml`
- [ ] Disk space is sufficient:
  ```bash
  df -h /var/lib/prometheus
  ```
  - [ ] Available space: _____ GB
  - [ ] Expected usage for 30 days: [estimate]
  - [ ] Status: Sufficient / Warning / Critical
- [ ] Database integrity:
  - [ ] No errors in Prometheus logs:
    ```bash
    tail -100 /var/log/prometheus/prometheus.log | grep -i "error"
    ```

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 3.2 Grafana

**Objective:** Grafana dashboards are displaying data

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Grafana instance is running:
  - [ ] Visit: http://localhost:3000
  - [ ] Login page appears
- [ ] Authentication is working:
  - [ ] Log in with admin credentials
  - [ ] Successfully authenticated
- [ ] Dashboards are created (5 dashboards):
  - [ ] Dashboard 1: Bridge Activity
    - [ ] Name: visible in sidebar
    - [ ] Panels load without errors
    - [ ] Charts show data (not empty)
  - [ ] Dashboard 2: RPC Provider Health
    - [ ] Name: visible in sidebar
    - [ ] Panels show provider status
  - [ ] Dashboard 3: Relayer Performance
    - [ ] Name: visible in sidebar
    - [ ] Shows block polling, proof submission
  - [ ] Dashboard 4: System Resources
    - [ ] Name: visible in sidebar
    - [ ] Shows CPU, memory, disk
  - [ ] Dashboard 5: Incident Response
    - [ ] Name: visible in sidebar
    - [ ] Shows incident timeline, logs
- [ ] Dashboard refresh is working:
  - [ ] Set auto-refresh to 30 seconds
  - [ ] Watch chart for 1 minute
  - [ ] Data should update every 30 seconds
- [ ] Datasource is connected:
  - [ ] Settings → Data sources
  - [ ] Prometheus datasource listed
  - [ ] Status: "Data source is working"

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 3.3 Alert Rules

**Objective:** Alerting is configured and working

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Alert rules are loaded:
  ```bash
  curl http://localhost:9090/api/v1/rules | grep alert
  ```
  - [ ] Returns list of alert rules
- [ ] Alert rules count: _____ (target: 10+)
- [ ] Critical alerts configured:
  - [ ] [ ] Relayer crashed
  - [ ] [ ] RPC provider down
  - [ ] [ ] X3 runtime not responding
  - [ ] [ ] High memory usage
  - [ ] [ ] Disk usage critical
- [ ] Alert destinations configured:
  - [ ] PagerDuty: ✅ Configured / ❌ Not configured
  - [ ] Slack: ✅ Configured / ❌ Not configured
  - [ ] Email: ✅ Configured / ❌ Not configured
  - [ ] SMS: ✅ Configured / ❌ Not configured
- [ ] Test alert fired successfully:
  - [ ] Manually trigger test alert in Prometheus
  - [ ] Alert notification received in Slack/email/etc.
  - [ ] Time from alert to notification: _____ seconds

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 3.4 PagerDuty Integration

**Objective:** On-call alerting is working

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] PagerDuty account exists
- [ ] Service is created: "[X3 Mainnet Launch]"
- [ ] Integration key configured in Prometheus:
  ```bash
  grep -i "pagerduty\|routing_key" /etc/prometheus/alertmanager.yml
  ```
  - [ ] Integration key is present (not the actual key, just verify it exists)
- [ ] On-call schedule created:
  - [ ] Primary on-call: [Name]
  - [ ] Secondary on-call: [Name]
  - [ ] Schedule covers 24/7: ✅ Yes / ❌ No
- [ ] Test incident:
  - [ ] Create test incident in PagerDuty
  - [ ] Primary on-call receives notification
  - [ ] Can acknowledge incident
  - [ ] Escalation works (if primary doesn't acknowledge)

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 3.5 Slack Integration

**Objective:** Slack notifications are working

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Slack workspace exists
- [ ] Channel `#mainnet-launch` created
- [ ] Webhook configured:
  - [ ] Webhook URL stored securely (not in config files)
  - [ ] Webhook is valid (test with curl)
- [ ] Test notification:
  ```bash
  curl -X POST [webhook-url] \
    -H 'Content-Type: application/json' \
    -d '{"text":"Test message from Prometheus"}'
  ```
  - [ ] Message appears in #mainnet-launch
  - [ ] Time to appearance: < 5 seconds
- [ ] Alert notification format:
  - [ ] Includes alert name
  - [ ] Includes severity level
  - [ ] Includes timestamp
  - [ ] Is readable and actionable

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

## 4. Security & Access Control (30 minutes)

### 4.1 SSH Access

**Objective:** Secure SSH access for team members

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] SSH keys are deployed for all team members:
  - [ ] Alice: SSH key installed
  - [ ] Bob: SSH key installed
  - [ ] Carol: SSH key installed
  - [ ] [Each team member]
- [ ] SSH authentication works:
  ```bash
  ssh -i ~/.ssh/id_rsa relayer-admin@[host] "whoami"
  ```
  - [ ] Returns username
  - [ ] No password prompt
- [ ] SSH security hardened:
  - [ ] PasswordAuthentication: disabled in `/etc/ssh/sshd_config`
  - [ ] PermitRootLogin: no
  - [ ] SSH port: not 22 (verify actual port in config)
- [ ] SSH key management:
  - [ ] Private keys: stored locally, not on server
  - [ ] Public keys: stored in `/home/relayer-admin/.ssh/authorized_keys`
  - [ ] No team member has shared keys

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 4.2 Sudo Access

**Objective:** Only authorized team members have sudo

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Sudo group exists
- [ ] Authorized members in sudo group:
  - [ ] Alice: sudo access
  - [ ] Bob: sudo access
  - [ ] [List of authorized members]
- [ ] Verify with: `grep "^sudo:" /etc/group`
- [ ] Sudo requires password for critical commands:
  - [ ] NOPASSWD entries: none for sensitive commands
  - [ ] Check: `grep "NOPASSWD" /etc/sudoers.d/*`
- [ ] Sudo audit logging enabled:
  - [ ] Sudo logs go to `/var/log/sudo.log` or syslog
  - [ ] Sample entry: [show example]

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 4.3 API Key Management

**Objective:** API keys are secured and not exposed

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] API keys are stored in environment variables:
  - [ ] `ETH_ALCHEMY_KEY` exported in `/etc/environment`
  - [ ] `ETH_INFURA_KEY` exported
  - [ ] `SOL_QUICKNODE_KEY` exported
  - [ ] All [N] keys present
- [ ] API keys are not in config files:
  - [ ] Grep for actual API keys: `grep -r "[actual-key-value]" /home/relayer-admin | wc -l`
  - [ ] Result: 0 (zero occurrences)
- [ ] API keys are not in git history:
  ```bash
  cd /home/relayer-admin/x3-chain && \
  git log -p --all | grep -i "alchemy\|infura\|quicknode" | head -5
  ```
  - [ ] Shows no sensitive values
- [ ] Environment variables are not visible to unprivileged users:
  - [ ] `env` as unprivileged user should NOT show API keys
- [ ] API key rotation schedule:
  - [ ] Rotation frequency: [Monthly/Quarterly/etc.]
  - [ ] Last rotation: [Date]
  - [ ] Next rotation: [Date]

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 4.4 Monitoring Access Control

**Objective:** Monitoring dashboards are behind authentication

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Grafana requires authentication:
  - [ ] Anonymous access: disabled
  - [ ] Check: Settings → Security
  - [ ] "Anonymous access" is OFF
- [ ] Prometheus requires authentication:
  - [ ] Behind reverse proxy (Nginx/Apache)
  - [ ] Requires login to access
  - [ ] Or: Restricted to internal network only
- [ ] Log access is restricted:
  - [ ] Log files: readable only by owner (chmod 600)
  - [ ] Log aggregation: behind authentication
  - [ ] Only authorized team members can view logs
- [ ] Access logs for monitoring:
  - [ ] Who accessed Grafana: [audit trail]
  - [ ] When: [audit trail]
  - [ ] What they viewed: [audit trail]

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

## 5. Disaster Recovery (30 minutes)

### 5.1 Database Backups (if applicable)

**Objective:** Database backups are working and tested

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Backup schedule configured:
  - [ ] Frequency: [Daily/Hourly/etc.]
  - [ ] Time: [UTC time]
  - [ ] Retention: [N] days
- [ ] Backups are being created:
  - [ ] Latest backup exists: [Location]
  - [ ] Last backup timestamp: _____ (should be < 24h old)
  - [ ] Backup size: _____ MB
- [ ] Backups are verified:
  - [ ] Backup file is readable
  - [ ] Backup file is not corrupted
  - [ ] Checksum verified (if applicable)
- [ ] Backups are encrypted:
  - [ ] Encryption method: [AES-256/other]
  - [ ] Encryption keys: stored securely
  - [ ] Keys are accessible to authorized personnel
- [ ] Backups are off-site:
  - [ ] Backup location: [S3/backup server/etc.]
  - [ ] Location is NOT on same server
  - [ ] Network connectivity to backup location: verified

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

### 5.2 Restore-from-Backup Test

**Objective:** We can restore from backup and recover

**Owner:** [Name]  
**Due:** [Date]

**Checklist:**

- [ ] Test environment prepared (staging)
- [ ] Restore procedure documented:
  - [ ] Location: [Document name/location]
  - [ ] Steps: [Number of steps]
  - [ ] Reviewed by: [Name]
- [ ] Restore test executed:
  - [ ] Download latest backup
  - [ ] Restore to staging database
  - [ ] Verify data integrity:
    - [ ] Row count: matches production
    - [ ] Spot check: [N] random rows match
  - [ ] Time to restore: _____ minutes (target: < 30 min)
- [ ] Recovery time objective (RTO):
  - [ ] Target: [Minutes]
  - [ ] Measured: _____ minutes
  - [ ] Status: ✅ Meets target / ⚠️ Close / ❌ Exceeds
- [ ] Recovery point objective (RPO):
  - [ ] Target: [Minutes]
  - [ ] Measured: _____ minutes of data loss
  - [ ] Status: ✅ Acceptable / ⚠️ Close / ❌ Unacceptable

**Result:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

**Notes:** _______________________________________________

---

## 6. Overall Infrastructure Sign-Off

**Summary:**

| Component | Status | Notes |
|-----------|--------|-------|
| Relayer Service | ✅ / ⚠️ / ❌ | [Note] |
| EVM RPC Providers | ✅ / ⚠️ / ❌ | [Note] |
| SVM RPC Providers | ✅ / ⚠️ / ❌ | [Note] |
| X3 Runtime | ✅ / ⚠️ / ❌ | [Note] |
| Prometheus | ✅ / ⚠️ / ❌ | [Note] |
| Grafana | ✅ / ⚠️ / ❌ | [Note] |
| Alerting | ✅ / ⚠️ / ❌ | [Note] |
| Security | ✅ / ⚠️ / ❌ | [Note] |
| Disaster Recovery | ✅ / ⚠️ / ❌ | [Note] |

**Overall Status:** ✅ READY / ⚠️ CAUTION / ❌ NOT READY

**Action Items (if any):**

- [ ] [Item] — Owner: [Name] — Due: [Date] — Priority: [1-5]
- [ ] [Item] — Owner: [Name] — Due: [Date] — Priority: [1-5]

**Final Sign-Off:**

> Infrastructure validation completed [Date].
>
> All systems verified and ready for mainnet launch.  
> No blocking issues identified.
>
> Status: ✅ READY FOR LAUNCH
>
> Validated by: [Infrastructure Lead]  
> Signature: ________________  
> Date: [Date]

---

**Document Version:** 1.0  
**Last Updated:** April 21, 2026  
**Status:** Ready for T-10d Execution
