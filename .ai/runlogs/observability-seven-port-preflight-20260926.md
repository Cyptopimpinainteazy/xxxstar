# The seven-validator observability gate can no longer pass on another run's nodes

Date: 2026-09-26
Subsystem: `scripts/monitoring/testnet7-observability-check.sh` (matrix row `X3-OPS-010`)
Gate: `observability across seven validators`, from `scripts/local-ci.sh --live`

## How this was found

I re-ran the gate to check the evidence cited on `X3-OPS-010`, and it **failed** — but not on
anything the row claims:

```
[obs7] PASS: a real Prometheus (promtool-validated config) scrapes 6 of 6 expected validators: all up
scripts/monitoring/testnet7-observability-check.sh: line 328: 2610590 Killed   prometheus ...
[obs7] FAIL: Grafana reports datasource health 'ERROR', not OK:
       ... dial tcp 127.0.0.1:19700: connect: connection refused ...
```

`prometheus.log` shows a clean start, a successful config load, all six scrapes up, and then no
shutdown lines at all — it was `SIGKILL`ed. No script in the tree kills `prometheus` by name, and
`load average` was ~25 from another agent's `rustc`. The real cause was visible on the next look:

```
$ ps -eo pid,args | grep run-7-validators-local
2612055 bash .../run-7-validators-local.sh --chain-spec /tmp/x3-obs7.hQwrYj/spec/...
$ ss -ltn | ... :19800 :30500
```

**Two invocations of this check were running at once**, and every port it binds is fixed:
validators on `RPC_BASE` 19800-19806, `P2P_BASE` 30500-30506 and `PROM_BASE` 19600-19606, with
Prometheus, Grafana and the collector on 19700/19701/19702. Overlapping runs therefore do not get
two networks, they get one. The consequence is worse than the kill: the losing run had already
printed

```
[obs7] PASS: a real Prometheus (promtool-validated config) scrapes 6 of 6 expected validators: all up
```

for a network **it had not started**, and its eventual failure blamed Grafana's datasource for what
was really a port collision. A green criterion earned by another process's nodes is exactly the
outcome this repository's evidence standard exists to prevent, so the check now refuses to start.

## The fix

`scripts/monitoring/testnet7-observability-check.sh` preflights every port it will bind — the three
per-validator ranges plus the three helper ports — before it creates its work directory, and fails
naming each port already held and the process holding it. It runs after the `info`/`fail`/`pass`
helpers and before `mktemp -d`, so a refusal leaves no `/tmp/x3-obs7.*` behind.

The refusal points at the two ways out: stop the other run, or move this one with
`X3_OBSERVABILITY_RPC_BASE` / `X3_OBSERVABILITY_P2P_BASE` / `X3_OBSERVABILITY_PROM_BASE`.

## Measured, both directions

Port held, check must refuse (bound a listener on the collector's helper port 19702):

```
$ python3 -m http.server 19702 --bind 127.0.0.1 &
$ bash scripts/monitoring/testnet7-observability-check.sh
[obs7] FAIL: port(s) already in use: 19702 held by users:(("python3",pid=2618388,fd=3)) — this
  check binds fixed ports (validators on rpc 19800, p2p 30500, metrics 19600, plus
  19700/19701/19702 for Prometheus, Grafana and the collector), so a concurrent run would scrape
  this run's validators and this run would scrape its — a pass that was earned by another
  process's nodes. Stop the other run, or move this one with X3_OBSERVABILITY_RPC_BASE,
  X3_OBSERVABILITY_P2P_BASE and X3_OBSERVABILITY_PROM_BASE.
exit=1        work dirs before=27 after=27      # nothing left behind
```

Ports free, gate must pass — the gate line, on the bytes committed with this note:

```
$ bash scripts/local-ci.sh --live --only observability-across-seven-validators
PASS observability across seven validators   82s
local-ci: all gates passed

[obs7] ports free: rpc 19800-19806, p2p 30500-30506, metrics 19600-19606, helpers 19700/19701/19702
[obs7] validator 1 :19600  name=x3-testnet-node-01 chain=x3_chain_testnet finalized=99  peers=6 role=4
        … validator 7 :19606  name=x3-testnet-node-07 chain=x3_chain_testnet finalized=93  peers=6 role=4
[obs7] PASS: a real Prometheus (promtool-validated config) scrapes 7 of 7 expected validators: all up
[obs7] PASS: Grafana serves the shipped dashboard and that panel returns 7 point(s) through Prometheus
[obs7] PASS: every observed validator agrees on one chain at finalized height 6
[obs7] PASS: Fluent Bit ingests 7 of 7 validator streams into 7 attributable sink file(s)
```

`bash scripts/check-script-syntax.sh` → `failures: 0`, `OK - every script parses`.

## Still open (not fixed here)

1. **The sibling checks have the same shape.** `scripts/monitoring/local3-monitoring-check.sh` and
   `scripts/monitoring/local3-logging-check.sh` bind fixed ports too and have no preflight, so the
   same collision is possible between them and between a `local3` check and the seven-validator
   check. They are two-three orders cheaper to run, but they are evidence as well.
2. **The other fixed-port live gates were not audited here** —
   `scripts/testnet/public-testnet-gate-drill.sh`, `scripts/snapshot-live-restore-proof.sh` and the
   `node/tests/*_live*.rs` ignored tests among them. One counter-example was checked:
   `scripts/mainnet/runtime_upgrade_rehearsal.sh` probes nine free ports rather than naming them
   (`free_port()` at line 202; its measured log line is `starting three validators (rpc
   34565/53587/58359)`), which is the pattern worth copying. The two-agent model that found this
   bug is the normal way this repository is worked, so each of the others deserves the same
   preflight-or-probe decision.
3. **The failure message.** Even with the preflight, a Prometheus that dies *after* the preflight
   still surfaces as `Grafana reports datasource health 'ERROR'`. The gate should re-check
   `kill -0 "$PROM_PID"` when the datasource is unhealthy and say so, the way the scrape loop
   already does.
4. `X3-OPS-010`'s row text was not changed by this note; the two gates it cites remain the evidence
   for its score, and this fix removes a way for one of them to be green for the wrong reason.
