# Deploy Explorer (in‑house / Kubernetes + Helm)

Quick step‑by‑step to deploy the `apps/explorer` frontend to your on‑prem / in‑house Kubernetes cluster using the Helm chart bundled in this repo.

Prerequisites
- kubectl configured for target cluster (context set to `x3-testnet` or equivalent)
- helm >= 3.8 installed
- Docker credentials (or internal registry) accessible from CI / runners
- TLS secret `explorer-tls-cert` created in `x3-chain` namespace (cert-manager will create it automatically when using Let's Encrypt)

Build & push image (example)
1. Build locally (CI will do this automatically):

   docker build -f deployment/docker/Dockerfile.explorer -t ghcr.io/<ORG>/x3-chain-explorer:<SHA> ./apps/explorer
   docker push ghcr.io/<ORG>/x3-chain-explorer:<SHA>

Helm deploy (manual)
1. Install / upgrade from repo chart (overrides image tag):

   helm upgrade --install explorer deployment/helm/explorer \
     --namespace x3-chain --create-namespace \
     --set image.repository=ghcr.io/<ORG>/x3-chain-explorer \
     --set image.tag=<SHA> \
     --wait --timeout 120s

2. Verify:

   kubectl get pods -n x3-chain -l app=explorer
   kubectl describe svc explorer -n x3-chain
   kubectl get ingress -n x3-chain
   curl -f https://explorer.testnet.x3-chain.io/ || echo "unreachable"

Rollbacks & updates
- Roll back to previous release:
  helm rollback explorer 1 -n x3-chain
- Update values (example change replica count):
  helm upgrade explorer deployment/helm/explorer --set replicaCount=3 -n x3-chain

Autoscaling (HPA)
- Enable autoscaling using the chart values or overrides:
  helm upgrade --install explorer deployment/helm/explorer \
    --namespace x3-chain --set autoscaling.enabled=true \
    --set autoscaling.minReplicas=2 --set autoscaling.maxReplicas=6 \
    --set autoscaling.targetCPUUtilizationPercentage=70

Canary / rollout example
- Quick canary using a separate release & host (manual approach):
  helm upgrade --install explorer-canary deployment/helm/explorer \
    --namespace x3-chain --create-namespace \
    --set image.tag=<CANARY_SHA> \
    --set replicaCount=1 \
    --set ingress.host=canary.explorer.testnet.x3-chain.io

  Verify the canary pod and check behaviour; when ready promote by updating the main release image tag.

Health-check & graceful shutdown
- Chart exposes `healthPath` (default `/`) and `lifecycle.preStopSleepSeconds`.
- To use a custom health endpoint:
  helm upgrade explorer deployment/helm/explorer --set healthPath=/api/health -n x3-chain
- Graceful termination is controlled by `lifecycle.preStopSleepSeconds` (default 10s).

Notes
- Chart path: `deployment/helm/explorer/` (default and env-specific values available)
- TLS secret name expected by chart: `explorer-tls-cert` (adjust `values.yaml` if different)
- CI updates image tag automatically and runs `helm upgrade --install explorer` when pushing to `main` (see `.github/workflows/production-deploy.yml`).

Troubleshooting
- If ingress TLS not issued check cert-manager logs and `Certificate`/`Ingress` events.
- For fast local testing you can use `kubectl port-forward svc/explorer 3000:3000 -n x3-chain` and visit http://localhost:3000

Contact
- DevOps / infra: devops@x3-chain.io
