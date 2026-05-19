#!/bin/bash
# Deploy X3 Chain infrastructure to Kubernetes
# Usage: ./scripts/k8s-deploy.sh [apply|delete|status]

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
K8S_DIR="$PROJECT_ROOT/k8s"
NAMESPACE="x3-chain"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $*"
}

# ── Validation ───────────────────────────────────────────────────────
if ! command -v kubectl &> /dev/null; then
    log_error "kubectl not found. Install kubectl to deploy to Kubernetes."
    exit 1
fi

if ! kubectl cluster-info &> /dev/null; then
    log_error "Not connected to a Kubernetes cluster."
    exit 1
fi

# ── Functions ────────────────────────────────────────────────────────
deploy() {
    log_step "Deploying X3 Chain infrastructure to Kubernetes"
    
    # Check if namespace exists
    if ! kubectl get namespace $NAMESPACE &> /dev/null; then
        log_info "Creating namespace: $NAMESPACE"
        kubectl create namespace $NAMESPACE
    fi
    
    # Apply manifests in order
    log_step "1/6 Applying Namespace and ConfigMaps"
    kubectl apply -f "$K8S_DIR/01-namespace.yaml"
    kubectl apply -f "$K8S_DIR/02-configmaps.yaml"
    
    log_step "2/6 Applying Secrets"
    kubectl apply -f "$K8S_DIR/03-secrets.yaml"
    
    log_step "3/6 Applying PersistentVolumeClaims"
    kubectl apply -f "$K8S_DIR/04-pvcs.yaml"
    
    log_step "4/6 Applying PostgreSQL Database"
    kubectl apply -f "$K8S_DIR/07-postgres-statefulset.yaml"
    
    log_step "5/6 Applying Validator StatefulSet"
    kubectl apply -f "$K8S_DIR/05-validators-statefulset.yaml"
    
    log_step "6/6 Applying Indexer Deployment"
    kubectl apply -f "$K8S_DIR/06-indexer-deployment.yaml"
    
    log_info "Deployment manifests applied successfully!"
    log_info "Waiting for pods to start..."
    
    # Wait for validators to be ready
    log_step "Waiting for validators to reach Running state (timeout: 5m)"
    kubectl wait --for=condition=ready pod -l app=x3-chain,component=validator \
        --namespace=$NAMESPACE --timeout=300s || true
    
    # Wait for indexer to be ready
    log_step "Waiting for indexer to reach Running state (timeout: 2m)"
    kubectl wait --for=condition=ready pod -l app=x3-indexer,component=indexer \
        --namespace=$NAMESPACE --timeout=120s || true
    
    log_info "Deployment complete!"
    display_status
}

delete() {
    log_warn "Deleting X3 Chain infrastructure from Kubernetes"
    
    # Delete in reverse order
    kubectl delete -f "$K8S_DIR/06-indexer-deployment.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/05-validators-statefulset.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/07-postgres-statefulset.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/04-pvcs.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/03-secrets.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/02-configmaps.yaml" --namespace=$NAMESPACE || true
    kubectl delete -f "$K8S_DIR/01-namespace.yaml" || true
    
    log_info "Resources deleted successfully!"
}

status() {
    log_info "X3 Chain Kubernetes Deployment Status"
    echo ""
    
    log_step "Namespace Status"
    kubectl get namespace $NAMESPACE || echo "Namespace not found"
    echo ""
    
    log_step "Pod Status"
    kubectl get pods -n $NAMESPACE -o wide
    echo ""
    
    log_step "Service Status"
    kubectl get svc -n $NAMESPACE
    echo ""
    
    log_step "StatefulSet Status"
    kubectl get statefulset -n $NAMESPACE
    echo ""
    
    log_step "Deployment Status"
    kubectl get deployment -n $NAMESPACE
    echo ""
    
    log_step "Validator Logs (last 20 lines)"
    kubectl logs -n $NAMESPACE -l app=x3-chain,component=validator --tail=20 --all-containers=true || true
    echo ""
    
    log_step "Indexer Logs (last 20 lines)"
    kubectl logs -n $NAMESPACE -l app=x3-indexer,component=indexer --tail=20 --all-containers=true || true
}

display_status() {
    echo ""
    log_info "Deployment Summary:"
    echo ""
    echo "  Namespace:  $NAMESPACE"
    echo "  Validators: 3 (StatefulSet)"
    echo "  Indexer:    2 (Deployment, HA)"
    echo "  Database:   PostgreSQL (StatefulSet)"
    echo ""
    
    log_step "Access Points:"
    
    # Get service endpoints
    RPC_LB=$(kubectl get svc x3-validator-rpc -n $NAMESPACE -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "pending")
    INDEXER_LB=$(kubectl get svc x3-indexer -n $NAMESPACE -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "pending")
    
    echo "  RPC Endpoint:        http://${RPC_LB}:9933 (LoadBalancer)"
    echo "  GraphQL Endpoint:    http://${INDEXER_LB}:4000 (LoadBalancer)"
    echo "  Internal DNS:"
    echo "    - Validator P2P:   x3-validators.x3-chain (headless service)"
    echo "    - Validator RPC:   x3-validator-rpc.x3-chain:9933"
    echo "    - Indexer:         x3-indexer-internal.x3-chain:4000"
    echo "    - Database:        postgres.x3-chain:5432"
    echo ""
    
    log_step "Next Steps:"
    echo "  1. Monitor pods:         kubectl get pods -n $NAMESPACE -w"
    echo "  2. Check logs:           kubectl logs -n $NAMESPACE -f <pod-name>"
    echo "  3. Verify consensus:     kubectl exec -n $NAMESPACE x3-validator-0 -- curl localhost:9933 2>/dev/null | jq"
    echo "  4. Check indexer:        curl -X POST http://\${INDEXER_IP}:4000/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{__typename}\"}'
    echo ""
}

# ── Main ─────────────────────────────────────────────────────────────
ACTION="${1:-status}"

case "$ACTION" in
    apply|deploy)
        deploy
        ;;
    delete|destroy|teardown)
        delete
        ;;
    status)
        status
        ;;
    logs)
        kubectl logs -n $NAMESPACE -f -l app=x3-chain,component=validator --all-containers=true
        ;;
    *)
        log_error "Invalid action: $ACTION"
        echo "Usage: $0 [apply|delete|status|logs]"
        exit 1
        ;;
esac
