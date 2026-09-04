# P4 Day 1 Bottlenecks & Risks

Date: 2026-03-15

## Technical Bottlenecks
- CUDA toolkit (`nvcc`) missing, so kernels cannot compile locally yet.
- `ed25519-donna` is not available on PyPI; using PyNaCl as interim CPU verification backend until a vendored C build or binding is added.
- Current `solana_accelerators.py` uses Python hashing per-transaction and NumPy arrays; CPU-side preprocessing may dominate until batch sizes are large and GPU transfer is fully optimized.
- Data transfer overhead (host → GPU → host) will be significant for small batches; minimum batch size needs to be enforced.
- Kernel in `solana_gpu_kernels.cu` is pseudo-code; real ed25519 verification requires constant-time field arithmetic and careful memory layout.
- Cupy + CUDA 11x dependency needs a driver/toolkit alignment; current driver reports CUDA 12.2 runtime.

## Test Harness Notes
- Async tests require `pytest-asyncio` (now installed).
- Test class was misnamed (`TestSigantureVerification`); renamed to `TestSignatureVerification` to match checklist invocation.
- Full suite still has benchmark marks requiring configuration if used.
