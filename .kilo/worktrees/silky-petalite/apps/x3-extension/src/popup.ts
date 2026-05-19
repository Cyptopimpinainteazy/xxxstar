// popup.ts — X3 Chain extension popup UI logic
// Queries the background service worker for the latest canonical snapshot and
// any pending drift alerts, then renders them into the popup DOM.

interface CanonicalSnapshot {
    blockNumber: number;
    identityRoot: string;
    assetRoot: string;
    treasuryRoot: string;
    capturedAtMs: number;
}

interface DriftAlert {
    surface: string;
    blockNumber: number;
    kind: 'identity' | 'asset' | 'treasury';
    detail: string;
    detectedAtMs: number;
}

document.addEventListener('DOMContentLoaded', () => {
    // ── Snapshot display ──────────────────────────────────────────────────────
    chrome.runtime.sendMessage({ type: 'GET_SNAPSHOT' }, (resp: { snapshot: CanonicalSnapshot | null }) => {
        const snap = resp?.snapshot;
        const el = document.getElementById('snapshot');
        if (el) {
            el.textContent = snap
                ? `Block: ${snap.blockNumber} | Identity: ${snap.identityRoot.slice(0, 8)}... | Asset: ${snap.assetRoot.slice(0, 8)}...`
                : 'No snapshot yet — waiting for node connection';
        }
    });

    // ── Recent alerts display ─────────────────────────────────────────────────
    chrome.runtime.sendMessage({ type: 'GET_ALERTS' }, (resp: { alerts: DriftAlert[] }) => {
        const alerts: DriftAlert[] = resp?.alerts ?? [];
        const el = document.getElementById('alerts');
        if (el) {
            if (alerts.length === 0) {
                el.innerHTML = '<li style="color:#4caf50">No alerts — in sync with on-chain truth</li>';
            } else {
                el.innerHTML = alerts
                    .map(
                        (a) =>
                            `<li style="color:#e57373">${a.kind} drift at block ${a.blockNumber}</li>`,
                    )
                    .join('');
            }
        }
    });
});
