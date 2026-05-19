// background.ts — X3 Chain service worker
// Manages the connection to the X3 node RPC, maintains a canonical truth
// snapshot, detects drift between successive snapshots, and raises browser
// notifications when any Merkle root changes.

interface CanonicalSnapshot {
    blockNumber: number;
    identityRoot: string;  // hex-encoded 32-byte Merkle root
    assetRoot: string;     // hex-encoded 32-byte Merkle root
    treasuryRoot: string;  // hex-encoded 32-byte Merkle root
    capturedAtMs: number;
}

interface DriftAlert {
    surface: 'extension';
    blockNumber: number;
    kind: 'identity' | 'asset' | 'treasury';
    detail: string;
    detectedAtMs: number;
}

let latestSnapshot: CanonicalSnapshot | null = null;
const pendingAlerts: DriftAlert[] = [];

// ---------------------------------------------------------------------------
// RPC helpers
// ---------------------------------------------------------------------------

/**
 * Fetch a canonical snapshot from the X3 node via the `x3_canonicalSnapshot`
 * JSON-RPC 2.0 method.  Returns null on any network or parse error so the
 * caller can skip the update cycle silently.
 */
async function fetchSnapshot(rpcUrl: string): Promise<CanonicalSnapshot | null> {
    try {
        const resp = await fetch(rpcUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method: 'x3_canonicalSnapshot',
                params: [],
            }),
        });
        const data = await resp.json();
        return (data.result as CanonicalSnapshot) ?? null;
    } catch {
        return null;
    }
}

// ---------------------------------------------------------------------------
// Drift detection
// ---------------------------------------------------------------------------

/**
 * Determine which root kind has drifted between two consecutive snapshots.
 * Returns the first mismatched root kind found (identity > asset > treasury).
 */
function classifyDrift(
    prev: CanonicalSnapshot,
    next: CanonicalSnapshot,
): 'identity' | 'asset' | 'treasury' {
    if (next.identityRoot !== prev.identityRoot) return 'identity';
    if (next.assetRoot !== prev.assetRoot) return 'asset';
    return 'treasury';
}

// ---------------------------------------------------------------------------
// Polling loop — runs every 30 seconds
// ---------------------------------------------------------------------------

setInterval(async () => {
    const stored = await chrome.storage.local.get(['rpcUrl']);
    const rpcUrl: string =
        typeof stored['rpcUrl'] === 'string' ? stored['rpcUrl'] : 'http://localhost:9933';

    const snap = await fetchSnapshot(rpcUrl);
    if (snap === null) return;

    if (
        latestSnapshot !== null && (
            snap.identityRoot  !== latestSnapshot.identityRoot  ||
            snap.assetRoot     !== latestSnapshot.assetRoot     ||
            snap.treasuryRoot  !== latestSnapshot.treasuryRoot
        )
    ) {
        const kind = classifyDrift(latestSnapshot, snap);
        const alert: DriftAlert = {
            surface: 'extension',
            blockNumber: snap.blockNumber,
            kind,
            detail: 'Canonical truth root changed',
            detectedAtMs: Date.now(),
        };
        pendingAlerts.push(alert);

        // Keep the alert buffer bounded to avoid unbounded memory growth.
        if (pendingAlerts.length > 100) pendingAlerts.splice(0, pendingAlerts.length - 100);

        chrome.notifications.create({
            type: 'basic',
            iconUrl: 'icons/icon48.png',
            title: 'X3 Chain: Canonical Truth Alert',
            message: `${kind} root changed at block ${snap.blockNumber}`,
        });
    }

    latestSnapshot = snap;
    await chrome.storage.local.set({ latestSnapshot: snap });
}, 30_000);

// ---------------------------------------------------------------------------
// Message handler — popup queries
// ---------------------------------------------------------------------------

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg.type === 'GET_SNAPSHOT') {
        sendResponse({ snapshot: latestSnapshot });
    }
    if (msg.type === 'GET_ALERTS') {
        sendResponse({ alerts: pendingAlerts.slice(-10) });
    }
    // Return true so the channel stays open for async responses from other
    // handlers that may be added in future.
    return true;
});
