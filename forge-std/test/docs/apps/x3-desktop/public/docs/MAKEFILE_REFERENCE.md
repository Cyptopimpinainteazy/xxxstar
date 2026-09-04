# Makefile Command Reference

Quick reference for all Makefile targets. Type `make <target>` to run.

## 📖 Help & Information

```bash
make help            # Show all available targets
make version         # Show versions of Node, npm, Docker
```

---

## 🚀 Development

### Quick Start

```bash
make install         # npm install (first time setup)
make dev             # npm run dev (start dev server with HMR)
make preview         # npm run preview (test production build)
```

### Full Stack Development

```bash
make dev-full        # Start everything (dev server + Docker Compose)
make dev-stop        # Stop Docker Compose services
```

---

## 🏗️ Build

### Production Build

```bash
make build           # npm run build (production bundle)
make analyze         # npm run build + webpack-bundle-analyzer
make bundle-analyze  # Detailed bundle analysis
```

---

## 🧪 Testing

### Unit Tests

```bash
make test            # jest (run all tests once)
make test-watch      # jest --watch (watch mode)
make test-coverage   # jest --coverage (with coverage report)
```

### End-to-End Tests

```bash
make e2e             # cypress run (headless)
make e2e-open        # cypress open (interactive UI)
```

### Performance Tests

```bash
make lighthouse      # Lighthouse audit (after npm run build & preview)
make perf-check      # Check performance metrics
```

---

## 🔍 Code Quality

### Linting & Formatting

```bash
make lint            # eslint src (check for issues)
make lint-fix        # eslint src --fix (auto-fix issues)
make type-check      # tsc --noEmit (TypeScript check)
make format          # prettier --write (auto-format)
make format-check    # prettier --check (check formatting)
```

### Full Quality Check

```bash
make check           # Run: lint + type-check + format-check
make audit           # npm audit (check dependencies)
make update          # npm update (update packages safely)
```

---

## 🐳 Docker

### Build Docker Image

```bash
make docker          # docker build (local image)
make docker-build    # Same as: make docker
docker tag dashboard:latest dashboard:v1.0  # Tag manually after
```

### Push to Registry

```bash
make docker-push     # Push to ghcr.io (requires login)
# Note: docker login -u USERNAME -p TOKEN ghcr.io
```

### Local Docker Compose

```bash
make docker-compose  # docker-compose up -d (production services)
make docker-down     # docker-compose down (stop services)
make docker-logs     # docker-compose logs -f (view logs)
```

### Development with Docker

```bash
make dev-docker      # docker-compose -f docker-compose.dev.yml up
```

---

## 🚀 Deployment

### Staging Deployment

```bash
make deploy-staging  # Deploy to staging server via SSH
# Uses: $STAGING_HOST, $STAGING_USER, $STAGING_KEY environment variables
```

### Production Deployment

```bash
make deploy-prod     # Deploy to production server via SSH
# Uses: $PROD_HOST, $PROD_USER, $PROD_KEY environment variables
```

### Kubernetes Deployment

```bash
make deploy-k8s      # kubectl apply -f kubernetes-manifest.yml
make k8s-namespace   # Create gpu-swarm namespace
make k8s-rollout     # Check rollout status
make k8s-logs        # View pod logs
make k8s-status      # Show all resources
```

---

## 📊 Monitoring & Logs

### Kubernetes Monitoring

```bash
make k8s-logs        # kubectl logs -f deployment/...
make k8s-status      # kubectl get all
make k8s-describe    # kubectl describe deployment
make k8s-events      # kubectl get events
make k8s-metrics     # kubectl top pods (if metrics available)
```

### Deployment Status

```bash
make deploy-status   # Check last deployment status
check-health         # HTTP health check
```

---

## 🔐 Security & Maintenance

### Pre-commit Setup

```bash
make pre-commit      # Install pre-commit hooks
make pre-commit-run  # Run hooks on all files
```

### Dependency Management

```bash
make audit           # npm audit (security check)
make audit-fix       # npm audit fix (auto-fix vulnerabilities)
make update          # npm update (update packages)
make outdated        # npm outdated (show outdated packages)
```

---

## 🧹 Cleanup

### Remove Build Artifacts

```bash
make clean           # rm -rf dist build node_modules
make clean-all       # clean + remove .env files + Docker cleanup
make clean-logs      # Clear old logs
```

### Docker Cleanup

```bash
make docker-clean    # Remove unused Docker resources
make docker-prune    # Heavy cleanup of Docker system
```

---

## 📈 Analysis & Reporting

### Code Analysis

```bash
make analyze         # Build + webpack-bundle-analyzer
make sonar           # Run SonarQube analysis
```

### Coverage & Metrics

```bash
make test-coverage   # Run tests with coverage report
make metrics         # Show project metrics
```

---

## 🔧 Utility Targets

### Version Info

```bash
make version         # Show Node, npm, Docker versions
make info            # Show all system information
```

### Install

```bash
make install         # npm install (initial setup)
make install-global  # Install global tools (optional)
```

---

## 📋 Complex Workflows

### Complete Development Setup

```bash
make install         # Install dependencies
make dev             # Start development server
# Then in another terminal:
make test-watch      # Watch tests while developing
```

### Full Quality Assurance

```bash
make lint            # Check linting
make type-check      # TypeScript validation
make test            # Run tests
make test-coverage   # Check coverage
make bundle-analyze  # Analyze bundle
# Fix any issues, then:
make format          # Auto-format code
```

### Build & Deploy to Staging

```bash
make build           # Build for production
make docker          # Build Docker image
make docker-push     # Push to registry
make deploy-staging  # Deploy to staging environment
make k8s-logs        # Monitor logs
```

### Full Deployment Pipeline

```bash
# 1. Test everything
make test
make lint
make type-check

# 2. Build
make build
make bundle-analyze

# 3. Container
make docker
make docker-push

# 4. Deploy
make deploy-staging  # Test first
make deploy-prod     # Production
```

---

## 🎯 Common Tasks

### "I want to start developing"

```bash
make install
make dev
# Open http://localhost:5173 in browser
```

### "I want to verify everything works"

```bash
make check           # Lint + type-check + format
make test            # Unit tests
make build           # Production build
make e2e             # End-to-end tests (requires dashboard running)
make lighthouse      # Performance audit
```

### "I want to deploy to staging"

First time:
```bash
make docker          # Build image locally first
make deploy-staging  # Deploy
```

Subsequent:
```bash
make deploy-staging  # Just deploy
```

### "I want to deploy to production"

```bash
make build           # Ensure build works
make test            # All tests pass
make docker          # Build image
make docker-push     # Push to registry
make deploy-prod     # Deploy to production
make k8s-logs        # Monitor deployment
```

### "Something is broken"

```bash
make clean           # Clear everything
make install         # Fresh install
make lint            # Check for issues
make type-check      # TypeScript errors
make test            # Test failures
# Fix issues
make build           # Verify build works
```

### "Performance is slow"

```bash
make bundle-analyze  # Analyze bundle size
make lighthouse      # Performance audit
make test-coverage   # Code coverage
make metrics         # Overall metrics
```

### "I want to clean up"

```bash
make clean           # Remove dist, node_modules
# Or for aggressive cleanup:
make clean-all       # Remove everything
make install         # Fresh start
```

---

## 🌍 Environment Variables for Deployment

Set these before running deployment commands:

```bash
export STAGING_HOST="staging.example.com"
export STAGING_USER="ubuntu"
export STAGING_KEY="$(cat ~/.ssh/staging-key)"

export PROD_HOST="prod.example.com"
export PROD_USER="ubuntu"
export PROD_KEY="$(cat ~/.ssh/prod-key)"

export DOCKER_USERNAME="your-username"
export DOCKER_PASSWORD="your-token"
```

Or use GitHub Secrets (automatically set in CI/CD).

---

## 📞 Troubleshooting

### "make: command not found"

Install Make:
- **macOS**: `brew install make` (usually pre-installed)
- **Linux**: `sudo apt-get install make`
- **Windows**: Use WSL or install Make via chocolatey: `choco install make`

### "make: *** [no such target]. Stop."

Target doesn't exist. Try:
```bash
make help   # Show all available targets
```

### "Command not found" when running target

Make sure dependencies are installed:
```bash
make install
```

### Makefile is outdated

Update from repository:
```bash
git pull origin main
```

---

## 🎓 Learning Makefile

The Makefile uses simple shell commands. Key patterns:

```makefile
target:          # Target name
	command      # Shell command (must be indented with TAB)
	another-cmd  # Multiple commands allowed

# Variables
VAR := value
echo $(VAR)

# Simple targets
make target      # Run target

# Phony targets (don't create files)
.PHONY: build test     # Declare these as phony

# Conditional
ifeq ($(shell command -v docker),)
  $(error Docker not installed)
endif
```

---

## ⚡ Performance Tips

### Faster Builds

```bash
make build          # First build
npm run build       # Direct npm (faster for subsequent builds)
```

### Faster Tests

```bash
make test-watch     # Only rerun changed tests
npm test -- --bail  # Stop on first failure
```

### Parallelize

```bash
# Run multiple makes in background
make lint &
make type-check &
make test &
wait               # Wait for all to finish
```

---

## 📚 Additional Resources

- **Makefile Docs**: https://www.gnu.org/software/make/manual/
- **Make Cheatsheet**: https://cheatography.com/alacner/cheat-sheets/make/

---

## 🎯 Summary

| Goal | Command |
|------|---------|
| **Start developing** | `make dev` |
| **Build for production** | `make build` |
| **Run all tests** | `make check && make test` |
| **Deploy to staging** | `make deploy-staging` |
| **Deploy to production** | `make deploy-prod` |
| **See all options** | `make help` |
| **Clean everything** | `make clean-all` |

---

**Last Updated**: February 2026
**Status**: ✅ Production Ready
