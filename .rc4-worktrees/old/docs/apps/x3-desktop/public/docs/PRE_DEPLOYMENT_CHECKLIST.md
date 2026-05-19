# Pre-Deployment Verification Checklist

Use this checklist to verify everything is ready before deploying to staging or production.

---

## ✅ Local Development (Required Before Any Deployment)

**Must Pass All Checks Before Proceeding**

### Installation
- [ ] Run `npm install` successfully
- [ ] All dependencies installed (should see "added X packages")
- [ ] No npm error messages
- [ ] `node_modules/` directory exists

### Development Server
- [ ] Start: `npm run dev`
- [ ] Server starts without errors
- [ ] Dashboard accessible at `http://localhost:5173`
- [ ] Hot Module Replacement (HMR) working (edit a file, see live update)
- [ ] Console has no errors
- [ ] No TypeScript warnings

### Page Verification
- [ ] Dashboard page loads ✅
- [ ] GPU Monitoring page loads ✅
- [ ] Task Management page loads ✅
- [ ] Network Topology page loads ✅
- [ ] Economics page loads ✅
- [ ] Governance page loads ✅
- [ ] Settings page loads ✅
- [ ] All navigation works ✅

### API Integration (if mock API available)
- [ ] Check Network tab → XHR/Fetch requests
- [ ] API calls complete without errors
- [ ] Data displays on dashboard
- [ ] WebSocket connections open (if available)

### Code Quality
- [ ] Run: `npm run lint` → No errors ✅
- [ ] Run: `npm run type-check` → 0 errors ✅
- [ ] Run: `npm run format-check` → No issues ✅

---

## ✅ Production Build (Required Before Any Deployment)

**Must Pass All Checks Before Proceeding**

### Build Execution
- [ ] Run: `npm run build`
- [ ] Build completes without errors
- [ ] `dist/` directory created
- [ ] Files in `dist/`: index.html, assets/*, etc.

### Build Metrics
- [ ] Build time: < 10 seconds ✅
- [ ] Main bundle: < 500 KB ✅
- [ ] Vendor bundle: < 1 MB ✅
- [ ] No breaking warnings in output

### Build Verification
- [ ] Run: `npm run preview`
- [ ] Server starts on `http://localhost:5000`
- [ ] Dashboard loads from production build
- [ ] All pages accessible
- [ ] No console errors
- [ ] Performance acceptable (not sluggish)

---

## ✅ Testing (Recommended Before Production)

**Should Pass Most/All Checks Before Production**

### Unit Tests
- [ ] Test framework configured: `jest.config.js` exists
- [ ] Run: `npm test` (at least syntax should work)
- [ ] Coverage config exists
- [ ] Tests can be written and run

### E2E Tests
- [ ] Framework configured: `cypress.config.ts` exists
- [ ] Base URL configured correctly
- [ ] Example tests can be written

### Performance Tests
- [ ] Lighthouse config exists: `.lighthouserc.json`
- [ ] Can run: `npx http-server dist` + Lighthouse analysis

---

## ✅ Docker & Container (Required if Using Docker)

**Must Pass All Checks Before Docker Deployment**

### Docker Build
- [ ] Docker installed: `docker --version`
- [ ] Run: `docker build -t dashboard:test .`
- [ ] Build succeeds without errors
- [ ] Image created: `docker images | grep dashboard`

### Docker Run
- [ ] Run: `docker run -p 3000:80 dashboard:test`
- [ ] Container starts without errors
- [ ] Dashboard accessible at `http://localhost:3000`
- [ ] Container logs show no errors
- [ ] Stop with `Ctrl+C` (graceful shutdown)

### Docker Compose (Dev)
- [ ] Docker Compose installed: `docker-compose --version`
- [ ] Run: `docker-compose -f docker-compose.dev.yml up`
- [ ] All services start (at least dashboard should work)
- [ ] Dashboard accessible
- [ ] Services can be stopped: `Ctrl+C`
- [ ] Run: `docker-compose -f docker-compose.dev.yml down` (cleanup)

### Docker Compose (Production)
- [ ] Run: `docker-compose -f docker-compose.yml up -d`
- [ ] All services start: `docker-compose ps`
- [ ] Dashboard accessible
- [ ] Database/Redis accessible
- [ ] Cleanup: `docker-compose -f docker-compose.yml down`

---

## ✅ GitHub & Workflows (Required Before CI/CD)

**Must Pass All Checks Before GitHub Actions Deployment**

### Repository Setup
- [ ] GitHub repository initialized
- [ ] Code pushed to `develop` branch
- [ ] Code pushed to `main` branch
- [ ] Both branches visible in GitHub

### Workflow Files
- [ ] `.github/workflows/dashboard-ci-cd.yml` exists
- [ ] `.github/workflows/dashboard-docker.yml` exists
- [ ] `.github/workflows/dashboard-performance.yml` exists
- [ ] Files are valid YAML (no syntax errors)

### GitHub Secrets (CRITICAL)
- [ ] Navigate: Settings → Secrets and variables → Actions
- [ ] Verify 9 secrets exist:
  - [ ] `DOCKER_USERNAME` ✅
  - [ ] `DOCKER_PASSWORD` ✅
  - [ ] `STAGING_HOST` ✅
  - [ ] `STAGING_USER` ✅
  - [ ] `STAGING_KEY` ✅
  - [ ] `PROD_HOST` ✅
  - [ ] `PROD_USER` ✅
  - [ ] `PROD_KEY` ✅
  - [ ] `SLACK_WEBHOOK_URL` ✅
- [ ] No extra spaces in secret values
- [ ] SSH keys include `-----BEGIN/END` headers
- [ ] Secret values are actual values (not placeholders)

---

## ✅ Deployment Infrastructure (Required Before Server Deployment)

**Must Pass All Checks Before Actually Deploying**

### Staging Server (If Using SSH)
- [ ] Server is running and accessible
- [ ] SSH access configured: `ssh ubuntu@STAGING_HOST`
- [ ] SSH connection successful (no timeout/refusal)
- [ ] Server has >2GB disk space: `df -h`
- [ ] Server has required ports open:
  - [ ] Port 22 (SSH) ✅
  - [ ] Port 80 (HTTP) ✅
  - [ ] Port 443 (HTTPS) ✅
- [ ] nginx installed (or Docker for containerized)
- [ ] Can create `/var/www/dashboard-staging/` directory
- [ ] Server user (ubuntu) has write permissions

### Production Server (If Using SSH)
- [ ] Same checks as staging
- [ ] Production server is more secure:
  - [ ] Firewall configured ✅
  - [ ] SSH key-only access (no passwords) ✅
  - [ ] Security updates applied ✅
- [ ] Backup procedure exists

### Kubernetes Cluster (If Using K8s)
- [ ] kubectl installed: `kubectl version`
- [ ] Cluster accessible: `kubectl get nodes`
- [ ] Can create namespaces: `kubectl create namespace test-ns`
- [ ] Can list resources: `kubectl get all -A`
- [ ] Namespace `gpu-swarm` ready (or will create)
- [ ] Sufficient resources: CPU & memory limits acceptable

---

## ✅ Configuration Files

**All files must be properly configured**

### Environment Files
- [ ] `.env.development` exists (for local dev)
- [ ] `.env.staging` exists (for staging)
- [ ] `.env.production` exists (for production)
- [ ] API URLs are correct for each environment
- [ ] WebSocket URLs are correct for each environment
- [ ] No secrets in markdown files
- [ ] No `.env` files committed to Git

### Configuration Files
- [ ] `vite.config.ts` exists and valid
- [ ] `tsconfig.json` exists with strict mode
- [ ] `tailwind.config.js` exists
- [ ] `jest.config.js` exists
- [ ] `cypress.config.ts` exists
- [ ] `.lighthouserc.json` exists
- [ ] `nginx.conf` exists
- [ ] `Dockerfile` exists
- [ ] `docker-compose.yml` exists
- [ ] `docker-compose.dev.yml` exists
- [ ] `kubernetes-manifest.yml` exists
- [ ] `Makefile` exists
- [ ] `package.json` exists with all scripts

### Documentation Files
- [ ] `docs/root/README.md` exists
- [ ] `DOCUMENTATION_INDEX.md` exists
- [ ] `docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md` exists (this file)
- [ ] `DEPLOYMENT.md` exists
- [ ] `SECRETS_SETUP.md` exists
- [ ] `CI_CD_GUIDE.md` exists
- [ ] `STATUS_AND_NEXT_STEPS.md` exists
- [ ] `docs/reports/IMPLEMENTATION_SUMMARY.md` exists
- [ ] `BUILD_REPORT.md` exists

---

## ✅ Manual Verification Steps

**Test deployments manually before automating**

### SSH Deployment Test
```bash
# Locally, test SSH connection
ssh -i ~/.ssh/staging-key ubuntu@STAGING_HOST
# Should connect without password
echo "SSH works!"
exit
```

- [ ] SSH connection successful
- [ ] No permission errors
- [ ] Can execute commands

### Docker Push Test (if applicable)

```bash
# Login to Docker registry
docker login -u DOCKER_USERNAME -p DOCKER_PASSWORD

# Test push
docker build -t test:latest .
docker tag test:latest ghcr.io/USERNAME/dashboard:test
docker push ghcr.io/USERNAME/dashboard:test
```

- [ ] Docker login successful
- [ ] Image builds successfully
- [ ] Image pushes to registry
- [ ] Image is accessible from registry

### Kubernetes Test (if applicable)

```bash
# Test kubectl access
kubectl get nodes

# Test namespace
kubectl create namespace gpu-swarm-test
kubectl get namespace gpu-swarm-test

# Cleanup
kubectl delete namespace gpu-swarm-test
```

- [ ] kubectl commands work
- [ ] Can create resources
- [ ] Can view resources
- [ ] Can delete resources

### Slack Webhook Test (if applicable)

```bash
# Test Slack webhook
curl -X POST -H 'Content-type: application/json' \
  --data '{"text":"Test from dashboard deployment"}' \
  YOUR_SLACK_WEBHOOK_URL
```

- [ ] Webhook URL valid
- [ ] Message appears in Slack channel
- [ ] Formatting is acceptable

---

## ✅ Security Verification

**Verify security before any deployment**

### Secrets Management
- [ ] No secrets in code files
- [ ] No secrets in configuration files
- [ ] No secrets in documentation
- [ ] All secrets stored in GitHub Secrets
- [ ] SSH keys are private (not public)
- [ ] Access tokens are temporary (not permanent)

### Access Control
- [ ] Only authorized people have SSH keys
- [ ] GitHub repo access is restricted
- [ ] Secrets access is restricted
- [ ] Server access is restricted
- [ ] Database credentials are rotated

### HTTPS/TLS
- [ ] SSL certificate obtained (if applicable)
- [ ] Certificate is valid and not expired
- [ ] HTTPS configured on servers
- [ ] HTTP redirects to HTTPS
- [ ] Security headers configured

### Dependency Security
- [ ] Run: `npm audit`
- [ ] No critical vulnerabilities
- [ ] No high vulnerabilities (unless acceptable)
- [ ] Dependencies are up-to-date
- [ ] Known bad packages are excluded

---

## ✅ Documentation & Knowledge

**Team should understand deployment**

### Documentation Review
- [ ] Team read `DOCUMENTATION_INDEX.md`
- [ ] Team read `docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md` (this file)
- [ ] Team read `CI_CD_GUIDE.md`
- [ ] Team familiar with Makefile commands
- [ ] Team knows their role in deployment

### Knowledge Check
- [ ] Someone can explain the CI/CD pipeline
- [ ] Someone can troubleshoot SSH issues
- [ ] Someone can view GitHub Actions logs
- [ ] Someone can rollback deployment
- [ ] Someone can monitor production

### Runbooks (Optional but Recommended)
- [ ] Deployment runbook exists
- [ ] Troubleshooting runbook exists
- [ ] Rollback runbook exists
- [ ] Incident response runbook exists
- [ ] Team trained on runbooks

---

## 🚀 Pre-Staging Deployment Checklist

**Final check before first staging deployment**

- [ ] ✅ All "Local Development" checks pass
- [ ] ✅ All "Production Build" checks pass
- [ ] ✅ All "GitHub & Workflows" checks pass
- [ ] ✅ All "Deployment Infrastructure" checks pass
- [ ] ✅ All "Configuration Files" checks pass
- [ ] ✅ All "Manual Verification Steps" pass
- [ ] ✅ All "Security Verification" checks pass
- [ ] ✅ All "Documentation & Knowledge" checks pass

**If any check fails**: STOP and fix before deployment!

---

## 🚀 Pre-Production Deployment Checklist

**Final check before first production deployment**

- [ ] ✅ All staging deployment checks pass
- [ ] ✅ Staging dashboard stable for 24+ hours
- [ ] ✅ All production servers prepared
- [ ] ✅ Backup procedure tested
- [ ] ✅ Rollback procedure tested
- [ ] ✅ Monitoring configured (recommended)
- [ ] ✅ Team on-call and ready
- [ ] ✅ Maintenance window scheduled (if needed)
- [ ] ✅ Stakeholders notified
- [ ] ✅ Everyone ready to proceed

**If any check fails**: STOP and address before production!

---

## 📋 Sign-Off

When all checks pass:

```
Project: GPU Swarm Dashboard
Deployment Target: Staging / Production (circle one)
Date: _______________
Deployed By: _______________
Reviewed By: _______________
Approved By: _______________

All checks verified: ✅ YES / ❌ NO
Ready to deploy: ✅ YES / ❌ NO

Notes:
_______________________________________
_______________________________________

```

---

## 📞 If Checks Fail

**Troubleshooting Guide**

### Build Fails
- Check: `npm run type-check` for TypeScript errors
- Check: `npm run lint` for linting errors
- Solution: Fix errors locally first
- Re-run: `npm run build`

### Docker Fails
- Check: `docker --version` to verify Docker installed
- Check: `docker ps` to verify Docker running
- Solution: Start Docker daemon
- Try again: `docker build .`

### GitHub Secrets Issue
- Check: Settings → Secrets and variables → Actions
- Verify: Secret names match exactly (case-sensitive)
- Verify: No extra spaces in values
- Solution: Fix secret values, re-test

### SSH Connection Fails
- Check: Can you ping the server?
- Check: Is SSH enabled on server? (`ssh -v ...`)
- Check: Do you have the correct private key?
- Solution: Verify key, server IP, username

### Kubernetes Issues
- Check: `kubectl get nodes` - cluster accessible?
- Check: `kubectl get namespace gpu-swarm` - namespace exists?
- Solution: Create namespace: `kubectl create namespace gpu-swarm`

---

## ✨ Success Criteria

When you can answer YES to all of these, you're ready:

- [ ] "The dashboard builds locally without errors"
- [ ] "The dashboard runs locally without errors"
- [ ] "All pages load and display content"
- [ ] "The production build completes successfully"
- [ ] "The Docker image builds successfully"
- [ ] "All GitHub secrets are configured"
- [ ] "The staging server is ready and accessible"
- [ ] "I can SSH to the staging server without errors"
- [ ] "The CI/CD workflows are valid YAML"
- [ ] "I understand the deployment process"
- [ ] "The team is ready to proceed"

---

**Status**: Ready when all checks are ✅

**Next**: Follow [DEPLOYMENT_CHECKLIST.md](../../../../runbooks/deployment/DEPLOYMENT_CHECKLIST.md) for step-by-step deployment

**Questions**: See [DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md) for help
