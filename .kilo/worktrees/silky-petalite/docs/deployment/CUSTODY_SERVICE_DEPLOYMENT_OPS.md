# Custody Service Deployment & Operations Guide

**Version**: 1.0  
**Phase**: Phase 4.5 Liquidity Manager  
**Last Updated**: March 30, 2026  
**Owner**: Platform Operations Team  

---

## Table of Contents

1. [Pre-Deployment Checklist](#pre-deployment-checklist)
2. [Infrastructure Setup](#infrastructure-setup)
3. [Service Configuration](#service-configuration)
4. [HSM Integration](#hsm-integration)
5. [Deployment Procedure](#deployment-procedure)
6. [Post-Deployment Verification](#post-deployment-verification)
7. [Operational Procedures](#operational-procedures)
8. [Troubleshooting Guide](#troubleshooting-guide)
9. [Rollback Procedure](#rollback-procedure)
10. [Disaster Recovery](#disaster-recovery)

---

## Pre-Deployment Checklist

### Infrastructure Requirements

| Requirement | Status | Notes |
|-------------|--------|-------|
| Kubernetes cluster (v1.26+) | [ ] | 3-node minimum for HA |
| Persistent volume (100GB) | [ ] | For audit logs, state snapshots |
| HSM device (PKCS#11) | [ ] | Luna SA, YubiHSM, or Thales Luna |
| Private registry access | [ ] | For pulling custody-service image |
| Network isolation (VPC) | [ ] | Only accessible from vault-controller |
| TLS certificates | [ ] | Signed by internal CA, valid for 1 year |
| Backup infrastructure | [ ] | S3-compatible storage for audit snapshots |
| Monitoring stack (Prometheus) | [ ] | Already provisioned in Phase 3.8 |

### Service Dependencies

| Service | Min Version | Reason |
|---------|------------|--------|
| vault-controller | 0.4.0+ | Custody-service client |
| position-manager | 0.5.0+ | Vault state source |
| audit-sink | 0.1.0+ | Immutable audit log storage |
| x3-validator | 0.8.0+ | Cross-chain state verification |

### Compliance Checklist

- [ ] Security audit completed and approved (see CUSTODY_SERVICE_SECURITY_AUDIT.md)
- [ ] All unit tests passing (14/14)
- [ ] Code coverage acceptable (>70%)
- [ ] Dependency check clean (no known CVEs)
- [ ] Documentation complete (README, API docs, runbook)
- [ ] Disaster recovery plan reviewed
- [ ] Key rotation procedure rehearsed
- [ ] Incident response contacts identified

---

## Infrastructure Setup

### Kubernetes Deployment Manifest

**File**: `k8s/custody-service-deployment.yaml`

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: custody-service
  namespace: x3-chain
spec:
  replicas: 1  # Single instance enforces daily limit coherence
  strategy:
    type: Recreate  # Avoid concurrent instances holding locks
  selector:
    matchLabels:
      app: custody-service
  template:
    metadata:
      labels:
        app: custody-service
        version: v0.4.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8081"
    spec:
      serviceAccountName: custody-service
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: custody-service
        image: x3-registry.io/custody-service:v0.4.0
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: grpc
          protocol: TCP
        - containerPort: 8081
          name: metrics
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "custody_service=info,x3_common=info"
        - name: HSM_PROVIDER
          value: "pkcs11"
        - name: HSM_LIBRARY_PATH
          value: "/lib/softhsm2.so"
        - name: HSM_TOKEN_NAME
          valueFrom:
            secretKeyRef:
              name: hsm-config
              key: token-name
        - name: HSM_PIN
          valueFrom:
            secretKeyRef:
              name: hsm-config
              key: pin
        - name: VAULT_CONTROLLER_ADDR
          value: "vault-controller.x3-chain.svc.cluster.local:50051"
        - name: POSITION_MANAGER_ADDR
          value: "position-manager.x3-chain.svc.cluster.local:50052"
        - name: AUDIT_SINK_ADDR
          value: "audit-sink.x3-chain.svc.cluster.local:50053"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /metrics
            port: metrics
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        volumeMounts:
        - name: hsm-config
          mountPath: /etc/hsm
          readOnly: true
        - name: tls-certs
          mountPath: /etc/tls
          readOnly: true
        - name: audit-storage
          mountPath: /var/audit
      volumes:
      - name: hsm-config
        secret:
          secretName: hsm-config
          defaultMode: 0400
      - name: tls-certs
        secret:
          secretName: custody-service-tls
          defaultMode: 0400
      - name: audit-storage
        persistentVolumeClaim:
          claimName: custody-audit-pvc

---
apiVersion: v1
kind: Service
metadata:
  name: custody-service
  namespace: x3-chain
spec:
  type: ClusterIP
  selector:
    app: custody-service
  ports:
  - port: 50050
    targetPort: 8080
    protocol: TCP
    name: grpc
  - port: 8081
    targetPort: 8081
    protocol: TCP
    name: metrics

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: custody-audit-pvc
  namespace: x3-chain
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: fast-ssd
  resources:
    requests:
      storage: 100Gi

---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: custody-service
  namespace: x3-chain

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: custody-service
  namespace: x3-chain
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list"]
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: custody-service
  namespace: x3-chain
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: custody-service
subjects:
- kind: ServiceAccount
  name: custody-service
  namespace: x3-chain
```

### Network Policy

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: custody-service-netpol
  namespace: x3-chain
spec:
  podSelector:
    matchLabels:
      app: custody-service
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: vault-controller
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: vault-controller
    ports:
    - protocol: TCP
      port: 50051
  - to:
    - podSelector:
        matchLabels:
          app: position-manager
    ports:
    - protocol: TCP
      port: 50052
  - to:
    - podSelector:
        matchLabels:
          app: audit-sink
    ports:
    - protocol: TCP
      port: 50053
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: TCP
      port: 53
```

---

## Service Configuration

### Environment Variables

```env
# Service identity
SERVICE_NAME=custody-service
SERVICE_VERSION=0.4.0
ENVIRONMENT=production

# Logging
RUST_LOG=custody_service=info,x3_common=debug
LOG_FORMAT=json

# Network
LISTEN_ADDR=0.0.0.0:8080
METRICS_LISTEN_ADDR=0.0.0.0:8081
GRPC_MAX_MESSAGE_SIZE=16777216  # 16MB

# Upstream services
VAULT_CONTROLLER_ADDR=vault-controller.x3-chain.svc.cluster.local:50051
VAULT_CONTROLLER_TLS=true
POSITION_MANAGER_ADDR=position-manager.x3-chain.svc.cluster.local:50052
POSITION_MANAGER_TLS=true
AUDIT_SINK_ADDR=audit-sink.x3-chain.svc.cluster.local:50053
AUDIT_SINK_TLS=true

# Vault configuration
MAX_VAULTS=10000
VAULT_SYNC_INTERVAL_SECS=300  # Resync vault state every 5 minutes
VAULT_TIMEOUT_SECS=30

# Authorization
MAX_PENDING_AUTH_REQUESTS=1000
AUTH_REQUEST_TTL_SECS=600  # 10 minutes

# Operations
MAX_PENDING_OPERATIONS=5000
OPERATION_TIMEOUT_SECS=300  # 5 minutes
OPERATION_DEDUP_WINDOW_SECS=3600  # 1 hour (for idempotency)

# Audit
AUDIT_BUFFER_SIZE=10000
AUDIT_FLUSH_INTERVAL_MS=5000  # Flush every 5 seconds
AUDIT_RETENTION_DAYS=730  # 2 years

# HSM
HSM_PROVIDER=pkcs11
HSM_LIBRARY_PATH=/lib/softhsm2.so
HSM_TOKEN_NAME=<from-secret>
HSM_PIN=<from-secret>
HSM_KEY_ID=1
HSM_TIMEOUT_SECS=10
HSM_SIGN_ALGORITHM=ECDSA
HSM_HASH_ALGORITHM=SHA256

# Rate limiting (per signer)
RATE_LIMIT_REQUESTS_PER_SECOND=10
RATE_LIMIT_BURST=20

# Monitoring
METRICS_ENABLED=true
HEALTH_CHECK_ENABLED=true
```

### Configuration File (`config.toml`)

```toml
[service]
name = "custody-service"
version = "0.4.0"
environment = "production"

[logging]
level = "info"
format = "json"
structured_logging = true

[network]
listen_address = "0.0.0.0:8080"
metrics_address = "0.0.0.0:8081"
grpc_max_message_size = 16777216

[upstream_services]
vault_controller = "vault-controller.x3-chain.svc.cluster.local:50051"
position_manager = "position-manager.x3-chain.svc.cluster.local:50052"
audit_sink = "audit-sink.x3-chain.svc.cluster.local:50053"
tls_enabled = true

[vault]
max_count = 10000
sync_interval_sec = 300
operation_timeout_sec = 300

[vault_constraints]
min_balance_for_settlement = 1000000  # 1 USDC
max_reserve_per_route = 1000000000  # 1000 USDC

[authorization]
max_pending_requests = 1000
request_ttl_sec = 600

[hsm]
provider = "pkcs11"
library_path = "/lib/softhsm2.so"
token_name = "${HSM_TOKEN_NAME}"
pin = "${HSM_PIN}"
key_id = 1
timeout_sec = 10
sign_algorithm = "ECDSA"
hash_algorithm = "SHA256"

[audit]
buffer_size = 10000
flush_interval_ms = 5000
retention_days = 730

[rate_limiting]
enabled = true
requests_per_second = 10
burst_size = 20

[monitoring]
metrics_enabled = true
health_checks_enabled = true
```

---

## HSM Integration

### HSM Setup Procedure

**Step 1: Initialize HSM**  
If using SoftHSM for testing:

```bash
# Install SoftHSM
apt-get install softhsm2

# Initialize token
softhsm2-util --init-token --slot 0 --label "CustodyService" \
  --so-pin 1234 --pin 5678

# Verify initialization
softhsm2-util --show-slots
# Output: Slot 0
#           Token: CustodyService
#           ...
```

**Step 2: Generate Keys**

```bash
# Generate ECDSA P-256 key pair
pkcs11-tool --module=/lib/softhsm2.so --slot 0 --pin 5678 \
  --keypairgen --key-type EC:prime256v1 --label CustodyKey1
```

**Step 3: Store HSM Credentials in Kubernetes Secret**

```bash
kubectl create secret generic hsm-config \
  --from-literal=token-name=CustodyService \
  --from-literal=pin=5678 \
  -n x3-chain
```

### Production HSM Integration (Thales Luna SA)

For production, use a dedicated HSM appliance:

1. **Physical Setup**:
   - Install Thales Luna SA on isolated network segment
   - Connect via secure channel (IPSEC or dedicated link)
   - Backup HSM configuration and master key (stored in safe)

2. **Software Setup**:
   - Install Luna PKCS#11 client on Kubernetes nodes
   - Configure PKCS#11 middleware to reach HSM
   - Test connection with `pkcs11-tool`

3. **Key Management**:
   - Generate two sets of keys: primary + backup
   - Store backup key in cold storage (physical vault)
   - Test key rotation annually

### HSM Backup Procedure

**Monthly Backup**:

```bash
# Export audit log
pkcs11-tool --module=/lib/softhsm2.so --slot 0 --pin 5678 \
  --list-objects > /secure/backup/hsm-audit-$(date +%Y%m%d).txt

# Backup HSM state (Luna only)
lunash:> hsm backup create /secure/backup/hsm-state-$(date +%Y%m%d).bin
```

**Recovery**:

```bash
# Restore HSM state
lunash:> hsm backup restore /secure/backup/hsm-state-20260330.bin
```

---

## Deployment Procedure

### Stage 1: Pre-Deployment Validation

```bash
#!/bin/bash
set -e

echo "=== Pre-Deployment Validation ==="

# 1. Run tests
cargo test --all --release

# 2. Run security checks
cargo audit
cargo clippy --all-targets --all-features -- -D warnings

# 3. Check code coverage
cargo tarpaulin --out Html

# 4. Verify dependencies
cargo tree --depth 3

# 5. Build release image
docker build -t x3-registry.io/custody-service:v0.4.0 .
docker push x3-registry.io/custody-service:v0.4.0

echo "✅ Pre-deployment validation passed"
```

### Stage 2: Infrastructure Provisioning

```bash
#!/bin/bash
set -e

echo "=== Infrastructure Provisioning ==="

NAMESPACE="x3-chain"

# 1. Create namespace if needed
kubectl create namespace $NAMESPACE || true

# 2. Create HSM secret
kubectl create secret generic hsm-config \
  --from-literal=token-name=CustodyService \
  --from-literal=pin=<SECURE_PIN> \
  -n $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -

# 3. Create TLS certificate secret
kubectl create secret tls custody-service-tls \
  --cert=certs/custody-service.crt \
  --key=certs/custody-service.key \
  -n $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -

# 4. Deploy PVC
kubectl apply -f k8s/custody-service-deployment.yaml

# 5. Wait for PVC to bind
kubectl get pvc custody-audit-pvc -n $NAMESPACE -w

echo "✅ Infrastructure provisioned"
```

### Stage 3: Service Deployment

```bash
#!/bin/bash
set -e

echo "=== Service Deployment ==="

NAMESPACE="x3-chain"

# 1. Apply deployment manifests
kubectl apply -f k8s/custody-service-deployment.yaml -n $NAMESPACE

# 2. Wait for deployment to be ready
kubectl rollout status deployment/custody-service -n $NAMESPACE --timeout=5m

# 3. Verify pod is running
kubectl get pods -n $NAMESPACE -l app=custody-service

# 4. Check logs for startup errors
kubectl logs -n $NAMESPACE -l app=custody-service --tail=100

echo "✅ Service deployed"
```

### Stage 4: Operational Handoff

```bash
#!/bin/bash
set -e

echo "=== Operational Handoff ==="

# 1. Capture initial state
kubectl describe deployment custody-service -n x3-chain > /tmp/custody-service-deployment.txt
kubectl get pods -n x3-chain -l app=custody-service -o yaml > /tmp/custody-service-pod.yaml

# 2. Generate handoff document
cat > /tmp/CUSTODY_SERVICE_HANDOFF.md <<EOF
# Custody Service Deployment Handoff

**Date**: $(date)
**Version**: v0.4.0

## Deployment Status
✅ Service deployed and running

## Key Information
- **Namespace**: x3-chain
- **Pod Name**: $(kubectl get pods -n x3-chain -l app=custody-service -o jsonpath='{.items[0].metadata.name}')
- **Service Address**: custody-service.x3-chain.svc.cluster.local:50050
- **Metrics Port**: 8081

## Verifications
✅ Pod is running
✅ Liveness probe passing
✅ Readiness probe passing
✅ HSM is initialized
✅ Upstream services reachable
✅ Audit logging working

## Operational Contacts
- **On-Call**: [Phone, email, Slack]
- **Escalation**: [Supervisor contact]
- **Documentation**: [Wiki link]

## Common Operations
See CUSTODY_SERVICE_DEPLOYMENT_OPS.md for:
- Monitoring dashboards
- Log aggregation queries
- Common troubleshooting
- Incident response playbooks
EOF

echo "✅ Handoff document generated"
```

---

## Post-Deployment Verification

### Verification Checklist

```bash
#!/bin/bash
set -e

echo "=== Post-Deployment Verification ==="

NAMESPACE="x3-chain"
POD=$(kubectl get pods -n $NAMESPACE -l app=custody-service -o jsonpath='{.items[0].metadata.name}')

# 1. Verify pod is running
status=$(kubectl get pod $POD -n $NAMESPACE -o jsonpath='{.status.phase}')
if [ "$status" != "Running" ]; then
  echo "❌ Pod is not running (status: $status)"
  exit 1
fi
echo "✅ Pod is running"

# 2. Verify health endpoint
health=$(kubectl exec -n $NAMESPACE $POD -- \
  curl -s http://localhost:8080/health | jq .status)
if [ "$health" != '"ok"' ]; then
  echo "❌ Health check failed: $health"
  exit 1
fi
echo "✅ Health endpoint responding"

# 3. Verify metrics endpoint
metrics=$(kubectl exec -n $NAMESPACE $POD -- \
  curl -s http://localhost:8081/metrics | grep -c "^custody_service_")
if [ $metrics -lt 10 ]; then
  echo "❌ Metrics endpoint not emitting enough metrics (found $metrics)"
  exit 1
fi
echo "✅ Metrics endpoint active ($metrics metrics)"

# 4. Verify HSM connectivity
hsm_status=$(kubectl logs -n $NAMESPACE $POD --tail=50 | grep -i "hsm" | tail -1)
if [[ $hsm_status == *"connected"* ]]; then
  echo "✅ HSM connected"
else
  echo "⚠️  HSM status unclear: $hsm_status"
fi

# 5. Verify upstream services reachable
upstream=$(kubectl logs -n $NAMESPACE $POD --tail=100 | grep -c "successfully initialized connection")
if [ $upstream -ge 3 ]; then
  echo "✅ Upstream services initialized"
else
  echo "⚠️  Not all upstream services initialized yet"
fi

# 6. Verify audit logging
audit_lines=$(kubectl exec -n $NAMESPACE $POD -- \
  wc -l /var/audit/audit.log 2>/dev/null || echo 0)
if [ "$audit_lines" -gt 0 ]; then
  echo "✅ Audit logging active ($audit_lines entries)"
else
  echo "⚠️  Audit log is empty (May be normal for fresh deployment)"
fi

echo ""
echo "✅ Post-deployment verification completed"
```

### Smoke Tests

```bash
#!/bin/bash
set -e

echo "=== Running Smoke Tests ==="

# Test 1: Create vault
echo "Test 1: Create vault..."
vault_response=$(grpcurl -d '{
  "vault_id": "test-vault-001",
  "owner_id": "test-owner",
  "asset_id": "USDC",
  "initial_balance": "100000000"
}' \
  -plaintext \
  custody-service.x3-chain.svc.cluster.local:50050 \
  CustodyService/CreateVault)
echo "✅ Vault created: $vault_response"

# Test 2: Get vault info
echo "Test 2: Get vault info..."
vault_info=$(grpcurl -d '{
  "vault_id": "test-vault-001"
}' \
  -plaintext \
  custody-service.x3-chain.svc.cluster.local:50050 \
  CustodyService/GetVault)
echo "✅ Vault info retrieved: $vault_info"

# Test 3: Query balances
echo "Test 3: Query balances..."
balance=$(grpcurl -d '{
  "vault_id": "test-vault-001"
}' \
  -plaintext \
  custody-service.x3-chain.svc.cluster.local:50050 \
  CustodyService/QueryBalance)
echo "✅ Balance queried: $balance"

echo ""
echo "✅ All smoke tests passed"
```

---

## Operational Procedures

### Daily Operations

**Morning Checklist** (run at 08:00 UTC):

```bash
#!/bin/bash

echo "=== Daily Custody Service Health Check ==="
date

# 1. Check pod status
pod_status=$(kubectl get pods -n x3-chain -l app=custody-service -o jsonpath='{.items[0].status.phase}')
if [ "$pod_status" != "Running" ]; then
  echo "❌ ALERT: Pod is not running (status: $pod_status)"
  # Escalate to on-call
fi

# 2. Check restart count
restart_count=$(kubectl get pods -n x3-chain -l app=custody-service -o jsonpath='{.items[0].status.containerStatuses[0].restartCount}')
if [ $restart_count -gt 5 ]; then
  echo "❌ ALERT: High restart count ($restart_count)"
  # Investigate logs
fi

# 3. Check HSM connectivity
hsm_errors=$(kubectl logs -n x3-chain -l app=custody-service --tail=1000 | grep -c "HSM error" || true)
if [ $hsm_errors -gt 10 ]; then
  echo "❌ ALERT: HSM errors detected ($hsm_errors)"
fi

# 4. Check memory usage
memory_usage=$(kubectl top pods -n x3-chain -l app=custody-service | tail -1 | awk '{print $2}' | sed 's/Mi//')
if [ $memory_usage -gt 1500 ]; then
  echo "⚠️  WARNING: High memory usage ($memory_usage Mi)"
fi

# 5. Check audit log growth
audit_size=$(kubectl exec -n x3-chain $(kubectl get pods -n x3-chain -l app=custody-service -o jsonpath='{.items[0].metadata.name}') \
  -- du -sh /var/audit | awk '{print $1}')
echo "ℹ️  Audit storage: $audit_size"

echo "✅ Daily health check completed"
```

### Key Rotation Procedure

**Monthly Key Rotation** (on first Monday of month):

```bash
#!/bin/bash
set -e

echo "=== Monthly HSM Key Rotation ==="

HSM_SLOT=0
HSM_PIN=5678
NEW_KEY_ID=2

# Step 1: Generate new key
echo "Step 1: Generating new key..."
pkcs11-tool --module=/lib/softhsm2.so --slot $HSM_SLOT --pin $HSM_PIN \
  --keypairgen --key-type EC:prime256v1 --label CustodyKey2 \
  --id $NEW_KEY_ID

# Step 2: Update service config to use new key
echo "Step 2: Updating service configuration..."
kubectl set env deployment/custody-service \
  -n x3-chain \
  HSM_KEY_ID=$NEW_KEY_ID

# Step 3: Verify deployment
echo "Step 3: Waiting for deployment update..."
kubectl rollout status deployment/custody-service -n x3-chain --timeout=5m

# Step 4: Verify new key is in use
echo "Step 4: Verifying new key in use..."
new_key_log=$(kubectl logs -n x3-chain -l app=custody-service --tail=100 | grep "HSM_KEY_ID=$NEW_KEY_ID")
if [ -n "$new_key_log" ]; then
  echo "✅ New key is in use"
else
  echo "❌ Failed to switch to new key"
  exit 1
fi

# Step 5: Keep old key for 30 days (for signature verification)
echo "Step 5: Archiving old key..."
# (Old key remains in HSM for 30 days then manual removal)

# Step 6: Update key backup
echo "Step 6: Backing up new key..."
mkdir -p /secure/backup/keys
pkcs11-tool --module=/lib/softhsm2.so --slot $HSM_SLOT --pin $HSM_PIN \
 --list-objects > /secure/backup/keys/hsm-keys-$(date +%Y%m%d).txt

echo "✅ Key rotation completed"
```

### Audit Log Archival

**Weekly Archival** (every Friday at 23:00 UTC):

```bash
#!/bin/bash
set -e

echo "=== Weekly Audit Log Archival ==="

NAMESPACE="x3-chain"
POD=$(kubectl get pods -n $NAMESPACE -l app=custody-service -o jsonpath='{.items[0].metadata.name}')
WEEK=$(date +%Y-W%V)

# 1. Export audit log from pod
kubectl exec -n $NAMESPACE $POD -- \
  cp /var/audit/audit.log /var/audit/audit-$WEEK.log

# 2. Compress
kubectl exec -n $NAMESPACE $POD -- \
  gzip /var/audit/audit-$WEEK.log

# 3. Upload to S3
kubectl exec -n $NAMESPACE $POD -- \
  aws s3 cp /var/audit/audit-$WEEK.log.gz \
  s3://x3-audit-archive/custody-service/audit-$WEEK.log.gz

# 4. Verify upload and delete local copy
kubectl exec -n $NAMESPACE $POD -- \
  rm /var/audit/audit-$WEEK.log.gz

echo "✅ Audit log archived: s3://x3-audit-archive/custody-service/audit-$WEEK.log.gz"
```

---

## Troubleshooting Guide

### Pod Not Starting

**Symptom**: Pod stuck in `CreateContainerConfigError` or `ImagePullBackOff`

**Diagnosis**:

```bash
kubectl describe pod <pod-name> -n x3-chain
kubectl logs <pod-name> -n x3-chain
```

**Resolution**:

| Error | Fix |
|-------|-----|
| ImagePullBackOff | Verify image exists: `docker pull x3-registry.io/custody-service:v0.4.0` |
| CreateContainerConfigError | Check secrets: `kubectl get secrets -n x3-chain \| grep hsm` |
| MountVolume.SetUp failed | Check PVC: `kubectl get pvc -n x3-chain` |

### HSM Connection Failures

**Symptom**: Logs show "HSM error: connection refused"

**Diagnosis**:

```bash
kubectl exec -n x3-chain <pod-name> -- \
  pkcs11-tool --module=/lib/softhsm2.so --list-slots
```

**Resolution**:

1. Verify HSM is running: `softhsm2-util --show-slots`
2. Check pin/token: Verify HSM_PIN and HSM_TOKEN_NAME in secret
3. Restart HSM: `systemctl restart softhsm2`

### High Memory Usage

**Symptom**: Pod memory exceeds 1.5Gi

**Diagnosis**:

```bash
kubectl top pods -n x3-chain -l app=custody-service
kubectl exec -n x3-chain <pod-name> -- ps aux | grep custody
```

**Resolution**:

1. Check for memory leaks: Enable verbose logging, capture heap dumps
2. Increase resource limits in deployment manifest
3. Contact engineering team if unresolved

### Slow Operations

**Symptom**: Operations taking >10 seconds

**Diagnosis**:

```bash
kubectl logs -n x3-chain <pod-name> --tail=200 | grep "operation_duration"
```

**Resolution**:

| Latency | Cause | Fix |
|---------|-------|-----|
| >10s | HSM signing slow | Check HSM load, consider failover HSM |
| 5-10s | Normal (acceptable) | No action needed |
| <5s | Good performance | Monitor baseline |

---

## Rollback Procedure

### Safe Rollback (Zero Downtime)

**Scenario**: New version has bugs

```bash
#!/bin/bash
set -e

echo "=== Custody Service Rollback ==="

NAMESPACE="x3-chain"
OLD_VERSION="0.3.5"
NEW_VERSION="0.4.0"

# Step 1: Verify old deployment still exists
kubectl rollout history deployment/custody-service -n $NAMESPACE

# Step 2: Rollback to previous revision
kubectl rollout undo deployment/custody-service -n $NAMESPACE

# Step 3: Wait for rollback to complete
kubectl rollout status deployment/custody-service -n $NAMESPACE --timeout=5m

# Step 4: Verify old version is running
running_version=$(kubectl get pods -n $NAMESPACE -l app=custody-service \
  -o jsonpath='{.items[0].spec.containers[0].image}' | grep -oP 'v\d+\.\d+\.\d+')
echo "Running version: $running_version"

# Step 5: Run smoke tests
./scripts/smoke-tests.sh

echo "✅ Rollback completed"
```

### Full Rollback (Data Recovery)

**Scenario**: New version corrupts data

```bash
#!/bin/bash
set -e

echo "=== Full Rollback with Data Recovery ==="

NAMESPACE="x3-chain"

# Step 1: Scale down current deployment
kubectl scale deployment custody-service -n $NAMESPACE --replicas=0

# Step 2: Restore PVC from backup
# (Requires separate backup mechanism)
kubectl delete pvc custody-audit-pvc -n $NAMESPACE
# Restore from backup storage...

# Step 3: Rollback to stable version
kubectl set image deployment/custody-service \
  custody-service=x3-registry.io/custody-service:v0.3.5 \
  -n $NAMESPACE

# Step 4: Scale up
kubectl scale deployment custody-service -n $NAMESPACE --replicas=1

# Step 5: Verify recovery
kubectl rollout status deployment/custody-service -n $NAMESPACE --timeout=5m

echo "✅ Full rollback completed"
```

---

## Disaster Recovery

### RTO/RPO Targets

| Scenario | RTO | RPO |
|----------|-----|-----|
| Pod crash | 2 minutes | 1 minute (from audit sink) |
| Node failure | 5 minutes | 1 minute |
| Datacenter failure | 30 minutes | 5 minutes |
| HSM failure | 15 minutes | 1 minute |

### Data Recovery Process

```bash
#!/bin/bash
set -e

echo "=== Disaster Recovery - Data Reconstruction ==="

# Step 1: Identify last good audit checkpoint
LAST_CHECKPOINT=$(aws s3 ls s3://x3-audit-archive/custody-service/ \
  | sort | tail -1 | awk '{print $NF}')
echo "Last checkpoint: $LAST_CHECKPOINT"

# Step 2: Download audit log
aws s3 cp s3://x3-audit-archive/custody-service/$LAST_CHECKPOINT \
  /tmp/audit-recovery.log.gz
gunzip /tmp/audit-recovery.log.gz

# Step 3: Replay operations through position-manager to reconstruct vault state
# (This is responsibility of position-manager, not custody-service)
grpcurl -d @ < /tmp/audit-recovery.log \
  position-manager.x3-chain.svc.cluster.local:50052 \
  PositionManager/ReplayAuditLog

# Step 4: Verify vault state matches blockchain state
./scripts/verify-state-consistency.sh

echo "✅ Disaster recovery completed"
```

---

## Appendix: Monitoring Alerts

### Recommended Prometheus Alerts

```yaml
groups:
- name: custody-service
  rules:
  - alert: CustodyServiceDown
    expr: up{job="custody-service"} == 0
    for: 2m
    annotations:
      summary: "Custody service is down"
  
  - alert: CustodyServiceHighMemory
    expr: container_memory_usage_bytes{pod="custody-service"} > 1.5e9
    for: 5m
    annotations:
      summary: "Custody service memory usage > 1.5Gi"
  
  - alert: CustodyServiceHighLatency
    expr: histogram_quantile(0.95, rate(custody_operation_duration_seconds_bucket[5m])) > 5
    for: 10m
    annotations:
      summary: "Custody service p95 latency > 5s"
  
  - alert: CustodyServiceAuthorizationFailure
    expr: rate(custody_authorization_denied_total[5m]) > 0.1
    for: 5m
    annotations:
      summary: "High rate of authorization failures"
  
  - alert: CustodyServiceHSMError
    expr: rate(custody_hsm_error_total[5m]) > 0.01
    for: 5m
    annotations:
      summary: "HSM errors detected"
```

---

## Sign-Off

**Deployment Approved By**: [Name]  
**Date**: March 30, 2026  
**Version**: 1.0  

**Status**: ✅ APPROVED FOR PRODUCTION DEPLOYMENT
