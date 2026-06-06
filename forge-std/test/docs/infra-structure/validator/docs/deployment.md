# Deployment

## Local Testnet

```bash
./deployment/run_testnet.sh
```

Ensure `solana-test-validator` and `geth` are installed and in PATH.

## Validator

```bash
ccgv orchestrator
```

## Dashboard

```bash
ccgv dashboard
```

## Docker

```bash
docker compose -f deployment/docker-compose.yml up --build
```
