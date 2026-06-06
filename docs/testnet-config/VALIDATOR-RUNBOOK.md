{
  "title": "SOLANA GPU VALIDATOR DEPLOYMENT RUNBOOK",
  "version": "1.0",
  "sections": [
    {
      "number": "1",
      "title": "Prerequisites & Requirements",
      "subsections": [
        "1.1 Hardware: 3x NVIDIA GPUs (CC 6.1+, 8GB VRAM each)",
        "1.2 Software: Linux 5.10+, CUDA 11.8+, Solana CLI",
        "1.3 Network: 1Gbps+ connection, low latency",
        "1.4 Disk: 2TB NVMe SSD (ledger), 500GB (accounts)"
      ]
    },
    {
      "number": "2",
      "title": "Installation Steps",
      "subsections": [
        "2.1 Extract deployment package",
        "2.2 Run install-validator.sh",
        "2.3 Install GPU kernels (install-gpu-kernels.sh)",
        "2.4 Configure validator (edit validator-testnet.toml)",
        "2.5 Download snapshot (optional, faster startup)"
      ]
    },
    {
      "number": "3",
      "title": "GPU Configuration",
      "subsections": [
        "3.1 Verify CUDA installation (nvidia-smi)",
        "3.2 Test GPU memory (nvidia-smi -q)",
        "3.3 Enable peer access (if multiple GPUs)",
        "3.4 Set GPU clock speeds (optimizations)",
        "3.5 Monitor GPU utilization during startup"
      ]
    },
    {
      "number": "4",
      "title": "Validator Startup",
      "subsections": [
        "4.1 Start validator: ./start-validator.sh",
        "4.2 Monitor slot catchup (should reach ~within 5 min)",
        "4.3 Verify GPU kernels loaded (check logs)",
        "4.4 Confirm consensus participation (voting)",
        "4.5 Check TPS metrics (should see 100k+)"
      ]
    },
    {
      "number": "5",
      "title": "Monitoring & Alerts",
      "subsections": [
        "5.1 Access Grafana (http://localhost:3000)",
        "5.2 Check TPS & Throughput dashboard",
        "5.3 Monitor GPU utilization (should be >70%)",
        "5.4 Review consensus health (fork distance 0-1)",
        "5.5 Set up alerting (see OPERATIONS-MANUAL)"
      ]
    },
    {
      "number": "6",
      "title": "Troubleshooting",
      "subsections": [
        "6.1 GPU kernel not loading? See TROUBLESHOOTING.md #1",
        "6.2 Low TPS? Check TROUBLESHOOTING.md #3",
        "6.3 Consensus issues? See TROUBLESHOOTING.md #5",
        "6.4 Memory leaks? Run: nvidia-smi -q -d MEMORY,UTILIZATION"
      ]
    },
    {
      "number": "7",
      "title": "Performance Tuning",
      "subsections": [
        "7.1 Adjust GPU batch sizes (in runtime config)",
        "7.2 Enable GPU peer access for multi-GPU",
        "7.3 Tune CUDA grid/block dimensions",
        "7.4 Monitor thermal throttling (expected <10% duration)"
      ]
    },
    {
      "number": "8",
      "title": "Security Hardening",
      "subsections": [
        "8.1 Run with minimal privileges (solana user)",
        "8.2 Restrict RPC port access (:9944)",
        "8.3 Enable firewall (except gossip :8001+)",
        "8.4 Rotate validator keys regularly"
      ]
    },
    {
      "number": "9",
      "title": "Maintenance & Updates",
      "subsections": [
        "9.1 Regular ledger cleanup (weekly)",
        "9.2 Monitor disk space (warn at 80%)",
        "9.3 Update GPU drivers (monthly)",
        "9.4 Backup validator keys (weekly)"
      ]
    },
    {
      "number": "10",
      "title": "Fallback Procedure",
      "subsections": [
        "10.1 If GPU fails: Disable GPU backend in config",
        "10.2 CPU-only mode: Still 733k TPS available",
        "10.3 Restart validator (will use CPU path)",
        "10.4 Investigate GPU issue while running CPU mode"
      ]
    }
  ],
  "quick_start": "Start validator: ./start-validator.sh (2 min startup, 5 min catchup)",
  "support_contact": "GitHub Issues: x3-chain/p4-gpu-accelerators"
}