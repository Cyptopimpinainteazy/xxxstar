// ── X3 Chain – Multi-Chain GPU Validator Dashboard ───────────────────────

function fmt(n, decimals = 0) {
  if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return Number(n).toFixed(decimals);
}

function setTxt(id, val) {
  const el = document.getElementById(id);
  if (el) el.textContent = val;
}

function renderGpuBars(gpus) {
  const container = document.getElementById("gpu_bars");
  if (!container || !gpus) return;
  container.innerHTML = gpus
    .map(
      (g, i) =>
        `<div class="gpu-bar-row">
          <span class="gpu-label">GPU ${i}</span>
          <div class="gpu-bar-bg">
            <div class="gpu-bar-fill" style="width:${g.util_pct}%"></div>
          </div>
          <span class="gpu-pct">${g.util_pct}%</span>
        </div>`
    )
    .join("");
}

async function refreshMetrics() {
  let data;
  try {
    const response = await fetch("/metrics.json");
    if (!response.ok) return;
    data = await response.json();
  } catch {
    return;
  }

  // Throughput
  setTxt("svm_tps", fmt(data.svm_tps || 0));
  setTxt("evm_tps", fmt(data.evm_tps || 0));
  setTxt("cosmos_tps", fmt(data.cosmos_tps || 0));
  setTxt("substrate_tps", fmt(data.substrate_tps || 0));
  setTxt("total_tx", fmt(data.total_tx || 0));
  setTxt("chains_active", data.chains_active || 0);

  // GPU utilization
  setTxt("gpu_count", data.gpu_count || 0);
  if (data.gpus) {
    renderGpuBars(data.gpus);
    const avg =
      data.gpus.reduce((s, g) => s + g.util_pct, 0) / (data.gpus.length || 1);
    setTxt("gpu_avg_util", avg.toFixed(0) + "%");
    const vramUsed = data.gpus.reduce((s, g) => s + (g.vram_used_mb || 0), 0);
    const vramTotal = data.gpus.reduce((s, g) => s + (g.vram_total_mb || 0), 0);
    setTxt("gpu_vram_used", `${fmt(vramUsed)} / ${fmt(vramTotal)} MB`);
  }

  // Crypto ops/sec
  setTxt("secp256k1_ops", fmt(data.secp256k1_ops || 0));
  setTxt("keccak256_ops", fmt(data.keccak256_ops || 0));
  setTxt("ed25519_ops", fmt(data.ed25519_ops || 0));
  setTxt("sha256_ops", fmt(data.sha256_ops || 0));

  // Gas savings
  if (data.gas_baseline && data.gas_optimized) {
    const pct =
      ((data.gas_baseline - data.gas_optimized) / data.gas_baseline) * 100;
    setTxt("gas_savings_pct", pct.toFixed(1) + "%");
  }
  setTxt("gas_baseline", fmt(data.gas_baseline || 0));
  setTxt("gas_optimized", fmt(data.gas_optimized || 0));

  // Atomic swaps
  setTxt(
    "success_rate",
    `${((data.atomic_success_rate || 0) * 100).toFixed(2)}%`
  );
  setTxt("rollbacks", data.atomic_rollbacks || 0);
  setTxt("pending_swaps", data.pending_swaps || 0);

  // Swarm compute
  setTxt("swarm_tasks", data.swarm_running || 0);
  setTxt("swarm_queued", data.swarm_queued || 0);
  setTxt("swarm_preemptions", data.swarm_preemptions || 0);

  // Health
  setTxt("gpu_health", data.gpu_health || "unknown");
  setTxt("svm_rpc", `${(data.svm_rpc_latency_ms || 0).toFixed(1)} ms`);
  setTxt("evm_rpc", `${(data.evm_rpc_latency_ms || 0).toFixed(1)} ms`);

  // Footer
  if (data.timestamp) {
    const ts = new Date(data.timestamp * 1000).toLocaleTimeString();
    setTxt("timestamp", `Last update: ${ts}`);
  }
  if (data.uptime_sec) {
    const h = Math.floor(data.uptime_sec / 3600);
    const m = Math.floor((data.uptime_sec % 3600) / 60);
    setTxt("uptime", `Uptime: ${h}h ${m}m`);
  }
}

setInterval(refreshMetrics, 1000);
refreshMetrics();
