# Deployment Verification Checklist - Hub Fee

Date: __________
Operator: __________
Environment: __________

## A. Startup

- [ ] Validator starts without panic
- [ ] RPC responds on expected endpoint
- [ ] Block height increases

## B. Functional (Phase 1)

- [ ] dApp registration succeeded
- [ ] `record_revenue(1_000_000)` succeeded
- [ ] `HubFeeCollected` emitted with fee `25_000`
- [ ] `record_revenue(10_000_000)` succeeded
- [ ] `HubFeeCollected` emitted with fee `250_000`
- [ ] post-fee earnings math verified
- [ ] withdrawal path succeeded

## C. Monitoring (Phase 2)

- [ ] monitor script ran for full duration
- [ ] event payload fields present and valid
- [ ] all observed fees match 250 bps
- [ ] no null/corrupt event values observed

## D. Decision

- [ ] TESTNET GO
- [ ] NO-GO

If NO-GO, reason:

________________________________________________________

Next remediation action:

________________________________________________________
