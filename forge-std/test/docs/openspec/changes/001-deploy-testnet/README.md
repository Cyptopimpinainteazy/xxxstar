Title: Deploy X3 Chain to testnet cluster

Summary:
- Deploy the X3 Chain stack to a Kubernetes test cluster with Prometheus and Grafana.
- Run sustained load testing of 1,000 TPS for acceptance.

Motivation:
- Validate horizontal scaling, GPU usage, and system stability under realistic load.

Scope & Deliverables:
1. Helm chart for x3-chain (scaffolded under `charts/x3-chain`).
2. GitHub Actions workflow to build, push, deploy, and run a k6 benchmark (`.github/workflows/k8s_perf_deploy.yml`).
3. Prometheus ServiceMonitor and Grafana dashboard scaffolds under `monitoring/grafana`.
4. k6 workload scripts under `tests/perf/k6/`.
5. Acceptance criteria: sustained 1k TPS for 10+ minutes with <1% errors and p99 latency target (to be defined; we propose p99 < 500ms initially).

Plan:
- Short: validate Helm deploy and health probes, run a 10m 1k TPS k6 test, iterate on resource sizing.
- Medium: add auto-scaling (HPA) and GPU node pools, introduce canary deployments.
- Long: integrate Loki/ELK and long-term soak tests (24h+), add Grafana alerts.

Required secrets / permissions:
- `DOCKER_REGISTRY`, `DOCKER_USERNAME`, `DOCKER_PASSWORD` for pushing images
- `KUBE_CONFIG` for cluster access
- `POSTGRES_URL` for DB access in test cluster

Notes:
- This is an initial proposal; create an OpenSpec change and link to CI PR with tests and dashboards.
