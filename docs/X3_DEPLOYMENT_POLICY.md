# X3 Deployment Policy — Docker vs Systemd

## Policy Summary

| Component | Allowed | Required Method |
|---|---|---|
| **Validator node** | ❌ No Docker | systemd, signed binary from GitHub releases |
| **Bootnode** | ❌ Prefer no Docker | systemd, signed binary from GitHub releases |
| **Public RPC node** | ⚠️ Optional | Prefer systemd; Docker acceptable if not validating |
| **Explorer** | ✅ Docker allowed | `docker compose` or K8s |
| **Indexer** | ✅ Docker allowed | `docker compose` or K8s |
| **PostgreSQL / Redis** | ✅ Docker allowed | `docker compose` or K8s |
| **Grafana / Prometheus / Loki** | ✅ Docker allowed | `docker compose` or K8s |
| **Faucet** | ✅ Docker allowed | `docker compose` or K8s |
| **Local devnet** | ✅ Docker allowed | `docker compose` |
| **CI test chain** | ✅ Docker allowed | `Dockerfile.mainnet-check` or ephemeral containers |
| **Mainnet release binary** | ❌ No Docker required | `srtool` + static linking + signed checksums |

## Why This Policy

### Docker is good for:
- **Rapid iteration** — spin up/down devnets in seconds
- **Reproducible CI** — same environment every run
- **Isolation of non-consensus services** — explorer, indexer, monitoring
- **Portability** — works identically on dev laptops, CI runners, and staging servers

### Docker is bad for validators:
1. **Performance overhead** — Docker networking adds latency and jitter to P2P timing
2. **Disk management** — `docker volume` adds abstraction layers; NVMe passthrough is complex
3. **Reliability** — `docker restart policies` vs `systemd restart=always` — systemd wins
4. **Security** — container escape vulnerabilities; extra kernel surface
5. **Operational simplicity** — debugging a validator issue is harder through container layers
6. **Stability** — `docker daemon restart` or image updates should never affect block production

## Directory Structure

```
packaging/
└── systemd/                   # Production validator deployment
    ├── x3-validator.service   # Authority/validator node
    └── x3-bootnode.service    # P2P boot node

docker/                        # Support infrastructure (NOT validators)
├── docker-compose.yml         # Explorer, indexer, monitoring, faucet
├── devnet/                    # Local devnet setup
│   └── docker-compose.yml
└── ci/                        # CI test chain images

scripts/
├── install-validator.sh       # Download binary + install systemd service
├── install-bootnode.sh        # Download binary + install bootnode systemd service
├── harden-validator.sh        # Security hardening for validator hosts
├── snapshot-restore.sh        # Chain snapshot backup/restore
└── testnet-full-launch.sh     # Local devnet (non-production)

devops/
└── docker/                    # Dockerfiles for support services
    ├── explorer/
    ├── indexer/
    ├── faucet/
    └── monitoring/
```

## Validator Installation Sequence (Production)

```
1. Provision bare-metal or VPS with Ubuntu 24.04 LTS
2. Mount NVMe storage at /var/lib/x3
3. Run: sudo bash scripts/install-validator.sh --version v0.4.0-rc.1
4. Run: sudo bash scripts/harden-validator.sh
5. Generate session keys: /usr/local/bin/x3-chain-node key generate
6. Set node-key: /usr/local/bin/x3-chain-node key insert --key-type aura --suri <seed>
7. Start: sudo systemctl start x3-validator
8. Verify: sudo journalctl -fu x3-validator
```

## Dev/Test Setup (Docker)

```
docker compose -f docker/devnet/docker-compose.yml up -d
```

## CI Pipeline

```
docker build -f Dockerfile.mainnet-check -t x3-mainnet-check .
docker run --rm x3-mainnet-check
```

## Decision Matrix

| Scenario | Validator | Bootnode | RPC | Explorer | Indexer | Monitoring |
|---|---|---|---|---|---|---|
| Local dev | Docker | Docker | Docker | Docker | Docker | Docker |
| CI test | Docker | Docker | Docker | — | — | Docker |
| Staging testnet | systemd | systemd | systemd | Docker | Docker | Docker |
| Public testnet | systemd | systemd | systemd/Docker | Docker | Docker | Docker |
| Mainnet | systemd | systemd | systemd | Docker | Docker | Docker |