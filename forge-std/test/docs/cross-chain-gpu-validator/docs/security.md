# Security

## Atomic Guarantees

- Both chains must validate before approval.
- Failures or timeouts trigger rollback.
- Registry unavailability fails closed.

## GPU Failover

GPU failure triggers CPU validation only as failover. If both fail,
the orchestrator fails closed and rejects swaps.

## GPU Parity Checks

GPU output is compared to CPU reference output when
`CCGV_GPU_PARITY_CHECK=true`. Mismatches trigger failover or failure
when GPU is required.
