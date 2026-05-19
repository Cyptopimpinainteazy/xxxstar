# Production Preflight Checklist

Run this before any staging or production deployment.

## 1. Environment
- Copy `.env.production.example` to your working `.env`
- Set `PRODUCTION_RPC_URL`
- Set `PRODUCTION_CHAIN_ID`
- Set `DEPLOYER_PRIVATE_KEY`
- Set `COMPILER_VERIFIER_ADDRESS`
- Set `CHECKER_VERIFIER_ADDRESS`
- Set `DEX_QUOTE_TOKEN_ADDRESS`

## 2. Optional controls
- Decide whether `BOOTSTRAP_DEX` should be `true` or `false`
- If bootstrapping DEX, set `DEX_BOT_LIQUIDITY` and `DEX_QUOTE_LIQUIDITY`
- If scheduling governance handoff, set `PENDING_MULTISIG_ADDRESS`
- If transferring ownership immediately, set `TRANSFER_OWNERSHIP_TO`
- If verifying contracts, set `VERIFY_CONTRACTS=true`
- If verifying contracts, set `ETHERSCAN_API_KEY`
- Decide whether `FAIL_ON_VERIFY_ERROR` should be enabled

## 3. Local verification
```bash
cd /home/lojak/Desktop/x3-chain-master/botchain-tri-vm-genesis/hardhat
npm test
```

## 4. Network preflight
```bash
cd /home/lojak/Desktop/x3-chain-master/botchain-tri-vm-genesis/hardhat
npm run deploy:preflight
```

Expected outcome:
- no transactions sent
- chain ID matches expectation
- quote token contract exists
- quote token balance is sufficient if DEX bootstrap is enabled
- verification settings are valid

## 5. Deployment
```bash
cd /home/lojak/Desktop/x3-chain-master/botchain-tri-vm-genesis/hardhat
npm run deploy:production
```

## 6. Post-deploy
- Save `deployments/<network>.production.json`
- Record final contract addresses in release notes/runbook
- Confirm ownership and pending multisig state on-chain
- Confirm any expected verification results
- Confirm DEX liquidity state if bootstrapped
