# CI/CD Pipeline Documentation

## Overview

The GPU Swarm Dashboard implements a comprehensive CI/CD pipeline with:

- **Automated Testing** - Unit, integration, and E2E tests
- **Code Quality** - Linting, type checking, formatting
- **Security Scanning** - Dependency audit, vulnerability checks
- **Performance Testing** - Lighthouse, bundle analysis
- **Automated Deployment** - Staging and production deployments
- **Monitoring & Notifications** - Slack alerts and GitHub releases

---

## GitHub Actions Workflows

### 1. Dashboard CI/CD (`dashboard-ci-cd.yml`)

**Triggers**: Push or PR to `main`/`develop` branches when dashboard files change

**Jobs**:

#### Build (Matrix: Node 18.x, 20.x)
- Installs dependencies
- Runs ESLint
- Type checking with TypeScript
- Production build
- Bundle size check
- Uploads artifacts (kept for 5 days)

#### Test
- Runs Jest unit tests
- Generates coverage report
- CodeCov upload

#### Security
- NPM audit for vulnerabilities
- OWASP dependency check

#### Quality
- ESLint analysis
- Prettier formatting check
- SonarQube analysis (if configured)

#### Deploy to Staging (on `develop` push)
- Downloads build artifacts
- SSH deploy to staging server
- Status notification

#### Deploy to Production (on `main` push)
- Docker image build and push
- Production server deployment
- GitHub release creation

#### Notifications
- Slack webhook on completion
- Status summary

**Run Time**: ~15 minutes

---

### 2. Docker Build (`dashboard-docker.yml`)

**Triggers**: Push to `main`/`develop` when dashboard files change

**Features**:
- Multi-architecture builds (amd64, arm64)
- Container registry push
- Trivy vulnerability scanning
- Docker metadata tagging

**Registry**: `ghcr.io`

---

### 3. Performance Tests (`dashboard-performance.yml`)

**Triggers**: Push, PR, or 2 AM UTC daily

**Jobs**:

#### Lighthouse Audit
- Performance scoring
- Accessibility audit
- Best practices check
- SEO validation
- PR comments with results

#### Bundle Analysis
- JavaScript file size tracking
- Bundle composition analysis
- Regression detection

#### Performance Regression
- Build time measurement
- Bundle size limits (2MB maximum)
- Failure on threshold exceed

**Thresholds**:
- Performance: 60+
- Accessibility: 85+
- Best Practices: 80+
- SEO: 85+

---

## Local Development

### Quick Start

```bash
# Install dependencies
npm install

# Start development server with HMR
npm run dev

# Full development environment with Docker Compose
docker-compose -f docker-compose.dev.yml up
```

### Available Commands

```bash
# Development
make dev                 # Start dev server
make build              # Production build
make preview            # Test production build

# Testing
make test               # Unit tests
make test-watch        # Watch mode
make test-coverage      # Coverage report
make e2e                # E2E tests

# Code Quality
make lint               # ESLint
make type-check         # TypeScript
make format             # Prettier
make format-check       # Format validation

# Docker
make docker             # Build image
make docker-compose     # Run compose
make docker-push        # Push to registry

# Deployment
make deploy-staging     # Deploy staging
make deploy-prod        # Deploy production
make deploy-k8s         # Deploy Kubernetes
```

---

## Deployment Strategies

### 1. Staging Deployment

Automatically triggered on `develop` branch push:

```bash
ssh $STAGING_USER@$STAGING_HOST
cd /var/www/dashboard-staging/
```

**URL**: `https://staging-dashboard.x3-chain.com`

**Configuration**: `.env.staging`

---

### 2. Production Deployment

Automatically triggered on `main` branch push:

Steps:
1. Build production bundle
2. Build Docker image with version tag
3. Push to container registry
4. Create GitHub release
5. Deploy to production servers
6. Update Kubernetes deployments (if enabled)

**URL**: `https://dashboard.x3-chain.com`

---

### 3. Kubernetes Deployment

Deploy using provided manifest:

```bash
# Apply configuration and deployment
kubectl apply -f kubernetes-manifest.yml

# Check rollout status
kubectl rollout status deployment/gpu-swarm-dashboard -n gpu-swarm

# View logs
kubectl logs -f deployment/gpu-swarm-dashboard -n gpu-swarm
```

**Features**:
- 3 replicas minimum
- Auto-scaling (3-10 replicas)
- Rolling updates
- Pod disruption budget
- Network policies
- Resource limits

---

## Environment Variables

### Development

```env
VITE_API_BASE_URL=http://localhost:5000/api
VITE_WS_BASE_URL=ws://localhost:5000/ws
VITE_DEBUG=true
```

### Staging

```env
VITE_API_BASE_URL=https://staging-api.x3-chain.com/api
VITE_WS_BASE_URL=wss://staging-api.x3-chain.com/ws
VITE_NETWORK_NAME=staging
```

### Production

```env
VITE_API_BASE_URL=https://api.x3-chain.com/api
VITE_WS_BASE_URL=wss://api.x3-chain.com/ws
VITE_NETWORK_NAME=mainnet
```

---

## GitHub Secrets Setup

Configure these secrets in GitHub repository settings:

```
DOCKER_USERNAME = Docker registry username
DOCKER_PASSWORD = Docker registry password (use token)
STAGING_HOST = Staging server hostname
STAGING_USER = SSH username
STAGING_KEY = SSH private key
PROD_HOST = Production server hostname
PROD_USER = SSH username
PROD_KEY = SSH private key
SLACK_WEBHOOK_URL = Slack notification webhook
SONAR_HOST_URL = SonarQube instance URL (optional)
SONAR_TOKEN = SonarQube authentication token (optional)
```

---

## Testing

### Unit Tests

```bash
npm test                # Run all tests once
npm run test:watch     # Watch mode
npm run test:coverage  # Generate coverage
```

**Coverage Thresholds**:
- Statements: 50%
- Branches: 50%
- Functions: 50%
- Lines: 50%

### E2E Tests

```bash
npm run e2e            # Run Cypress tests
npm run e2e:open      # Open Cypress UI
```

**Base URL**: `http://localhost:5173`

### Lighthouse Audits

```bash
npm run build           # Build first
npm run preview         # Start preview server
make lighthouse         # Run Lighthouse

# Or through CI:
# Automatically runs on schedule and PRs
```

---

## Monitoring & Logging

### GitHub Actions

- View runs: **Actions** tab in repository
- Download artifacts: **Artifacts** section
- Deployment status: **Deployments** tab

### Slack Notifications

Pipeline completion status sent to configured webhook:
- ✅ Build passed
- ❌ Build failed
- Deploy notifications

### Performance Metrics

- Build time tracking
- Bundle size monitoring
- Lighthouse scores trend

---

## Troubleshooting

### Build Failures

1. Check logs in GitHub Actions
2. Verify Node.js version (20.x recommended)
3. Clear npm cache: `npm cache clean --force`
4. Reinstall dependencies: `npm ci`

### Test Failures

1. Run locally: `npm test`
2. Check for dependencies: `npm audit`
3. Update packages: `npm update`

### Deployment Issues

1. Verify environment variables are set
2. Check SSH keys/credentials
3. Confirm target server accessibility
4. Review logs: `kubectl logs ...`

### Docker Issues

1. Build locally: `docker build .`
2. Check disk space
3. Verify Docker credentials: `docker login`

---

## Security

### Vulnerability Scanning

- NPM audit: Runs on all PRs
- OWASP dependency check: Weekly
- Container scan: Trivy on Docker build

**Action**: Fix critical/high vulnerabilities before merge

### Best Practices

- Keep dependencies updated
- Use GitHub secret management
- Enable branch protection rules
- Require PR reviews before merge
- Use SSH keys (not passwords)

---

## Performance Optimization

### Bundle Size Limit

**Current**: ~600KB (Recharts included)
**Threshold**: 2MB
**Target**: <500KB

### Optimization Techniques

1. Code splitting by route
2. Lazy loading for charts
3. Tree shaking unused code
4. Minification & gzip
5. Service worker caching

### Lighthouse Targets

- Performance: 90+
- Accessibility: 95+
- Best Practices: 95+
- SEO: 95+

---

## Versioning & Releases

### Semantic Versioning

- **MAJOR**: Breaking changes
- **MINOR**: New features
- **PATCH**: Bug fixes

### Release Process

1. Update version in `package.json`
2. Create git tag
3. Push to main
4. GitHub Actions creates release
5. Docker image tagged with version

**Format**: `dashboard-vN` (e.g., `dashboard-v42`)

---

## Maintenance

### Dependency Updates

```bash
npm update              # Update within version ranges
npm outdated            # Check for updates
npm audit fix           # Fix vulnerabilities
```

**Schedule**: Weekly checks, monthly updates

### Archive Old Artifacts

- Keep last 5 builds
- Delete after 1 month
- GitHub Actions cleanup automatic

---

## Integration with Backend

### API Configuration

Dashboard connects to backend via:
- **REST API**: `https://api.x3-chain.com/api`
- **WebSocket**: `wss://api.x3-chain.com/ws`

### Health Checks

```bash
# API health
curl https://api.x3-chain.com/api/health

# Dashboard health
curl https://dashboard.x3-chain.com/health
```

---

## Support & Documentation

- **Build Report**: [BUILD_REPORT.md](BUILD_REPORT.md)
- **Deployment Guide**: [DEPLOYMENT.md](DEPLOYMENT.md)
- **Implementation**: [IMPLEMENTATION_SUMMARY.md](../../../../reports/IMPLEMENTATION_SUMMARY.md)
- **API Docs**: See backend documentation
- **GitHub Issues**: Report bugs and feature requests

---

## Quick Reference

| Task | Command |
|------|---------|
| Development | `make dev` or `npm run dev` |
| Build | `make build` or `npm run build` |
| Test | `make test` or `npm test` |
| Deploy Staging | `make deploy-staging` |
| Deploy Production | `make deploy-prod` |
| Docker Build | `make docker` |
| Full Environment | `make dev-full` |

---

**Last Updated**: February 2026
**Status**: Production Ready ✅
