/**
 * IChainAdapter — abstract adapter interface for blockchain connectors.
 *
 * Each chain family (EVM, Bitcoin, Solana, etc.) implements this interface.
 */
/**
 * Base adapter with shared logic for latency tracking and error counting.
 */
export class BaseChainAdapter {
    endpoint = "";
    connected = false;
    requestCount = 0;
    errorCount = 0;
    startTime = 0;
    latencySamples = [];
    async connect(endpoint) {
        this.endpoint = endpoint;
        this.startTime = Date.now();
        this.connected = true;
    }
    async disconnect() {
        this.connected = false;
    }
    isConnected() {
        return this.connected;
    }
    async getMetrics() {
        const sorted = [...this.latencySamples].sort((a, b) => a - b);
        const p50 = sorted[Math.floor(sorted.length * 0.5)] || 0;
        return {
            blockHeight: 0,
            tps: 0,
            peerCount: 0,
            latencyMs: p50,
            totalRequests: this.requestCount,
            totalErrors: this.errorCount,
            uptimeSeconds: Math.floor((Date.now() - this.startTime) / 1000),
            finalityLag: 0,
        };
    }
    /** Track a request's latency */
    trackRequest(startMs) {
        this.requestCount++;
        this.latencySamples.push(Date.now() - startMs);
        if (this.latencySamples.length > 1000)
            this.latencySamples.shift();
    }
    trackError() {
        this.errorCount++;
    }
    /**
     * Make an RPC call with latency tracking.
     */
    async rpcCall(method, params = []) {
        const start = Date.now();
        try {
            const res = await fetch(this.endpoint, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ jsonrpc: "2.0", id: this.requestCount + 1, method, params }),
            });
            const json = await res.json();
            this.trackRequest(start);
            if (json.error) {
                this.trackError();
                throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
            }
            return json.result;
        }
        catch (err) {
            this.trackError();
            this.trackRequest(start);
            throw err;
        }
    }
    /**
     * Make an HTTP GET call with latency tracking.
     */
    async httpGet(path) {
        const start = Date.now();
        try {
            const url = this.endpoint.endsWith("/") ? this.endpoint + path : `${this.endpoint}/${path}`;
            const res = await fetch(url);
            const json = await res.json();
            this.trackRequest(start);
            return json;
        }
        catch (err) {
            this.trackError();
            this.trackRequest(start);
            throw err;
        }
    }
}
//# sourceMappingURL=base.js.map