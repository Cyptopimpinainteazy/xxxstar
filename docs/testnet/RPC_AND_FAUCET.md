# X3 Public Testnet RPC and Faucet

## RPC Plan

### Public RPC

- Planned HTTPS RPC hostname from infra routing: `https://rpc.x3star.net`
- Planned WebSocket hostname from infra routing: `wss://ws.x3star.net`
- Purpose: wallet and explorer read/write access for testnet users
- Current repo evidence: these hostnames are present in `infra/cloudflare-tunnel/config.yml`
- Launch condition: hostnames must be validated against the public testnet deployment before announcing them externally
- Access controls: rate limiting, request body limits, `safe` RPC methods for validator surfaces, and endpoint-level monitoring

### Validator RPC

- Private validator RPC must not be exposed publicly.
- Validator RPC should be bound to localhost or private network only.
- Current validator deployment shape in `k8s/05-validators-statefulset.yaml` uses:
	- RPC port `9933`
	- `--rpc-methods=safe`
	- Prometheus metrics on `9616`

## Explorer and Dashboard Plan

- Planned explorer hostnames from infra routing: `https://explorer.x3star.net` and `https://blockexplorer.x3star.net`
- Monitoring dashboard stack exists in `deployment/monitoring/docker-compose.yml`
- Internal Grafana default URL during ops bring-up: `http://localhost:3000`
- Owner: Release operations team
- Required visibility: block height, finality lag, node health, transaction lookup
- Launch condition: explorer hostname must point to a real explorer deployment, not the placeholder web service currently present in tunnel config

## Faucet Plan

- Deployment status: not yet deployed in repo-backed public infrastructure
- Required owner before launch: release operations team with dedicated faucet wallet custodian
- Public URL: must be assigned before launch and published together with RPC and explorer announcement
- Funding source: dedicated faucet wallet only
- Treasury isolation: faucet wallet must be separate from treasury and must never use treasury keys

## Faucet Abuse Controls

- Minimum release policy before launch:
	- one funded request per account cooldown window
	- one funded request per source IP cooldown window
	- manual override only by release operations owner
- Anti-bot control: CAPTCHA or equivalent challenge
- Rate limit: IP and account throttling
- Abuse response: temporary blocklist and manual review

## Monitoring and Alerting

- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000`
- Alertmanager: `http://localhost:9093`
- Loki: `http://localhost:3100`
- Validator metrics target: port `9616`

These services are defined in `deployment/monitoring/docker-compose.yml` and validator metrics exposure is defined in `k8s/05-validators-statefulset.yaml`.

## Security Rule

The faucet must never use treasury keys. Any violation is a launch blocker.
