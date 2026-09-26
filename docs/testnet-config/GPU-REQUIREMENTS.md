{
  "title": "GPU REQUIREMENTS & SPECIFICATIONS",
  "version": "1.0",
  "minimum_hardware": {
    "gpu": {
      "model": "NVIDIA GeForce GTX 1070 (or better)",
      "count": 3,
      "vram_per_gpu": "8GB",
      "total_vram": "24GB",
      "compute_capability": "6.1 (min 6.0)"
    },
    "cpu": {
      "cores": "16+ physical cores",
      "frequency": "3.0 GHz+",
      "memory": "128GB RAM"
    },
    "storage": {
      "ledger": "2TB NVMe SSD (ledger)",
      "accounts": "500GB NVMe SSD (accounts DB)",
      "type": "NVMe (PCIe 3.0+ preferred)"
    }
  },
  "software_stack": {
    "cuda": {
      "version": "11.8+",
      "compatibility": "CC 6.0+"
    },
    "nvidia_driver": {
      "version": "520.56+",
      "note": "Must support CUDA 11.8+"
    },
    "linux": {
      "kernel": "5.10.0+",
      "distributions": [
        "Ubuntu 20.04+ LTS",
        "Debian 11+",
        "CentOS 8+"
      ]
    },
    "solana": {
      "version": "1.17.0+ (GPU variant)",
      "note": "Use provided binary"
    }
  },
  "performance_expectations": {
    "_note": "Unverified: every figure below was carried over from the withdrawn GPU release note (see RELEASE-NOTES.md and GAP-GPU-CLAIMS). No command in this repository measures any of them; the only benchmarks it does contain record signature and hash rates, not finalized-chain throughput. Treat them as targets at best.",
    "signature_verification": "825k sig/sec per GPU (unverified vendor figure)",
    "poh_computation": "1.55M hash/sec (unverified vendor figure)",
    "tx_validation": "1.8M tx/sec (unverified vendor figure)",
    "overall_tps": "2M+ TPS with 3x GPUs (unverified vendor figure)",
    "testnet_actual": "1-5M TPS (network dependent) - unverified, no measurement exists"
  },
  "thermal_considerations": {
    "tjunction_max": "72\u00b0C (GTX 1070)",
    "normal_operation": "60-70\u00b0C",
    "throttling_temperature": "73\u00b0C+",
    "cooling": "Active cooling required (GPU deshroud + room cooling)"
  }
}
