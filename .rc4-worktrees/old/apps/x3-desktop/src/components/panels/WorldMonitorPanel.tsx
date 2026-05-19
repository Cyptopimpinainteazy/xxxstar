import React, { useEffect, useState } from "react";
import IframePanel from "@/components/panels/IframePanel";

/**
 * WorldMonitorPanel — quick integration wrapper for the World Monitor app.
 *
 * Strategy (Stage 1): prefer a locally-running dev server, fall back to the
 * remote cloud demo. Later stages will port core panels into native React
 * components and remove the iframe.
 */
const CANDIDATE_URLS = [
  // Local dev server used during development (vite default)
  "http://localhost:5173",
  // Packaged / preinstalled path (will be copied into Tauri app-data by setup scripts)
  "/app-store/worldmonitor/index.html",
  // Cloud fallback (always available)
  "https://worldmonitor.app",
];

// Reuse guarded tauriInvoke pattern used elsewhere
async function tauriInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
}

const probe = async (url: string, timeout = 700) => {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);
    await fetch(url, { mode: "no-cors", signal: controller.signal });
    clearTimeout(timer);
    return true;
  } catch {
    return false;
  }
};

function detectSES(): boolean {
  // Best-effort SES detection: presence of `lockdown` or SES globals
  try {
    if (typeof (window as any).lockdown === 'function') return true;
    if ((window as any).__SES__ || (window as any).harden) return true;
  } catch {
    /* ignore */
  }
  return false;
}

const ipfsGatewayForCid = (cid: string) => {
  // Prefer local gateway if available; otherwise use public gateway
  return `http://127.0.0.1:8080/ipfs/${cid}/`;
};

const WorldMonitorPanel: React.FC = () => {
  const [url, setUrl] = useState<string | null>(null);
  const [sesDetected, setSesDetected] = useState(false);
  const [ipfsCids, setIpfsCids] = useState<string[]>([]);

  useEffect(() => {
    let mounted = true;

    (async () => {
      setSesDetected(detectSES());

      // Quick-config override: check for a pre-configured ISPF CID (localStorage first, then Vite env)
      const storedIspfCid = (() => {
        try {
          const local = localStorage.getItem('WORLDMONITOR_ISPF_CID');
          if (local) return local.trim();
        } catch { /* ignore */ }

        try {
          // Vite-config fallback (build-time)
          // eslint-disable-next-line @typescript-eslint/ban-ts-comment
          // @ts-ignore
          const envCid = (typeof import.meta !== 'undefined' && (import.meta as any).env?.VITE_ISPF_CID) ||
            (typeof process !== 'undefined' && (process.env as any).VITE_ISPF_CID);
          if (envCid) return String(envCid).trim();
        } catch { /* ignore */ }

        return null;
      })();

      if (storedIspfCid) {
        // Prefer explicit ISPF config — construct gateway URL if a custom gateway is set, otherwise use ispf:// scheme
        const gateway = (() => {
          try {
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            const g = (typeof import.meta !== 'undefined' && (import.meta as any).env?.VITE_ISPF_GATEWAY) ||
              (typeof process !== 'undefined' && (process.env as any).VITE_ISPF_GATEWAY);
            return g ? String(g).replace(/\/$/, '') : null;
          } catch {
            return null;
          }
        })();

        const targetUrl = gateway ? `${gateway}/ipfs/${storedIspfCid}/` : `ispf://${storedIspfCid}`;
        if (mounted) {
          setIpfsCids([storedIspfCid]);
          setUrl(targetUrl);
          return;
        }
      }

      // 1) Try IPFS first (safe, does not execute remote third-party scripts in our realm)
      try {
        const ipfsInfo = await tauriInvoke<any>('launch_ipfs_storage').catch(() => null);
        if (ipfsInfo && Array.isArray(ipfsInfo.pinned_objects)) {
          const matches = ipfsInfo.pinned_objects
            .filter((p: any) => /world-?monitor/i.test(p.name || ''))
            .map((p: any) => p.cid);
          if (matches.length > 0) {
            if (!mounted) return;
            setIpfsCids(matches);
            setUrl(ipfsGatewayForCid(matches[0]));
            return;
          }
        }
      } catch (err) {
        // ignore IPFS probe failures
      }

      // 2) If SES is active prefer native/cloud fallback (avoid injecting third-party intrinsics)
      if (detectSES()) {
        // Try local dev server quickly, otherwise use remote cloud (but show native fallback UI)
        const localOk = await probe(CANDIDATE_URLS[0]);
        if (localOk && mounted) {
          setUrl(CANDIDATE_URLS[0]);
          return;
        }
        if (mounted) {
          // mark that we'll show a SES-safe native fallback (no iframe scripting expected)
          setUrl(null);
          return;
        }
      }

      // 3) Usual probe order (dev -> packaged -> cloud)
      for (const candidate of CANDIDATE_URLS) {
        if (!mounted) return;
        const ok = await probe(candidate);
        if (ok) {
          if (mounted) setUrl(candidate);
          return;
        }
      }

      if (mounted) setUrl(CANDIDATE_URLS[CANDIDATE_URLS.length - 1]);
    })();

    return () => {
      mounted = false;
    };
  }, []);

  // SES detected & no URL chosen: show safe native fallback that reads IPFS metadata
  if (sesDetected && !url) {
    return (
      <div className="p-6 text-sm text-gray-300 bg-[#0a0a0f] h-full">
        <h3 className="text-lg font-semibold text-white mb-3">World Monitor — Safe Mode</h3>
        <p className="mb-4 text-gray-400">Secure JS lockdown (SES) detected — running a safe, read-only fallback that uses IPFS/local data instead of loading the full external frontend.</p>
        {ipfsCids.length > 0 ? (
          <div className="space-y-2">
            <div className="text-gray-300">Found pinned World Monitor assets on IPFS:</div>
            {ipfsCids.map((c) => (
              <div key={c} className="flex items-center justify-between bg-[#111] px-3 py-2 rounded">
                <div className="text-xs text-[#00e5ff] truncate">{c}</div>
                <div className="flex items-center gap-2">
                  <a className="text-xs text-[#ff6b35] underline" href={ipfsGatewayForCid(c)} target="_blank" rel="noreferrer">Open (gateway)</a>
                  <button
                    className="text-xs bg-[#2a2a2a] px-2 py-1 rounded border border-gray-700 text-gray-300"
                    onClick={() => {
                      try { localStorage.setItem('WORLDMONITOR_ISPF_CID', c); alert('Saved ISPF CID to localStorage'); } catch(e) { /* ignore */ }
                    }}
                  >Use ISPF CID</button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-gray-500">No World Monitor bundle pinned to local IPFS. Use the cloud fallback or install the app into Tauri app-data.</div>
        )}

        <div className="mt-4">
          <label className="text-xs text-gray-400">Manual ISPF CID</label>
          <div className="flex gap-2 mt-2">
            <input id="ispf-cid-input" className="flex-1 px-3 py-2 bg-[#0f0f12] border border-gray-800 rounded text-sm text-white" placeholder="paste ispf CID here" />
            <button
              className="px-4 py-2 bg-[#2a2a2a] text-gray-300 rounded border border-gray-700"
              onClick={() => {
                try {
                  const el = document.getElementById('ispf-cid-input') as HTMLInputElement | null;
                  if (el && el.value.trim()) {
                    const cid = el.value.trim();
                    localStorage.setItem('WORLDMONITOR_ISPF_CID', cid);
                    setIpfsCids([cid]);
                    // prefer custom gateway if set via env
                    const gateway = (typeof import.meta !== 'undefined' && (import.meta as any).env?.VITE_ISPF_GATEWAY) || (typeof process !== 'undefined' && (process.env as any).VITE_ISPF_GATEWAY);
                    const targetUrl = gateway ? `${String(gateway).replace(/\/$/, '')}/ipfs/${cid}/` : `ispf://${cid}`;
                    setUrl(targetUrl);
                    el.value = '';
                  }
                } catch (e) { /* ignore */ }
              }}
            >Save & Use</button>
          </div>

          <div className="mt-6 flex gap-2">
            <button
              className="px-4 py-2 bg-[#ff6b35] text-white rounded"
              onClick={() => setUrl(CANDIDATE_URLS[CANDIDATE_URLS.length - 1])}
            >
              Open cloud fallback
            </button>
            <button
              className="px-4 py-2 bg-[#2a2a2a] text-gray-300 rounded border border-gray-700"
              onClick={() => window.open('https://worldmonitor.app', '_blank')}
            >
              Open in browser
            </button>
          </div>
        </div>
      </div>
    );
  }

  // While probing, show the iframe-panel connecting UI
  if (url === null) {
    return <IframePanel url={CANDIDATE_URLS[0]} title="World Monitor" sandbox={"allow-scripts allow-same-origin"} />;
  }

  // When we have a URL (IPFS gateway or remote), render inside an isolated iframe.
  // If the source is IPFS or remote, pass a conservative sandbox to reduce attack surface.
  const useSandbox = /ipfs|worldmonitor\.app|localhost|127\.0\.0\.1/.test(url);

  return (
    <IframePanel
      url={url}
      title="World Monitor"
      sandbox={useSandbox ? "allow-scripts allow-same-origin allow-forms" : undefined}
    />
  );
};

export default WorldMonitorPanel;
