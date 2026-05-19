# X3 Mainnet Disaster Recovery Runbooks

**Classification**: CRITICAL OPERATIONAL PROCEDURES  
**Last Updated**: 2026-04-26  
**Owned By**: X3 Chain Operations  

---

## Index

1. **Chain Stall** (blocks stop producing)
2. **Finality Stall** (blocks produce but don't finalize)
3. **Validator Equivocation Detected** (validator misbehavior)
4. **Bad Runtime Upgrade** (invalid code deployed)
5. **Bridge Exploit Detected** (cross-chain attack)
6. **RPC Outage** (node API down)
7. **Database Corruption** (storage integrity lost)
8. **Validator Key Compromise** (secret key stolen)
9. **Governance Attack** (malicious vote passes)
10. **DEX Reserve Anomaly** (liquidity pool imbalance)

---

## 1. Chain Stall Recovery

**Symptoms:**
- Block production halts (no blocks for > 2 minutes)
- Network reports 0 peers or critical peer loss
- `curl localhost:9944 -d '{"jsonrpc":"2.0","method":"chain_getHeader","id":1}'` returns stale block number

**First 5 Minutes:**

```bash
# 1. Confirm the stall
curl -s localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' | jq .

# 2. Check validator status
curl -s localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","id":1}' | jq .

# 3. Get peer count
curl -s localhost:9944 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_syncState","id":1}' | jq .

# 4. Check logs for panic or disconnect
tail -100 /var/log/x3-chain-node.log | grep -i "error\|panic\|disconnect"
```

**Who Acts:**
- On-call validator operator
- Escalate to chain operations after 5 min

**What Gets Paused:**
- All validator upgrades (freeze the validator set)
- New bridge deposits (can pause EVM/SVM bridges)
- Large DEX trades (if DEX is also paused)

**Recovery Steps:**

1. **Check peer connectivity**
   ```bash
   # Try to manually connect to a bootnode
   x3-chain-node --chain mainnet \
     --bootnodes /dns/bootnode.x3.chain/tcp/30333/p2p/... \
     --rpc-port 9945 &
   ```

2. **If network is partitioned:** Coordinate with all validators to restart in known-good state
   ```bash
   # Graceful shutdown
   killall -TERM x3-chain-node
   sleep 3
   
   # Clear bad state (if suspected)
   rm -rf ~/.local/share/x3-chain/chains/x3-mainnet/db
   
   # Restart
   x3-chain-node --chain mainnet --rpc-port 9945 &
   ```

3. **If validators offline:** Contact all validators via out-of-band (phone, Discord)
   - Confirm their nodes are responsive
   - Instruct restart with current chain spec

4. **Monitor recovery**
   ```bash
   # Watch block height increase
   watch -n 2 'curl -s localhost:9944 -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"id\":1}" | jq .result.number'
   ```

5. **Post-mortem (after recovery)**
   - Collect logs from all validators
   - Identify root cause
   - Implement fix
   - Test before merging

**Recovery Time:** 5-30 min (depending on root cause)

**Escalation Path:**
- 5 min: Page on-call
- 10 min: Page chain lead
- 15 min: Activate incident command

---

## 2. Finality Stall Recovery

**Symptoms:**
- Blocks produce normally but `finalized_block_hash` doesn't advance
- `chain_subscribeFinalizedHeads` event stream stops
- Latest finalized block is 100+ blocks old

**First 5 Minutes:**

```bash
# 1. Confirm finality stalled
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","id":1}' | jq .

# 2. Compare to best block
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"chain_getHeader","id":1}' | jq '.result.number'

# 3. Check GRANDPA logs
journalctl -u x3-chain-node -n 50 | grep -i "grandpa\|finality\|vote"
```

**Who Acts:**
- Validator lead
- Chain operations

**What Gets Paused:**
- Governance votes (can't finalize)
- Bridge withdrawals (require finality)
- Treasury spending (deferred until finality recovers)

**Recovery Steps:**

1. **Identify stalled validator**
   ```bash
   # Get current GRANDPA voters
   curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_call","params":["AuthoritiesRpc_authorities",null],"id":1}' | jq .
   ```

2. **Check which validators are online**
   - Query each validator's RPC separately
   - At least 2/3 must be online

3. **If fewer than 2/3 validators online:**
   - Contact validators, instruct restart
   - Wait for 2/3 quorum

4. **If all validators online but finality stalled:**
   - Likely bug in GRANDPA consensus
   - Restart all validators simultaneously:
     ```bash
     # Coordinate via #validators channel
     killall -TERM x3-chain-node
     sleep 5
     x3-chain-node --chain mainnet --validator &
     ```

5. **Monitor finality recovery**
   ```bash
   watch -n 1 'curl -s localhost:9944 -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getFinalizedHead\",\"id\":1}" | jq .result'
   ```

**Recovery Time:** 1-10 min

**Escalation Path:**
- 5 min: Page validator lead
- 10 min: Activate war room call

---

## 3. Validator Equivocation Detected

**Symptoms:**
- Slashing event in logs: `equivocation detected`
- Validator that signed two different blocks at same height reported
- Stake reduced by slashing amount

**First 5 Minutes:**

```bash
# 1. Check slashing event
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0x1234..."],"id":1}' # query slashing pallet

# 2. Identify equivocating validator
journalctl -u x3-chain-node | grep -i "equivocation\|double_vote"

# 3. Get validator info
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_call","params":["ValidatorsApi_validators",null],"id":1}' | jq .
```

**Who Acts:**
- Network security officer
- Validator whose key is compromised

**What Gets Paused:**
- Affected validator removed from set
- Gateway to emergency halt if > 2 validators equivocate

**Recovery Steps:**

1. **Immediately contact affected validator**
   - Out of band (phone)
   - Confirm key is compromised
   - Instruct key rotation

2. **Verify slashing was applied**
   ```bash
   # Check slashing amount
   curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["pallet_staking slashing_spans VALIDATOR_ID"],"id":1}' | jq .
   ```

3. **Validator regenerates new keys**
   - Use secure key generation procedure
   - Submit `set_keys` transaction with new session keys

4. **Return validator to active set**
   - Governance vote to add validator back (if needed)
   - Or automatic reactivation at next era

5. **Post-mortem**
   - Investigate how key was compromised
   - Check validator machine for malware/intrusion
   - Rotate all secrets

**Recovery Time:** 30 min - 24 hours (validator operator dependent)

**Escalation Path:**
- Immediate: Page network security
- 10 min: Incident command
- If > 2 validators equivocate: Activate emergency halt procedure

---

## 4. Bad Runtime Upgrade Recovery

**Symptoms:**
- Node crashes immediately after runtime upgrade
- `panic: unable to decode block` errors
- New runtime fails to compile state

**First 5 Minutes:**

```bash
# 1. Roll back immediately
killall -TERM x3-chain-node
# Clear WASM cache
rm -rf ~/.local/share/x3-chain/chains/x3-mainnet/db/wasm_runtime_cache
# Restart with previous binary
/path/to/previous-binary/x3-chain-node --chain mainnet &

# 2. Verify recovery
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"chain_getHeader","id":1}' | jq .
```

**Who Acts:**
- Chain lead
- All validators (coordinated)

**What Gets Paused:**
- No new extrinsics accepted (runtime in broken state)
- Governance disabled until recovery

**Recovery Steps:**

1. **All validators revert to previous binary**
   ```bash
   # Coordinate in #validators channel
   # All validators:
   pkill x3-chain-node
   cp /backup/previous-binary ~/x3-chain-node
   ~/x3-chain-node --chain mainnet &
   ```

2. **Verify chain recovers**
   - Wait 30 seconds
   - Check if blocks resume

3. **Identify the bad runtime**
   - What was deployed?
   - Check git diff: `git diff v123..v124`
   - Find failing code path

4. **Fix and test locally**
   ```bash
   # Fix the code
   # Test: cargo test --all
   # Build release: cargo build -p x3-chain-node --release
   # Test on testnet first!
   ```

5. **Governance vote for new upgrade**
   - Proposed via governance
   - Voting period: 24-72 hours
   - Careful testing required

**Recovery Time:** 5-10 min (revert), 24-72 hours (fix and re-upgrade)

**Prevention:**
- Always test runtime upgrades on testnet
- Have N-1 binary available
- Practice upgrade procedure monthly

---

## 5. Bridge Exploit Detected

**Symptoms:**
- Unusually large EVM/SVM transfer across bridge
- Bridge reserve drops unexpectedly
- `bridge_exploit` event in logs

**First 5 Minutes:**

```bash
# 1. Get bridge state
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_call","params":["BridgeRpc_get_reserve",null],"id":1}' | jq .

# 2. Get recent bridge txs
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0xbridge..."],"id":1}' | jq .

# 3. Check for malicious signatures
journalctl -u x3-chain-node | grep -i "bridge\|signature.*invalid\|verification.*failed"
```

**Who Acts:**
- Bridge security team
- Chain lead

**What Gets Paused:**
- PAUSE THE BRIDGE IMMEDIATELY
- No new cross-chain transfers
- Freeze all bridge pallets

**Recovery Steps:**

1. **Pause the bridge**
   ```bash
   # Via governance emergency call or sudo
   x3-chain-node submit-extrinsic \
     bridge::pause_bridge \
     --sudo \
     --rpc-port 9944
   ```

2. **Investigate the exploit**
   - What was the attack?
   - How was it possible?
   - Review bridge pallet code

3. **Determine funds recovery**
   - Are stolen funds recoverable?
   - Is it just a state corruption?

4. **Implement fix**
   - Patch the vulnerability
   - Test extensively
   - Governance vote if necessary

5. **Reactivate bridge**
   - Only after fix is verified
   - Gradual rollout (small transfers first)

**Recovery Time:** 1-48 hours (depends on exploit severity)

**Escalation Path:**
- Immediate: Page bridge team
- 5 min: Incident command
- 15 min: Legal & communications

---

## 6. RPC Node Outage

**Symptoms:**
- `curl localhost:9944` returns connection refused
- RPC port not responding (9944/9945)
- Wallet/app cannot connect

**First 5 Minutes:**

```bash
# 1. Check if node is running
ps aux | grep x3-chain-node

# 2. Check if port is listening
netstat -tuln | grep 9944

# 3. Try to restart
systemctl restart x3-chain-node

# 4. Check logs
journalctl -u x3-chain-node -n 100 | tail
```

**Who Acts:**
- RPC node operator
- On-call DevOps

**What Gets Paused:**
- User transactions (can't submit extrinsics)
- Wallet/dApp functionality
- Analytics & indexing

**Recovery Steps:**

1. **Check system resources**
   ```bash
   df -h        # disk space
   free -h      # memory
   top -b -n 1  # CPU
   ```

2. **If OOM (out of memory):**
   ```bash
   # Graceful shutdown
   killall -TERM x3-chain-node
   sleep 3
   
   # Prune the database
   x3-chain-node purge-chain --chain mainnet
   
   # Restart
   x3-chain-node --chain mainnet --rpc-port 9944 &
   ```

3. **If disk full:**
   ```bash
   # Remove old chain data
   rm -rf ~/.local/share/x3-chain/chains/x3-mainnet/db/full
   
   # Restart
   x3-chain-node --chain mainnet --rpc-port 9944 &
   ```

4. **If logs show panic:**
   - Investigate panic message
   - Check if it's a known issue
   - Potentially revert binary if recent upgrade caused it

5. **Verify RPC responds**
   ```bash
   curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"system_health","id":1}' | jq .
   ```

**Recovery Time:** 2-15 min

**Prevention:**
- Run multiple RPC nodes with load balancing
- Set up monitoring for disk/memory
- Auto-restart on crash (systemd)

---

## 7. Database Corruption

**Symptoms:**
- `error: corrupted trie node`
- `error: invalid block: state root mismatch`
- Node won't sync past certain block

**First 5 Minutes:**

```bash
# 1. Confirm corruption
journalctl -u x3-chain-node | grep "corrupt\|invalid\|mismatch"

# 2. Try reindex
killall x3-chain-node
x3-chain-node --chain mainnet --reclaim-space archive &
```

**Who Acts:**
- Database/infrastructure team
- Chain operations lead

**What Gets Paused:**
- All users of this node
- Affected RPC endpoints

**Recovery Steps:**

1. **Backup the corrupted database**
   ```bash
   tar -czf corrupted-db-backup.tar.gz ~/.local/share/x3-chain/chains/x3-mainnet/db/
   ```

2. **Try automated repair**
   ```bash
   x3-chain-node --chain mainnet --reclaim-space archive
   ```

3. **If repair fails, wipe and resync**
   ```bash
   rm -rf ~/.local/share/x3-chain/chains/x3-mainnet/db
   x3-chain-node --chain mainnet --sync=warp --rpc-port 9944 &
   ```

4. **Monitor resync**
   ```bash
   watch -n 5 'curl -s localhost:9944 -d "{\"jsonrpc\":\"2.0\",\"method\":\"system_syncState\",\"id\":1}" | jq .result.currentBlock'
   ```

5. **Root cause analysis**
   - Was there power loss?
   - Disk failure?
   - Memory error?
   - Upgrade issue?

**Recovery Time:** 5 min - 24 hours (depending on resync time)

---

## 8. Validator Key Compromise

**Symptoms:**
- Suspicious transactions from validator
- Key appears to be signing from multiple locations
- Validator reports key stolen/lost

**First 5 Minutes:**

```bash
# 1. Contact validator
# (out of band call)

# 2. Check if validator is still signing
journalctl -u x3-chain-node | grep "block signed"

# 3. Check for equivocation in logs
journalctl -u x3-chain-node | grep -i "equivocation\|double"
```

**Who Acts:**
- Validator operations
- Security team
- Chain operations lead

**What Gets Paused:**
- Affected validator removed from set
- All transactions signed by that validator paused

**Recovery Steps:**

1. **Immediately disable the validator**
   ```bash
   # Validator operator:
   killall x3-chain-node  # stop current validator
   
   # Or governance emergency vote:
   # governance::emergency_pause_validator(VALIDATOR_ID)
   ```

2. **Rotate the key**
   - Generate new session keys on secure machine
   - Submit `set_keys` extrinsic
   - Transition to active at next era

3. **Audit the compromise**
   - How was key stolen?
   - From where?
   - For how long?
   - What transactions were signed?

4. **Implement mitigations**
   - Hardware security module (HSM) for keys
   - Air-gapped key generation
   - Multisig for critical operations

5. **Return validator to set**
   - At next era or via governance

**Recovery Time:** 30 min - 24 hours

**Prevention:**
- Use hardware wallets/HSM for validator keys
- Air-gapped key generation
- Regular key rotation

---

## 9. Governance Attack

**Symptoms:**
- Malicious governance proposal passes
- Sudo call to execute bad code
- Emergency halt triggered maliciously

**First 5 Minutes:**

```bash
# 1. Check active proposals
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["democracy referenda"],"id":1}' | jq .

# 2. Check if bad proposal is referendum
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_call","params":["DemocracyRpc_ongoing_referenda",null],"id":1}' | jq .
```

**Who Acts:**
- Governance committee
- Chain lead

**What Gets Paused:**
- All governance votes
- Execution of active referendum

**Recovery Steps:**

1. **If referendum not yet finalized:**
   - Governance emergency vote to cancel
   - Or wait for referendum to expire

2. **If bad code already executed:**
   - Invoke emergency halt procedure
   - Revert via hard fork if necessary

3. **Identify the attack**
   - Who proposed the malicious referendum?
   - How did they pass?
   - Were governance parameters exploited?

4. **Implement governance changes**
   - Increase voting thresholds
   - Require council approval
   - Longer voting periods
   - Delayed execution

5. **Monitor future proposals**
   - Increased scrutiny
   - Community education
   - Potential fork if attack was successful

**Recovery Time:** 5 min - weeks (if hard fork required)

---

## 10. DEX Reserve Anomaly

**Symptoms:**
- Pool shows more reserves than possible
- Spot price wildly different from exchanges
- Arbitrage bot exploits anomaly

**First 5 Minutes:**

```bash
# 1. Get pool state
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_call","params":["DexRpc_get_pool","0xPOOL_ID"],"id":1}' | jq .

# 2. Check for recent large swaps
journalctl -u x3-chain-node | grep "dex\|swap"

# 3. Get pool balance
curl -s localhost:9944 -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["dex pool balance"],"id":1}' | jq .
```

**Who Acts:**
- DEX operations
- Chain lead

**What Gets Paused:**
- DEX trading (can pause trading pair or whole DEX)
- Liquidity provision/withdrawal

**Recovery Steps:**

1. **Pause the affected pool**
   ```bash
   dex::pause_pool(POOL_ID, true)
   ```

2. **Investigate the anomaly**
   - Was it a code bug?
   - Attack via flash loan?
   - Oracle price manipulation?
   - Database corruption?

3. **Audit recent transactions**
   - Check all swaps in past hour
   - Identify exploit if any

4. **Determine recovery**
   - Revert illegitimate swaps?
   - Adjust pool reserves manually?
   - Hard fork if necessary?

5. **Implement fix**
   - Fix the underlying issue
   - Add invariant checks
   - Test new code

6. **Resume trading**
   - Governance vote to unpause
   - Monitor for further anomalies

**Recovery Time:** 30 min - 24 hours

---

## General Emergency Procedures

### Emergency Halt (Nuclear Option)

Used when immediate action is required to preserve network integrity:

```bash
# 1. Get sudo key (or invoke via governance)
# 2. Submit emergency halt extrinsic
x3-chain-node submit-extrinsic \
  system::set_emergency_halt \
  --sudo \
  --rpc-port 9944

# This:
# - Stops block production
# - Freezes state changes
# - Allows read-only queries
# - Requires governance to resume
```

### Post-Incident Review

After any incident:

1. **Timeline**
   - When did incident start?
   - When was it detected?
   - When was it resolved?
   - Gaps?

2. **Root cause analysis**
   - Why did it happen?
   - Why wasn't it caught?
   - How can we prevent it?

3. **Action items**
   - Immediate fixes deployed?
   - Medium-term improvements?
   - Long-term architectural changes?

4. **Communication**
   - What did we tell the community?
   - What should we have said differently?

---

## Contacts

| Role | Name | Phone | Email | Discord |
|------|------|-------|-------|---------|
| Chain Lead | | | | |
| Security Lead | | | | |
| Validator Lead | | | | |
| DevOps Lead | | | | |
| Communications | | | | |

**War Room:** [Discord/Slack channel link]  
**Status Page:** [status.x3chain.com]  
**Incident Post-mortem Repo:** `https://github.com/x3-chain/incident-reports`

---

**Document Version:** 1.0  
**Last Updated:** 2026-04-26  
**Next Review:** 2026-05-26 (monthly)
