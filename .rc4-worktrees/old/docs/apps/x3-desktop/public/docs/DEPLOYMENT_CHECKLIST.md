# Deployment Checklist & Quick Start

## 📋 Pre-Deployment Checklist

### Local Development Validation

- [ ] Clone repository: `git clone ...`
- [ ] Install dependencies: `npm install`
- [ ] Start dev server: `npm run dev`
- [ ] Test dashboard runs on `http://localhost:5173`
- [ ] Verify all 7 pages load without errors
- [ ] Test WebSocket connections (if available)

### Build Verification

- [ ] Run production build: `npm run build`
- [ ] Verify build succeeds with no errors
- [ ] Check bundle sizes are acceptable (<2MB main):
  - Expected main: ~140 KB
  - Expected vendor: ~560 KB
- [ ] Start preview: `npm run preview`
- [ ] Test preview at `http://localhost:5000`

### Testing (Optional but Recommended)

- [ ] Run unit tests: `npm test`
- [ ] Check test coverage: `npm run test:coverage`
- [ ] Run E2E tests: `npm run e2e` (requires local dashboard running)
- [ ] All tests pass ✅

---

## 🔑 GitHub Secrets Configuration

### Step 1: Generate SSH Keys

**For Staging**:
```bash
ssh-keygen -t ed25519 -f ~/.ssh/dashboard-staging -C "github-actions-staging"
# Press Enter for empty passphrase
# Files created: dashboard-staging (private), dashboard-staging.pub (public)
```

**For Production**:
```bash
ssh-keygen -t ed25519 -f ~/.ssh/dashboard-prod -C "github-actions-prod"
# Press Enter for empty passphrase
```

### Step 2: Add Public Keys to Servers

**On Staging Server**:
```bash
ssh ubuntu@staging.x3-chain.com

# Add public key to authorized_keys
mkdir -p ~/.ssh
echo "$(cat ~/.ssh/dashboard-staging.pub)" >> ~/.ssh/authorized_keys
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys

# Verify (should return 'ubuntu')
whoami
```

**On Production Server**:
```bash
ssh ubuntu@prod.x3-chain.com

# Same steps as staging
mkdir -p ~/.ssh
echo "$(cat ~/.ssh/dashboard-prod.pub)" >> ~/.ssh/authorized_keys
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

### Step 3: Prepare Secret Values

**Get Docker Token** (for ghcr.io):
1. Visit: https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Select scopes: `write:packages`, `read:packages`, `delete:packages`
4. Generate and copy token (won't show again!)

**Get Slack Webhook**:
1. Go to: https://api.slack.com/apps
2. Create New App → "From scratch"
3. App name: "GitHub Actions"
4. Select workspace
5. Go to "Incoming Webhooks" → "Add New Webhook to Workspace"
6. Select channel (e.g., #deployments)
7. Copy the webhook URL

### Step 4: Add Secrets to GitHub

1. Go to: GitHub repo → **Settings** → **Secrets and variables** → **Actions**
2. Click: **New repository secret**

**Add these secrets**:

| Name | Value |
|------|-------|
| `DOCKER_USERNAME` | `your-github-username` |
| `DOCKER_PASSWORD` | *(GitHub token from Step 3)* |
| `STAGING_HOST` | `staging.x3-chain.com` (or IP) |
| `STAGING_USER` | `ubuntu` |
| `STAGING_KEY` | *(Contents of ~/.ssh/dashboard-staging)* |
| `PROD_HOST` | `prod.x3-chain.com` (or IP) |
| `PROD_USER` | `ubuntu` |
| `PROD_KEY` | *(Contents of ~/.ssh/dashboard-prod)* |
| `SLACK_WEBHOOK_URL` | *(Webhook from Step 3)* |

### Step 5: Verify Secrets

- [ ] 8 secrets added to GitHub
- [ ] No trailing spaces in values
- [ ] SSH keys include -----BEGIN/END headers
- [ ] Docker token is valid (test: `docker login -u USERNAME -p TOKEN ghcr.io`)
- [ ] Slack webhook works (test: `curl -X POST ...` example in SECRETS_SETUP.md)

---

## 🚀 Deployment Process

### Staging Deployment

**Automatic on `develop` branch push**:

1. Make changes locally:
   ```bash
   git checkout develop
   # Make your changes...
   ```

2. Commit and push:
   ```bash
   git add .
   git commit -m "Update dashboard: feature description"
   git push origin develop
   ```

3. GitHub Actions automatically:
   - ✅ Builds your code
   - ✅ Runs tests
   - ✅ Checks security
   - ✅ Creates Docker image
   - ✅ Deploys to staging via SSH
   - ✅ Sends Slack notification

4. Monitor in:
   - GitHub Actions tab: See workflow status
   - Slack: Receive deployment notification
   - Staging URL: https://staging.x3-chain.com

5. **Verify**:
   - [ ] Staging dashboard loads
   - [ ] All pages work
   - [ ] No console errors
   - [ ] WebSocket connects
   - [ ] API calls work

### Production Deployment

**Automatic on `main` branch push**:

1. After testing on staging, merge to main:
   ```bash
   git checkout main
   git merge develop
   git push origin main
   ```

2. GitHub Actions automatically:
   - ✅ Builds your code
   - ✅ Runs all tests
   - ✅ Security checks
   - ✅ Creates Docker image
   - ✅ Pushes to GitHub Container Registry
   - ✅ Deploys to production via SSH
   - ✅ Creates GitHub Release
   - ✅ Sends Slack notification

3. Monitor deployment:
   - GitHub Actions → Select run → Monitor steps
   - Watch logs in real-time
   - Receive Slack notification on completion

4. **Verify Production**:
   - [ ] Production dashboard loads
   - [ ] All pages accessible
   - [ ] Performance acceptable
   - [ ] No errors in console
   - [ ] Real metrics displaying

---

## 🐳 Docker Deployment (Alternative)

### Build Docker Image Locally

```bash
# Build image
docker build -t dashboard:latest .

# Run image
docker run -p 3000:80 dashboard:latest

# Test at http://localhost:3000
```

### Push to Registry

```bash
# Login
docker login ghcr.io -u USERNAME -p TOKEN

# Tag image
docker tag dashboard:latest ghcr.io/yourorg/dashboard:latest

# Push
docker push ghcr.io/yourorg/dashboard:latest
```

### Deploy with Docker Compose

```bash
# Production environment
docker-compose -f docker-compose.yml up -d

# Development environment
docker-compose -f docker-compose.dev.yml up
```

---

## ☸️ Kubernetes Deployment

### Prerequisites

- [ ] kubectl installed and configured
- [ ] Access to Kubernetes cluster
- [ ] Namespace exists: `gpu-swarm`

### Deploy

```bash
# Create namespace if needed
kubectl create namespace gpu-swarm

# Apply manifest (ConfigMap, Deployment, Service, HPA, PDB, NetworkPolicy)
kubectl apply -f kubernetes-manifest.yml

# Wait for deployment
kubectl rollout status deployment/gpu-swarm-dashboard -n gpu-swarm

# Verify pod is running
kubectl get pods -n gpu-swarm

# Check service
kubectl get svc -n gpu-swarm
```

### Access Dashboard

```bash
# Port forward for testing
kubectl port-forward -n gpu-swarm svc/gpu-swarm-dashboard 3000:80

# Access at http://localhost:3000
```

### Monitor

```bash
# View logs
kubectl logs -f deployment/gpu-swarm-dashboard -n gpu-swarm

# Watch pods
kubectl get pods -n gpu-swarm -w

# Check events
kubectl describe deployment gpu-swarm-dashboard -n gpu-swarm
```

### Scale Manually

```bash
# Scale to specific number
kubectl scale deployment gpu-swarm-dashboard -n gpu-swarm --replicas=5

# See HPA status
kubectl get hpa -n gpu-swarm -w
```

---

## 🔍 Monitoring & Troubleshooting

### GitHub Actions Logs

1. **View Workflow Run**:
   - Go to Actions tab
   - Click workflow run
   - See detailed logs for each step

2. **Common Failures**:
   - **Linting error**: See lint errors in logs, fix locally
   - **Type error**: Run `npm run type-check` locally to debug
   - **Build failure**: Run `npm run build` locally to reproduce
   - **Deploy failure**: Check SSH credentials and server connectivity

### Performance Monitoring

```bash
# Lighthouse audit
npm run build
npm run preview  # Run this in another terminal
make lighthouse

# Bundle analysis
npm run build
npx webpack-bundle-analyzer dist/assets/index-*.js
```

### Server Monitoring

**SSH into server**:
```bash
ssh ubuntu@staging.x3-chain.com

# Check running processes
ps aux | grep dashboard

# View logs
sudo journalctl -u dashboard -f

# Check disk space
df -h

# Monitor resources
top
```

### Docker Container Monitoring

```bash
# View running containers
docker ps

# View logs
docker logs -f dashboard

# Get container stats
docker stats

# Inspect container
docker inspect dashboard
```

### Kubernetes Pod Monitoring

```bash
# Real-time pod status
kubectl get pods -n gpu-swarm -w

# Pod logs
kubectl logs -f deployment/gpu-swarm-dashboard -n gpu-swarm

# Previous pod logs (if crashed)
kubectl logs pod-name -n gpu-swarm --previous

# Pod details
kubectl describe pod pod-name -n gpu-swarm

# Resource usage
kubectl top pods -n gpu-swarm
```

---

## 🔄 Rollback Procedures

### Rollback Last Deploy

**GitHub Actions**:
1. Go to Actions tab
2. Select the workflow run before the failed one
3. Click "Re-run all jobs"
4. This redeploys the previous version

**Kubernetes**:
```bash
# View rollout history
kubectl rollout history deployment/gpu-swarm-dashboard -n gpu-swarm

# Rollback to previous
kubectl rollout undo deployment/gpu-swarm-dashboard -n gpu-swarm

# Rollback to specific revision
kubectl rollout undo deployment/gpu-swarm-dashboard -n gpu-swarm --to-revision=2
```

**Manual Revert**:
```bash
# If using SSH deployment
ssh ubuntu@prod.x3-chain.com
cd /var/www/dashboard
git revert HEAD
git push

# Or restore from backup
git checkout previous-commit
```

---

## ✅ Post-Deployment Validation

### Immediate (within 5 minutes)

- [ ] Dashboard accessible: `curl https://dashboard.x3-chain.com`
- [ ] Returns HTTP 200
- [ ] HTML content contains "Dashboard"
- [ ] No server errors in logs

### Short-term (within 1 hour)

- [ ] All pages load: Dashboard, GPU, Tasks, Network, etc.
- [ ] Charts render correctly
- [ ] WebSocket connects (check console)
- [ ] API calls work (check Network tab)
- [ ] No JavaScript errors (check console)
- [ ] Performance acceptable (check Lighthouse)

### Medium-term (within 24 hours)

- [ ] Monitor uptime
- [ ] Check error logs
- [ ] Verify metrics are updating in real-time
- [ ] Performance stays consistent
- [ ] No security warnings

### Quarterly Review

- [ ] Performance trend analysis
- [ ] Dependency updates
- [ ] Security patches
- [ ] Capacity planning (if needed)
- [ ] User feedback integration

---

## 📞 Support & Debugging

### Get Help

1. **GitHub Issues**: Create issue with:
   - Error message
   - Steps to reproduce
   - Environment (browser, OS)
   - Relevant logs (sanitized)

2. **Slack Channel**: #dashboard-support
   - For quick questions
   - Share error messages
   - Ask for deployment help

3. **Check Documentation**:
   - [CI_CD_GUIDE.md](CI_CD_GUIDE.md) - Pipeline details
   - [SECRETS_SETUP.md](SECRETS_SETUP.md) - Secret configuration
   - [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
   - [BUILD_REPORT.md](BUILD_REPORT.md) - Build information

### Debug Commands

```bash
# Check application health
npm run build  # Build locally to test
npm run test   # Run tests
npm run lint   # Check for issues

# Check deployment config
cat .env.staging   # View staging config (don't commit secrets!)
code .github/workflows/dashboard-ci-cd.yml  # Review workflow

# Check Docker
docker build . -t test
docker run -p 3000:80 test
```

---

## 🎓 Learning Resources

- **React & TypeScript**: [React Docs](https://react.dev)
- **Vite Build**: [Vite Guide](https://vitejs.dev/)
- **GitHub Actions**: [Actions Docs](https://docs.github.com/actions)
- **Kubernetes**: [K8s Docs](https://kubernetes.io/docs/)
- **Docker**: [Docker Docs](https://docs.docker.com/)

---

## 📊 Next Steps

After successful deployment:

1. **Set Up Monitoring**:
   - [ ] Configure Prometheus scraping
   - [ ] Set up Grafana dashboards
   - [ ] Configure alert rules

2. **Add Tests**:
   - [ ] Write Jest unit tests
   - [ ] Write Cypress E2E tests
   - [ ] Aim for 70%+ coverage

3. **Performance Optimization**:
   - [ ] Run Lighthouse baseline
   - [ ] Optimize Core Web Vitals
   - [ ] Monitor bundle size

4. **Security Hardening**:
   - [ ] Enable rate limiting
   - [ ] Add CORS headers
   - [ ] Enable CSP
   - [ ] Set up WAF rules

5. **Documentation**:
   - [ ] Create runbooks
   - [ ] Document API endpoints
   - [ ] Create troubleshooting guides

---

**Status**: 🟢 Ready for Deployment
**Last Updated**: February 2026
