# X3 RPC Gateway Policy

## Public RPC Architecture

```
Users ──> DNS ──> Load Balancer ──> Rate Limiter ──> RPC Node(s)
                                     │
                                     └──> Admin API (restricted)
```

## Allowed RPC Methods (Public)

### Safe Methods (always allowed)
| Method | Purpose |
|---|---|
| `chain_getBlock` | Fetch block by hash/number |
| `chain_getBlockHash` | Get block hash |
| `chain_getFinalizedHead` | Get latest finalized block |
| `chain_getHeader` | Get block header |
| `state_getMetadata` | Get runtime metadata |
| `state_getRuntimeVersion` | Get runtime version |
| `system_chain` | Get chain name |
| `system_chainType` | Get chain type |
| `system_health` | Get node health |
| `system_name` | Get node name |
| `system_version` | Get node version |
| `system_peers` | Get connected peers |
| `state_getStorage` | Query storage keys |
| `state_getPairs` | Get key-value pairs |
| `state_queryStorage` | Multi-key storage query |

### Unsafe Methods (blocked on public RPC)
| Method | Reason |
|---|---|
| `system_addReservedPeer` | Network manipulation |
| `system_removeReservedPeer` | Network manipulation |
| `author_insertKey` | Key management |
| `author_rotateKeys` | Session key management |
| `author_hasKey` | Key information disclosure |
| `author_hasSessionKeys` | Session key info disclosure |
| `offchain_localStorageSet` | Off-chain worker manipulation |
| `dev_getBlockStats` | Dev-only |
| `system_nodeRoles` | Internal info |

## Rate Limiting

| Tier | Auth | Rate Limit |
|---|---|---|
| Anonymous | No API key | 100 req/min |
| Registered | API key | 1,000 req/min |
| Premium | API key + whitelist | 10,000 req/min |

## Deployment Matrix

| Environment | Architecture | Notes |
|---|---|---|
| Local dev | Single node | No rate limiting |
| Staging | 1 RPC node behind LB | Rate limiting enabled |
| Testnet alpha | 2 RPC nodes + LB | Anonymous tier + faucet |
| Testnet beta | 3 RPC nodes + LB + CDN | Registered tier available |
| Mainnet | 5+ RPC nodes + LB + CDN + WAF | Full tier model |