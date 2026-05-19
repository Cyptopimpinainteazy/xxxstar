/**
 * IframePanel — embeds a URL inside a desktop window via iframe.
 *
 * Uses a fetch-based health check before rendering the iframe, since
 * iframe onError only fires for very specific network-level failures.
 * Includes a load timeout to detect unresponsive servers.
 */
import React, { useState, useEffect, useRef, useCallback } from "react";

interface IframePanelProps {
  url: string;
  title?: string;
  // optional sandbox attribute for increased isolation (pass through to iframe)
  sandbox?: string;
  // optional `allow` attribute (feature policy) for the iframe
  allow?: string;
  // optional referrer-policy for the iframe
  referrerPolicy?: React.HTMLAttributeReferrerPolicy;
}

type Status = "checking" | "loading" | "ready" | "unreachable";

const HEALTH_TIMEOUT = 5_000; // ms to wait for fetch health check
const LOAD_TIMEOUT = 12_000; // ms to wait for iframe to finish loading

interface AppStartupHint {
  appName: string;
  port: string;
  command: string;
  workdir: string;
}

const PORT_HINTS: Record<string, AppStartupHint[]> = {
  "3001": [
    {
      appName: "X3 App Store",
      port: "3001",
      command: "npm run dev",
      workdir: "x3-app-store/frontend/",
    },
  ],
  "5175": [
    {
      appName: "Ollama Code Reviewer",
      port: "5175",
      command: "npm run dev",
      workdir: "ollama-code-reviewer/",
    },
  ],
  "5176": [
    {
      appName: "3aiXchange DEX",
      port: "5176",
      command: "npm run dev",
      workdir: "apps/3ai/dex/frontend/",
    },
  ],
  "8080": [
    {
      appName: "GPU Validator Dashboard",
      port: "8080",
      command: "python -m cross_chain_gpu_validator.cli dashboard --port 8080",
      workdir: "cross-chain-gpu-validator/",
    },
    {
      appName: "Autonomic Control Plane",
      port: "8080",
      command: "python -m swarm.autonomic",
      workdir: "swarm/autonomic/",
    },
  ],
  "8787": [
    {
      appName: "Foundry/Hardhat GUI",
      port: "8787",
      command: "python3 server.py --port 8787",
      workdir: "tools/foundry-hardhat-gui/",
    },
  ],
  "3020": [
    {
      appName: "Blockchain TPS Tester",
      port: "3020",
      command: "PORT=3020 npm start",
      workdir: "infra-structure/services/blockchain-tps/",
    },
  ],
  "9101": [
    {
      appName: "GPU Swarm Node Admin",
      port: "9101",
      command: "cargo run -p gpu-swarm",
      workdir: "crates/gpu-swarm/",
    },
  ],
};

const URL_HINTS: Record<string, AppStartupHint> = {
  "localhost:8080/dashboard.html": {
    appName: "Autonomic Control Plane",
    port: "8080",
    command: "python -m swarm.autonomic",
    workdir: "swarm/autonomic/",
  },
};

const IframePanel: React.FC<IframePanelProps> = ({ url, title, sandbox, allow, referrerPolicy }) => {
  const [status, setStatus] = useState<Status>("checking");
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const loadTimerRef = useRef<ReturnType<typeof setTimeout>>();

  /**
   * Normalize special schemes (ipfs://, ispf://) into HTTP gateway URLs so
   * fetch/iframe can consume them in the browser environment.
   */
  const normalizeUrl = (u: string) => {
    try {
      if (/^ipfs:\/\//i.test(u) || /^ispf:\/\//i.test(u)) {
        // strip scheme and leading slashes
        const cid = u.replace(/^ipfs:\/\//i, '').replace(/^ispf:\/\//i, '');
        // prefer a local gateway when available (developer expectation)
        return `http://127.0.0.1:8080/ipfs/${cid}/`;
      }
      return u;
    } catch {
      return u;
    }
  };

  const resolveStartupHint = useCallback(
    (targetUrl: string): AppStartupHint | null => {
      try {
        const parsed = new URL(normalizeUrl(targetUrl));
        const urlKey = `${parsed.hostname}:${parsed.port}${parsed.pathname}`;
        const direct = URL_HINTS[urlKey];
        if (direct) return direct;

        if (!parsed.port) return null;
        const candidates = PORT_HINTS[parsed.port];
        if (!candidates || candidates.length === 0) return null;
        if (!title) return candidates[0];

        const normalizedTitle = title.toLowerCase();
        return (
          candidates.find((candidate) =>
            normalizedTitle.includes(candidate.appName.toLowerCase())
          ) ?? candidates[0]
        );
      } catch {
        return null;
      }
    },
    [title],
  );

  const startupHint = resolveStartupHint(url);

  /** Probe the URL with fetch to see if the server is alive (normalizes IPFS/ISPF) */
  const checkReachable = useCallback(async () => {
    setStatus("checking");
    const probeUrl = normalizeUrl(url);
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), HEALTH_TIMEOUT);
      await fetch(probeUrl, {
        mode: "no-cors", // we don't need to read the body, just verify reachability
        signal: controller.signal,
      });
      clearTimeout(timer);
      setStatus("loading");
    } catch {
      setStatus("unreachable");
    }
  }, [url]);

  // Run health check on mount / URL change
  useEffect(() => {
    checkReachable();
    return () => {
      if (loadTimerRef.current) clearTimeout(loadTimerRef.current);
    };
  }, [checkReachable]);

  // Start a load-timeout when we transition to "loading"
  useEffect(() => {
    if (status === "loading") {
      loadTimerRef.current = setTimeout(() => {
        // If still "loading" after timeout, the iframe might be stuck
        setStatus("unreachable");
      }, LOAD_TIMEOUT);
    }
    return () => {
      if (loadTimerRef.current) clearTimeout(loadTimerRef.current);
    };
  }, [status]);

  const handleIframeLoad = () => {
    if (loadTimerRef.current) clearTimeout(loadTimerRef.current);
    setStatus("ready");
  };

  const handleRetry = () => {
    // Force iframe to reload too
    if (iframeRef.current) {
      iframeRef.current.src = "";
    }
    checkReachable();
  };

  /* ── Checking / connecting overlay ── */
  if (status === "checking") {
    return (
      <div className="flex items-center justify-center h-full bg-[#0a0a0f]">
        <div className="text-center">
          <div className="inline-block w-6 h-6 border-2 border-[#ff6b35]/30 border-t-[#ff6b35] rounded-full animate-spin mb-3" />
          <div className="text-xs font-mono text-[#888]">Connecting to {title || "app"}…</div>
          <div className="text-[10px] font-mono text-[#555] mt-1">{url}</div>
        </div>
      </div>
    );
  }

  /* ── Unreachable / error overlay ── */
  if (status === "unreachable") {
    return (
      <div className="flex items-center justify-center h-full bg-[#0a0a0f]">
        <div className="text-center max-w-xs">
          <div className="text-3xl mb-3">⚠️</div>
          <div className="text-sm text-[#ff6b35] font-medium mb-2">
            {startupHint
              ? `${startupHint.appName} is not running on port ${startupHint.port}`
              : `Cannot reach ${title || "application"}`}
          </div>
          <div className="text-xs text-[#888] mb-1">
            The app server is not responding at:
          </div>
          <code className="text-[10px] text-[#00e5ff] bg-[#111] px-3 py-1.5 rounded block mb-3">
            {url}
          </code>
          {startupHint ? (
            <div className="text-[10px] text-[#666] mb-4 text-left bg-[#111] rounded px-3 py-2">
              <div className="mb-1">Run:</div>
              <code className="block text-[#9fe7ff]">{startupHint.command}</code>
              <div className="mt-1">
                in <code>{startupHint.workdir}</code>
              </div>
            </div>
          ) : (
            <div className="text-[10px] text-[#666] mb-4">
              Start the server first, then retry.
            </div>
          )}
          <button
            className="text-xs text-[#ff6b35] border border-[#ff6b35]/40 px-4 py-1.5 rounded
              hover:bg-[#ff6b35]/10 transition-colors"
            onClick={handleRetry}
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  /* ── Loading / Ready — show iframe ── */
  return (
    <div className="relative w-full h-full bg-[#0a0a0f]">
      {/* Loading overlay — fades out once iframe signals load */}
      {status === "loading" && (
        <div className="absolute inset-0 flex items-center justify-center z-10 bg-[#0a0a0f]">
          <div className="text-center">
            <div className="inline-block w-6 h-6 border-2 border-[#ff6b35]/30 border-t-[#ff6b35] rounded-full animate-spin mb-3" />
            <div className="text-xs font-mono text-[#888]">Loading {title || "app"}…</div>
          </div>
        </div>
      )}

      <iframe
        ref={iframeRef}
        src={typeof url === 'string' ? (typeof (normalizeUrl) === 'function' ? normalizeUrl(url) : url) : url}
        title={title || "Application"}
        className="w-full h-full border-0"
        onLoad={handleIframeLoad}
        loading="lazy"
        allow={allow ?? "clipboard-read; clipboard-write"}
        sandbox={typeof sandbox !== 'undefined' ? sandbox : undefined}
        referrerPolicy={referrerPolicy ?? "no-referrer"}
      />
    </div>
  );
};

export default IframePanel;
