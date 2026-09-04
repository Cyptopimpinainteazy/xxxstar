# Task Progress — Full Cross-Chain + Cross-VM to 100%

## Internal Cross-VM (95% → 100%)
- [ ] Audit remaining 5% gap items in `pallet-x3-cross-vm-router` (fuzz tests, edge cases)
- [ ] Add property-based fuzz testing for supply invariant

## External EVM (22% → 100%)
- [ ] Create `X3-contracts/evm/contracts/X3ExternalGateway.sol` — lock/release ERC20 per chain
- [ ] Create `X3-contracts/evm/contracts/X3VmERC20.sol` — kernel-callable ERC20 adapter
- [ ] Create `X3-contracts/evm/contracts/X3KernelBridge.sol` — bridge interface contract
- [ ] Create `X3-contracts/evm/contracts/interfaces/IX3Verification.sol` — proof interface
- [ ] Wire production EVM receipt proof verifier in `crates/x3-verification-router/`
- [ ] Wire `pallet-x3-crosschain-gateway` extrinsic for production proof submission
- [ ] Create gateway integration tests

## SVM / Solana (10% → 100%)
- [ ] Create SVM adapter program in `programs/svm/x3-svm-token-adapter/`
- [ ] Create SVM lock/deposit interface
- [ ] Create Solana proof verifier adapter

## Bitcoin (5% → 100%)
- [ ] Create Bitcoin vault mechanism (threshold multisig + proof accounting)
- [ ] Create Bitcoin deposit/withdraw flow

## Relayer (50% → 100%)
- [ ] Wire relayer `crates/x3-relayer/` with gateway extrinsics
- [ ] Add EVM event watching + proof construction
- [ ] Add X3 withdrawal event watching + release submission

## End-to-End Tests (0% → 100%)
- [ ] Create external ERC20 → X3 deposit flow test
- [ ] Create X3 → external ERC20 withdrawal flow test
- [ ] Create full cross-chain round-trip test
- [ ] Create replay/failure/invariant tests for external flows

## Final Hardening
- [ ] Wire all new crates into runtime
- [ ] Update CURRENT_MAINNET_STATUS.md
- [ ] Update CI to run all new tests