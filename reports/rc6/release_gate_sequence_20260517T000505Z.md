# Release Gate Sequence Run (20260517T000505Z)

## 1) Build (cargo build -p x3-chain-node --release)
exit_code=127

## 2) Cross-chain smoke (scripts/mainnet/rc2_internal_settlement_smoke.sh)
exit_code=1

## 3) Invariant/Security suite (scripts/run-security-gates.sh all)
exit_code=127

## 4) RC6 readiness (scripts/mainnet/rc6_public_testnet_readiness.sh)
exit_code=1

## Log Files
- reports/rc6/build_20260517T000505Z.log
- reports/rc6/rc2_smoke_20260517T000505Z.log
- reports/rc6/security_gates_20260517T000505Z.log
- reports/rc6/rc6_readiness_20260517T000505Z.log
