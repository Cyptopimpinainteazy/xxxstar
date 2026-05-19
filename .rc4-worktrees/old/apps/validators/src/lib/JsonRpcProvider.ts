/**
 * X3 JSON-RPC Provider
 * 
 * Generic JSON-RPC provider for interacting with X3 Chain nodes.
 * Supports both HTTP and WebSocket connections.
 */

/**
 * JSON-RPC request options
 */
export interface JsonRpcOptions {
    url: string;
    method: 'POST' | 'GET';
    headers?: Record<string, string>;
    timeout?: number;
}

/**
 * JSON-RPC request payload
 */
export interface JsonRpcRequest {
    jsonrpc: '2.0';
    method: string;
    params?: any[];
    id: number | string;
}

/**
 * JSON-RPC response
 */
export interface JsonRpcResponse<T = any> {
    jsonrpc: '2.0';
    result?: T;
    error?: {
        code: number;
        message: string;
        data?: any;
    };
    id: number | string;
}

/**
 * Subscription callback type
 */
export type SubscriptionCallback<T> = (data: T) => void;

/**
 * Subscription handle
 */
export interface SubscriptionHandle {
    id: string;
    unsubscribe: () => void;
}

/**
 * JSON-RPC Provider
 */
export class JsonRpcProvider {
    private url: string;
    private wsUrl: string;
    private ws: WebSocket | null = null;
    private subscriptions: Map<string, SubscriptionCallback<any>> = new Map();
    private nextId: number = 1;
    private timeout: number;

    constructor(url: string, wsUrl?: string, timeout: number = 30000) {
        this.url = url;
        this.wsUrl = wsUrl || url.replace('http://', 'ws://').replace('https://', 'wss://');
        this.timeout = timeout;
    }

    /**
     * Make a JSON-RPC request
     * @param method - RPC method name
     * @param params - Request parameters
     */
    async request<T>(method: string, params: any[] = []): Promise<T> {
        const payload: JsonRpcRequest = {
            jsonrpc: '2.0',
            method,
            params,
            id: this.nextId++,
        };

        return fetch(this.url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload),
            signal: AbortSignal.timeout(this.timeout),
        }).then(async (response) => {
            if (!response.ok) {
                throw new Error(`HTTP error: ${response.status}`);
            }

            const data: JsonRpcResponse<T> = await response.json();

            if (data.error) {
                throw new Error(`RPC error: ${data.error.message} (code: ${data.error.code})`);
            }

            return data.result as T;
        });
    }

    /**
     * Subscribe to a WebSocket-based RPC method
     * @param method - Subscription method name
     * @param params - Subscription parameters
     * @param callback - Callback function for updates
     */
    subscribe<T>(method: string, params: any[] = [], callback: SubscriptionCallback<T>): Promise<string> {
        return new Promise((resolve, reject) => {
            if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
                this.connectWebSocket().then(() => {
                    this.doSubscribe(method, params, callback, resolve, reject);
                }).catch(reject);
            } else {
                this.doSubscribe(method, params, callback, resolve, reject);
            }
        });
    }

    private connectWebSocket(): Promise<void> {
        return new Promise((resolve, reject) => {
            this.ws = new WebSocket(this.wsUrl);

            this.ws.onopen = () => {
                resolve();
            };

            this.ws.onmessage = (event) => {
                this.handleWebSocketMessage(event);
            };

            this.ws.onerror = (error) => {
                reject(error);
            };

            this.ws.onclose = () => {
                this.ws = null;
            };
        });
    }

    private doSubscribe<T>(
        method: string,
        params: any[],
        callback: SubscriptionCallback<T>,
        resolve: (id: string) => void,
        reject: (error: Error) => void
    ): void {
        const id = `${method}_${this.nextId++}`;
        this.subscriptions.set(id, callback);

        const payload: JsonRpcRequest = {
            jsonrpc: '2.0',
            method,
            params,
            id,
        };

        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            try {
                this.ws.send(JSON.stringify(payload));
                resolve(id);
            } catch (error) {
                reject(error as Error);
            }
        } else {
            reject(new Error('WebSocket not connected'));
        }
    }

    private handleWebSocketMessage(event: MessageEvent): void {
        try {
            const data: JsonRpcResponse<any> = JSON.parse(event.data);

            // Handle subscription notification
            if (data.method && data.params && data.params.subscription) {
                const subscriptionId = data.params.subscription;
                const callback = this.subscriptions.get(subscriptionId);
                if (callback) {
                    callback(data.params.result);
                }
            }
        } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
        }
    }

    /**
     * Unsubscribe from a subscription
     * @param id - Subscription ID
     */
    unsubscribe(id: string): void {
        this.subscriptions.delete(id);

        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            const payload: JsonRpcRequest = {
                jsonrpc: '2.0',
                method: 'unsubscribe',
                params: [id],
                id: this.nextId++,
            };
            this.ws.send(JSON.stringify(payload));
        }
    }

    /**
     * Close the WebSocket connection
     */
    close(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.subscriptions.clear();
    }
}
