{
  "title": "TROUBLESHOOTING GUIDE",
  "version": "1.0",
  "faqs": [
    {
      "number": 1,
      "issue": "GPU kernel fails to load",
      "diagnosis": "Check error log for 'CUDA_ERROR_NOT_PERMITTED'",
      "solutions": [
        "Verify CUDA 11.8+ installed: nvcc --version",
        "Check GPU compute capability: nvidia-smi -q",
        "Ensure GTX 1070+ (CC 6.1+)",
        "Rebuild kernels if version mismatch"
      ]
    },
    {
      "number": 2,
      "issue": "Low TPS (below 100k)",
      "diagnosis": "GPU not being fully utilized",
      "solutions": [
        "Check GPU utilization: nvidia-smi (should be >70%)",
        "Verify no GPU errors: nvidia-smi -q (check values)",
        "Check network bandwidth (might be bottleneck)",
        "Increase batch sizes in gpu-runtime-config.json"
      ]
    },
    {
      "number": 3,
      "issue": "Memory leak detected",
      "diagnosis": "VRAM usage growing >100MB/hour",
      "solutions": [
        "Restart validator (should reset VRAM)",
        "Check for CUDA context leaks in logs",
        "Verify malloc/free balance in GPU kernels",
        "Switch to CPU-only mode as temporary fix"
      ]
    },
    {
      "number": 4,
      "issue": "Validator out of sync",
      "diagnosis": "Fork distance > 3 slots",
      "solutions": [
        "Check network connectivity (ping testnet-entrypoint)",
        "Verify RPC endpoint accessible",
        "Check GPU latency (should be <50ms)",
        "Review commit graph: solana status"
      ]
    },
    {
      "number": 5,
      "issue": "Consensus timeouts",
      "diagnosis": "Validator missing votes/blocks",
      "solutions": [
        "Reduce GPU batch sizes (lower latency)",
        "Check CPU utilization (should be <50%)",
        "Verify GPU isn't thermal throttling",
        "Review voting transaction latency"
      ]
    }
  ]
}