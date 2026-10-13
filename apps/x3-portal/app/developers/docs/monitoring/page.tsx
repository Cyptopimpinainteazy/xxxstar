'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function MonitoringPage() {
  return (
    <DocLayout
      title="Monitoring & Observability"
      description="Monitor your X3 Atlas Sphere nodes and applications"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Comprehensive monitoring is essential for maintaining healthy nodes and applications.
        Learn how to set up observability for X3 Atlas Sphere infrastructure.
      </p>

      <h2>Built-in Metrics</h2>
      <p>
        X3 nodes expose Prometheus metrics on port 9615 by default:
      </p>
      <CodeBlock language="bash">
{`# Start node with Prometheus metrics
./atlas-sphere-node \\
  --prometheus-port 9615 \\
  --prometheus-external  # Allow external access (use with caution)

# Test metrics endpoint
curl http://localhost:9615/metrics`}
      </CodeBlock>

      <h2>Key Metrics</h2>

      <h3>Block Production</h3>
      <CodeBlock language="promql">
{`# Current block height
substrate_block_height{status="best"}

# Block production rate
rate(substrate_block_height{status="best"}[5m])

# Finalized block height
substrate_block_height{status="finalized"}

# Finality lag (best - finalized)
substrate_block_height{status="best"} - substrate_block_height{status="finalized"}`}
      </CodeBlock>

      <h3>Networking</h3>
      <CodeBlock language="promql">
{`# Connected peers
substrate_sub_libp2p_peers_count

# Network bytes in/out
rate(substrate_sub_libp2p_network_bytes_total[5m])

# Sync status
substrate_sub_libp2p_is_major_syncing`}
      </CodeBlock>

      <h3>Transaction Pool</h3>
      <CodeBlock language="promql">
{`# Transactions in pool
substrate_sub_txpool_validations_scheduled

# Transaction archive/archive/imports
rate(substrate_sub_txpool_submitted_txns[5m])`}
      </CodeBlock>

      <h3>Comit Specific</h3>
      <CodeBlock language="promql">
{`# Comits submitted per minute
rate(atlas_kernel_comits_submitted_total[1m])

# Comit success rate
sum(rate(atlas_kernel_comits_finalized_total[5m])) / 
sum(rate(atlas_kernel_comits_submitted_total[5m]))

# Average Comit execution time
histogram_quantile(0.95, rate(atlas_kernel_comit_execution_seconds_bucket[5m]))`}
      </CodeBlock>

      <h2>Prometheus Setup</h2>
      <CodeBlock language="yaml" title="prometheus.yml">
{`global:
  scrape_interval: 15s
  evaluation_interval: 15s

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

rule_files:
  - "alerts/*.yml"

scrape_configs:
  - job_name: 'atlas-nodes'
    static_configs:
      - targets:
        - 'validator1:9615'
        - 'validator2:9615'
        - 'validator3:9615'
        labels:
          network: 'testnet'
          type: 'validator'
    
  - job_name: 'atlas-rpc'
    static_configs:
      - targets:
        - 'rpc1:9615'
        - 'rpc2:9615'
        labels:
          network: 'testnet'
          type: 'rpc'`}
      </CodeBlock>

      <h2>Alerting Rules</h2>
      <CodeBlock language="yaml" title="alerts/atlas.yml">
{`groups:
  - name: atlas-alerts
    rules:
      - alert: NodeDown
        expr: up{job="atlas-nodes"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Atlas node {{ $labels.instance }} is down"

      - alert: BlockProductionStalled
        expr: increase(substrate_block_height{status="best"}[5m]) == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No new blocks in 5 minutes"

      - alert: HighFinalityLag
        expr: substrate_block_height{status="best"} - substrate_block_height{status="finalized"} > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Finality lag is high ({{ $value }} blocks)"

      - alert: LowPeerCount
        expr: substrate_sub_libp2p_peers_count < 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count: {{ $value }}"

      - alert: HighComitFailureRate
        expr: |
          sum(rate(atlas_kernel_comits_failed_total[5m])) / 
          sum(rate(atlas_kernel_comits_submitted_total[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Comit failure rate > 10%"`}
      </CodeBlock>

      <h2>Grafana Dashboard</h2>
      <p>
        Import our pre-built apps/dash-legacy-2-legacy-2board for X3 Atlas Sphere:
      </p>
      <CodeBlock language="bash">
{`# Download apps/dash-legacy-2-legacy-2board JSON
curl -O https://raw.githubusercontent.com/Cyptopimpinainteazy/atlas-sphere/main/monitoring/grafana-apps/dash-legacy-2-legacy-2board.json

# Import via Grafana UI or API
curl -X POST \\
  -H "Content-Type: application/json" \\
  -d @grafana-apps/dash-legacy-2-legacy-2board.json \\
  http://admin:password@localhost:3000/api/apps/dash-legacy-2-legacy-2boards/db`}
      </CodeBlock>

      <h2>Logging</h2>
      <CodeBlock language="bash">
{`# Set log level
./atlas-sphere-node -l info

# Module-specific logging
./atlas-sphere-node -l atlas_kernel=debug,grandpa=info

# JSON logging for log aggregation
./atlas-sphere-node --log-format json

# Log to file
./atlas-sphere-node 2>&1 | tee /var/log/atlas/node.log`}
      </CodeBlock>

      <h3>Structured Logging with Vector</h3>
      <CodeBlock language="toml" title="vector.toml">
{`[sources.atlas_logs]
type = "file"
include = ["/var/log/atlas/*.log"]

[transforms.parse_json]
type = "remap"
inputs = ["atlas_logs"]
source = '''
. = parse_json!(.message)
'''

[sinks.elasticsearch]
type = "elasticsearch"
inputs = ["parse_json"]
endpoint = "http://elasticsearch:9200"
index = "atlas-logs-%Y-%m-%d"`}
      </CodeBlock>

      <h2>Telemetry</h2>
      <p>
        Submit telemetry to the X3 telemetry server for network-wide visibility:
      </p>
      <CodeBlock language="bash">
{`./atlas-sphere-node \\
  --telemetry-url "wss://telemetry.atlas-sphere.io/submit 0"

# View telemetry apps/dash-legacy-2-legacy-2board
# https://telemetry.atlas-sphere.io`}
      </CodeBlock>

      <h2>Health Checks</h2>
      <CodeBlock language="bash">
{`# Simple health check
curl -s http://localhost:9933/health | jq

# Detailed system health
curl -s -H "Content-Type: application/json" \\
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health"}' \\
  http://localhost:9933 | jq

# Expected response:
# {
#   "peers": 25,
#   "isSyncing": false,
#   "shouldHavePeers": true
# }`}
      </CodeBlock>

      <Callout type="info" title="Monitoring Stack">
        We recommend the Prometheus + Grafana + Alertmanager stack for production 
        monitoring. For logs, use Vector with Elasticsearch or Loki.
      </Callout>
    </DocLayout>
  );
}
