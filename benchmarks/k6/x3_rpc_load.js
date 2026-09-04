// X3 Chain — k6 RPC / WebSocket Load Test
//
// Run: k6 run --vus 50 --duration 60s benchmarks/k6/x3_rpc_load.js
//
// Endpoints tested:
//   - eth_call (read-only)
//   - eth_sendRawTransaction (write)
//   - eth_getLogs (historical query)
//   - x3_submitAtomicSwap
//   - x3_quoteMultiVmSwap
//   - x3_getBridgeStatus
//   - x3_estimateAtomicFee
//   - x3_getValidatorMetrics
//   - WebSocket subscribe / unsubscribe

import http from 'k6/http';
import { check, sleep, fail } from 'k6';
import { Trend, Counter, Rate } from 'k6/metrics';
import ws from 'k6/ws';

const RPC_URL = __ENV.RPC_URL || 'http://127.0.0.1:9933';
const WS_URL  = __ENV.WS_URL  || 'ws://127.0.0.1:9944';

// Custom metrics
const ethCallLatency = new Trend('eth_call_latency');
const ethSendRawLatency = new Trend('eth_sendRawTransaction_latency');
const ethGetLogsLatency = new Trend('eth_getLogs_latency');
const atomicSwapSubmitLatency = new Trend('x3_submitAtomicSwap_latency');
const quoteMultiVmLatency = new Trend('x3_quoteMultiVmSwap_latency');
const bridgeStatusLatency = new Trend('x3_getBridgeStatus_latency');
const estimateAtomicFeeLatency = new Trend('x3_estimateAtomicFee_latency');
const validatorMetricsLatency = new Trend('x3_getValidatorMetrics_latency');
const rpcErrors = new Rate('rpc_errors');
const wsReconnects = new Counter('ws_reconnects');

// ─── RPC Helpers ────────────────────────────────────────────────────────────

function rpcCall(method, params = []) {
    const payload = JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: method,
        params: params,
    });

    const response = http.post(RPC_URL, payload, {
        headers: { 'Content-Type': 'application/json' },
    });

    const ok = check(response, {
        'status 200': (r) => r.status === 200,
        'no error in body': (r) => {
            if (r.status !== 200) return false;
            try {
                const body = JSON.parse(r.body);
                return !body.error;
            } catch {
                return false;
            }
        },
    });

    if (!ok) {
        rpcErrors.add(1);
    }
    return response;
}

// ─── eth_call ───────────────────────────────────────────────────────────────

function testEthCall() {
    const t0 = Date.now();
    const resp = rpcCall('eth_call', [{
        to: '0x0000000000000000000000000000000000000001',
        data: '0x',
    }, 'latest']);
    ethCallLatency.add(Date.now() - t0);
}

// ─── eth_sendRawTransaction ─────────────────────────────────────────────────

function testEthSendRawTransaction() {
    // Dummy signed tx (won't actually be valid — measures RPC ingestion)
    const dummyTx = '0x02f8' + '01'.repeat(100);
    const t0 = Date.now();
    const resp = rpcCall('eth_sendRawTransaction', [dummyTx]);
    ethSendRawLatency.add(Date.now() - t0);
}

// ─── eth_getLogs ────────────────────────────────────────────────────────────

function testEthGetLogs() {
    const t0 = Date.now();
    const resp = rpcCall('eth_getLogs', [{
        fromBlock: '0x0',
        toBlock: 'latest',
        address: '0x0000000000000000000000000000000000000001',
        topics: [],
    }]);
    ethGetLogsLatency.add(Date.now() - t0);
}

// ─── x3_submitAtomicSwap ────────────────────────────────────────────────────

function testX3SubmitAtomicSwap() {
    const payload = {
        hashlock: '0x' + 'aa'.repeat(32),
        from_chain: 1,
        to_chain: 2,
        from_amount: '1000000000000000000',
        to_amount: '990000000000000000',
        min_receive: '980000000000000000',
        timelock: Date.now() + 3600 * 1000,
        recipient: '0x' + 'bb'.repeat(20),
    };

    const t0 = Date.now();
    const resp = rpcCall('x3_submitAtomicSwap', [payload]);
    atomicSwapSubmitLatency.add(Date.now() - t0);
}

// ─── x3_quoteMultiVmSwap ────────────────────────────────────────────────────

function testX3QuoteMultiVmSwap() {
    const t0 = Date.now();
    const resp = rpcCall('x3_quoteMultiVmSwap', [{
        token_in: '0x' + '01'.repeat(32),
        token_out: '0x' + '02'.repeat(32),
        amount: '1000000000000000000',
        chains: ['evm', 'svm'],
    }]);
    quoteMultiVmLatency.add(Date.now() - t0);
}

// ─── x3_getBridgeStatus ─────────────────────────────────────────────────────

function testX3GetBridgeStatus() {
    const t0 = Date.now();
    const resp = rpcCall('x3_getBridgeStatus', ['0x' + 'cc'.repeat(32)]);
    bridgeStatusLatency.add(Date.now() - t0);
}

// ─── x3_estimateAtomicFee ───────────────────────────────────────────────────

function testX3EstimateAtomicFee() {
    const t0 = Date.now();
    const resp = rpcCall('x3_estimateAtomicFee', [{
        from_chain: 1,
        to_chain: 2,
        amount: '1000000000000000000',
        legs: 1,
    }]);
    estimateAtomicFeeLatency.add(Date.now() - t0);
}

// ─── x3_getValidatorMetrics ─────────────────────────────────────────────────

function testX3GetValidatorMetrics() {
    const t0 = Date.now();
    const resp = rpcCall('x3_getValidatorMetrics', []);
    validatorMetricsLatency.add(Date.now() - t0);
}

// ─── WebSocket Subscription ─────────────────────────────────────────────────

function testWebSocket() {
    const url = WS_URL;
    let reconnectCount = 0;

    const res = ws.connect(url, {}, function (socket) {
        socket.on('open', () => {
            // Subscribe to new heads
            socket.send(JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method: 'chain_subscribeNewHeads',
                params: [],
            }));

            // Unsubscribe after 2s
            socket.setTimeout(() => {
                socket.close();
            }, 2000);
        });

        socket.on('message', (msg) => {
            try {
                const data = JSON.parse(msg);
                if (data.result !== undefined) {
                    // Got subscription ID, done
                }
            } catch {}
        });

        socket.on('close', () => {
            reconnectCount++;
            wsReconnects.add(1);
        });

        socket.on('error', (e) => {
            rpcErrors.add(1);
        });
    });

    check(res, { 'ws connected': (r) => r && r.status === 101 });
}

// ─── k6 Lifecycle ───────────────────────────────────────────────────────────

export const options = {
    stages: [
        { duration: '10s', target: 10 },   // ramp-up
        { duration: '30s', target: 50 },   // steady load
        { duration: '10s', target: 100 },  // spike
        { duration: '10s', target: 0 },    // ramp-down
    ],
    thresholds: {
        'eth_call_latency': ['p(95)<500', 'p(99)<1000'],
        'eth_sendRawTransaction_latency': ['p(95)<1000'],
        'x3_quoteMultiVmSwap_latency': ['p(95)<2000'],
        'rpc_errors': ['rate<0.05'],
    },
};

export default function () {
    testEthCall();
    sleep(0.1);

    testEthGetLogs();
    sleep(0.1);

    testX3QuoteMultiVmSwap();
    sleep(0.1);

    testX3GetBridgeStatus();
    sleep(0.1);

    testX3EstimateAtomicFee();
    sleep(0.1);

    // Write operations (less frequent to avoid spam)
    if (Math.random() < 0.1) {
        testEthSendRawTransaction();
        sleep(0.05);
    }

    if (Math.random() < 0.05) {
        testX3SubmitAtomicSwap();
        sleep(0.05);
    }

    if (Math.random() < 0.02) {
        testX3GetValidatorMetrics();
        sleep(0.05);
    }

    // WebSocket every 10th iteration
    if (__ITER % 10 === 0) {
        testWebSocket();
    }
}

// ─── Summary Handler ────────────────────────────────────────────────────────

export function handleSummary(data) {
    const summary = {
        timestamp: new Date().toISOString(),
        duration_seconds: data.state.testRunDurationMs / 1000,
        vus_max: data.metrics.vus_max ? data.metrics.vus_max.values.max : 0,
        iterations: data.metrics.iterations.values.count,
        metrics: {
            eth_call_p50: ethCallLatency.values ? ethCallLatency.values['p(50)'] : null,
            eth_call_p95: ethCallLatency.values ? ethCallLatency.values['p(95)'] : null,
            eth_call_p99: ethCallLatency.values ? ethCallLatency.values['p(99)'] : null,
            eth_sendRaw_p95: ethSendRawLatency.values ? ethSendRawLatency.values['p(95)'] : null,
            eth_getLogs_p95: ethGetLogsLatency.values ? ethGetLogsLatency.values['p(95)'] : null,
            atomic_swap_p95: atomicSwapSubmitLatency.values ? atomicSwapSubmitLatency.values['p(95)'] : null,
            quote_multivm_p95: quoteMultiVmLatency.values ? quoteMultiVmLatency.values['p(95)'] : null,
            bridge_status_p95: bridgeStatusLatency.values ? bridgeStatusLatency.values['p(95)'] : null,
            estimate_fee_p95: estimateAtomicFeeLatency.values ? estimateAtomicFeeLatency.values['p(95)'] : null,
            validator_metrics_p95: validatorMetricsLatency.values ? validatorMetricsLatency.values['p(95)'] : null,
            rpc_error_rate: rpcErrors ? rpcErrors.name : null,
            ws_reconnect_total: wsReconnects ? wsReconnects.name : null,
        },
    };

    return {
        'reports/benchmarks/x3-k6-summary.json': JSON.stringify(summary, null, 2),
        stdout: `\nX3 k6 Load Test Summary\n${JSON.stringify(summary, null, 2)}\n`,
    };
}