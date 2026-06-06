# Atlas Sphere X3 Fee Schedule

| Module         | Chain | Operation         | Fee (bps) | Notes                       |
|----------------|-------|------------------|-----------|-----------------------------|
| AtlasSphereX3  | All   | Transfer         | 50        | Configurable, default 0.5%  |
| AtlasSphereX3  | All   | Staking          | 100       | Configurable, default 1%    |
| AtlasSphereX3  | All   | Swap             | 25        | Configurable, default 0.25% |
| WrappedX3      | All   | Mint/Burn        | 50        | Per chain, configurable     |
| AtomicBridge   | All   | Bridge Transfer  | 50        | Per chain, configurable     |
| StakingPool    | All   | Claim/Unstake    | 0         | No fee, rewards from pool   |
| Treasury       | All   | Fee Routing      | 0         | Split on receipt            |

- All fees are routed to Treasury and split per config.
- Fees can be adjusted by governance proposals.
