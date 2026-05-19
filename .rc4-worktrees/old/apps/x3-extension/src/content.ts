// content.ts — X3 Chain content script
// Injected into every page that matches the manifest's content_scripts rule.
// Exposes a minimal `window.x3Provider` object that dApps can use to send
// signing and query requests to the extension background service worker.
//
// The provider surface intentionally mirrors the Ethereum provider API shape
// (EIP-1193) so that dApps familiar with MetaMask can adapt with minimal
// friction.

(window as unknown as Record<string, unknown>).x3Provider = {
    /** Semver version of the X3 provider API. */
    version: '0.1.0',

    /**
     * Send an RPC request through the extension background.
     *
     * @param method - JSON-RPC 2.0 method name (e.g. `x3_canonicalSnapshot`).
     * @param params - Positional parameters for the method.
     * @returns A promise that resolves with the background response.
     */
    request: async (method: string, params: unknown[]): Promise<unknown> => {
        return new Promise((resolve) => {
            chrome.runtime.sendMessage({ type: 'RPC_REQUEST', method, params }, resolve);
        });
    },
};
