#!/usr/bin/env bash
# Sourced by scripts/mainnet/public_testnet_gate.sh (Gate 7: forced node restart
# drill) and driven by tests/test_restart_proof_verdict.py.
#
# Gate 7 used to accept any file at reports/drill_node_restart.md that contained
# `restart_drill: PASS`. A report from a different chain, or from this chain's
# *previous* boot, satisfied a launch criterion. The drill now records the genesis
# hash it restarted and when it ran; this function is the whole rule that reads
# those back.
#
# It is a function rather than inline shell for two reasons:
#
#   * the gate needs a seven-validator network to run at all, so an inline rule
#     has no test — which is how the fail-open above survived. This rule is
#     covered by tests/test_restart_proof_verdict.py.
#   * it takes `now` as an argument instead of reading the clock, so a test can
#     place a proof at any age instead of waiting for one to age.
#
# `restart_proof_verdict <report> <expected_genesis> <now_epoch> [<max_age_seconds>] [<rpc_url>]`
# prints exactly one line to stdout — `pass`, or `fail:<reason>` — and never
# exits non-zero and never writes anywhere. The caller decides what a refusal
# means, so a refusal can never be mistaken for a pass by a missing `set -e`.

# A proof older than this cannot certify a launch. Chain identity is necessary but
# not sufficient: a network re-booted from the same spec has the same genesis
# hash, so a report from that network's *previous* boot still names this chain.
# Twenty-four hours is a policy bound rather than a proof of boot identity — the
# honest claim it supports is "a restart proof from the last day, on this chain"
# — and it is deliberately generous because the documented flow is drill-then-gate.
X3_RESTART_PROOF_MAX_AGE_SECONDS="${X3_RESTART_PROOF_MAX_AGE_SECONDS:-86400}"

# A drill host whose clock runs slightly ahead of the gate's is ordinary. A proof
# dated far enough in the future that the gate cannot place it in time is refused
# rather than rounded into "fresh".
X3_RESTART_PROOF_MAX_SKEW_SECONDS="${X3_RESTART_PROOF_MAX_SKEW_SECONDS:-300}"

restart_proof_verdict() {
    local report="$1" expected_genesis="$2" now="$3"
    local max_age="${4:-$X3_RESTART_PROOF_MAX_AGE_SECONDS}"
    local rpc_url="${5:-}"

    if [[ ! -f "$report" ]]; then
        printf 'fail:no drill report at %s — run scripts/drills/node_restart_drill.sh\n' "$report"
        return 0
    fi
    if ! grep -q 'restart_drill: PASS' "$report"; then
        printf 'fail:drill report present but not PASS — see %s\n' "$report"
        return 0
    fi

    # A PASS from *some* run is not a proof about *this* network. Until 2026-09-28
    # this criterion accepted any file at the report path containing
    # `restart_drill: PASS`, so a report from a different chain — or from a
    # previous boot of this one — satisfied a launch criterion. A report that
    # predates the field is refused rather than trusted, because there is no way
    # to tell which network it described.
    local report_chain
    report_chain="$(sed -n 's/^- restart_drill_chain: //p' "$report" | head -1)"
    if [[ -z "$report_chain" || "$report_chain" == "unknown" ]]; then
        printf 'fail:the report at %s does not name the chain it restarted (reports written before 2026-09-28 do not) — re-run scripts/drills/node_restart_drill.sh against this network\n' "$report"
        return 0
    fi
    if [[ -z "$expected_genesis" ]]; then
        printf "fail:could not read this chain's genesis hash%s, so the restart report cannot be tied to it\n" "${rpc_url:+ from $rpc_url}"
        return 0
    fi
    if [[ "$report_chain" != "$expected_genesis" ]]; then
        printf 'fail:the report proves a restart on chain %s but this gate reads %s — the drill was run against a different network (or a previous boot)\n' "$report_chain" "$expected_genesis"
        return 0
    fi

    local epoch
    epoch="$(sed -n 's/^- restart_drill_epoch: //p' "$report" | head -1)"
    if [[ -z "$epoch" ]]; then
        printf 'fail:the report at %s has no restart_drill_epoch — re-run scripts/drills/node_restart_drill.sh against this network so the proof is dated\n' "$report"
        return 0
    fi
    if [[ ! "$epoch" =~ ^[0-9]+$ ]]; then
        printf 'fail:the report at %s dates the restart as %s, which is not a unix timestamp — re-run scripts/drills/node_restart_drill.sh\n' "$report" "$epoch"
        return 0
    fi
    if [[ ! "$now" =~ ^[0-9]+$ || ! "$max_age" =~ ^[0-9]+$ ]]; then
        printf 'fail:the gate cannot judge the age of the restart proof (now=%s, max_age=%s)\n' "$now" "$max_age"
        return 0
    fi

    local age=$(( now - epoch ))
    if (( age < -X3_RESTART_PROOF_MAX_SKEW_SECONDS )); then
        printf 'fail:the restart proof on %s is dated %ss in the future — that is not a time this gate can reason about; check the clock on the host that ran the drill\n' "$expected_genesis" "$(( -age ))"
        return 0
    fi
    if (( age > max_age )); then
        printf 'fail:the restart proof on %s is %sh old — a launch gate cannot be certified by a restart from a previous boot; re-run scripts/drills/node_restart_drill.sh\n' "$expected_genesis" "$(( age / 3600 ))"
        return 0
    fi

    printf 'pass\n'
    return 0
}
