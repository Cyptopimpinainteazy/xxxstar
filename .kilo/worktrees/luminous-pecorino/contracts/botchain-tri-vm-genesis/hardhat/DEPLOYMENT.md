# Production Deployment

## Files
- `scripts/deploy.js`: local/dev deployment with mock quote token bootstrap
- `scripts/deploy.production.js`: production-oriented deployment with explicit environment validation
- `.env.production.example`: required environment variables for production deployment

## Safety model
The production deploy script refuses to run on the in-process `hardhat` network and requires explicit values for:
- `COMPILER_VERIFIER_ADDRESS`
- `CHECKER_VERIFIER_ADDRESS`
- `DEX_QUOTE_TOKEN_ADDRESS`

It never deploys mock WETH. If DEX bootstrap is enabled, it expects an existing quote token and pre-funded deployer balances.

## Example flow
```bash
cd /home/lojak/Desktop/x3-chain-master/botchain-tri-vm-genesis/hardhat
cp .env.production.example .env
# edit values carefully
npm test
npm run deploy:preflight
npm run deploy:production
```

## Dry-run / preflight
`npm run deploy:preflight` performs a production-style validation pass without sending transactions. It checks:
- required environment variables are present
- connected chain ID matches `PRODUCTION_CHAIN_ID` when set
- verifier and governance addresses are well-formed
- `DEX_QUOTE_TOKEN_ADDRESS` has deployed code
- deployer quote-token balance is sufficient when `BOOTSTRAP_DEX=true`
- verification settings are internally consistent

## Optional controls
- `BOOTSTRAP_DEX=true` enables initial liquidity bootstrap using `DEX_BOT_LIQUIDITY` and `DEX_QUOTE_LIQUIDITY`
- `PENDING_MULTISIG_ADDRESS` schedules a delayed MarriageLicense multisig update
- `TRANSFER_OWNERSHIP_TO` transfers ownership of BOT, MarriageLicense, and AtomicSwapAdapter after deployment
- `VERIFY_CONTRACTS=true` enables explorer verification after deployment
- `FAIL_ON_VERIFY_ERROR=true` makes verification failures fail the deployment

## Verification
When `VERIFY_CONTRACTS=true`, the production script uses Hardhat verify with `ETHERSCAN_API_KEY` and records verification status per contract in the deployment output JSON.

## Artifacts
The production deployment script writes a deployment record to `deployments/<network>.production.json`.
