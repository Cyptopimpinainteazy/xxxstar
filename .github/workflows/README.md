# X3 Security Workflows

This repository uses multiple GitHub Actions workflows to maintain enterprise-grade security posture for the X3 blockchain mainnet.

## 🔒 Security Workflows Overview

### 1. CodeQL Analysis (`codeql-analysis.yml`)
- **Purpose**: Built-in security scanning for vulnerabilities, secrets, and security issues
- **Languages**: JavaScript, TypeScript, Rust, Python
- **Triggers**: Push/PR + Weekly schedule
- **Output**: Security alerts in GitHub Security tab

### 2. OSV Scanner (`osv-scan.yml`)
- **Purpose**: Dependency vulnerability scanning using OSV.dev database
- **Scope**: All dependencies across all package managers
- **Triggers**: Push/PR + Weekly schedule
- **Output**: SARIF format security findings

### 3. Rust Clippy (`rust-clippy.yml`)
- **Purpose**: Code quality and security linting for Rust code
- **Scope**: All Rust crates in workspace
- **Triggers**: Push/PR
- **Output**: Clippy warnings/errors

### 4. Semgrep (`semgrep.yml`)
- **Purpose**: Security-focused static analysis with community rules
- **Scope**: All source code
- **Triggers**: Push/PR + Weekly schedule
- **Output**: SARIF security findings

### 5. Trivy (`trivy.yml`)
- **Purpose**: Container image vulnerability scanning
- **Scope**: Docker images built from repository
- **Triggers**: Push/PR + Weekly schedule
- **Output**: Container vulnerability reports

### 6. Snyk Security (`snyk.yml`)
- **Purpose**: Comprehensive dependency and infrastructure scanning
- **Scope**: NPM, Python, and infrastructure dependencies
- **Triggers**: Push/PR + Weekly schedule
- **Output**: Security vulnerabilities and fixes

### 7. Security Dashboard (`security-dashboard.yml`)
- **Purpose**: Weekly comprehensive security assessment
- **Scope**: Rust (cargo-audit), NPM (audit), Python (safety)
- **Triggers**: Manual + Weekly schedule
- **Output**: Consolidated security report artifact

## 🚀 Setup Instructions

### Required Secrets
```bash
# For Snyk workflow
SNYK_TOKEN=your_snyk_token_here
```

### Workflow Dependencies
- All workflows use SARIF format for GitHub Security tab integration
- Container-based workflows (Semgrep) use official images
- Rust workflows require `protoc` for protobuf compilation

## 📊 Security Coverage

| Component | CodeQL | OSV | Clippy | Semgrep | Trivy | Snyk | Dashboard |
|-----------|--------|-----|--------|---------|-------|------|-----------|
| Rust Code | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| JavaScript/TypeScript | ✅ | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ |
| Python | ✅ | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ |
| Dependencies | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Containers | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ |
| Infrastructure | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |

## 🔧 Maintenance

### Weekly Schedule
- Monday 6 AM UTC: CodeQL Analysis
- Monday 7 AM UTC: OSV Scanner
- Monday 8 AM UTC: Semgrep
- Monday 9 AM UTC: Trivy
- Monday 10 AM UTC: Snyk
- Monday 11 AM UTC: Security Dashboard

### Alert Management
- All findings appear in GitHub Security tab
- Critical/high severity alerts trigger notifications
- Weekly dashboard provides comprehensive security status

### Updating Rules
- CodeQL: Auto-updates with GitHub
- Semgrep: Uses community rules, update via workflow
- Clippy: Updates with Rust toolchain
- Others: Update via workflow configuration

## 🛡️ Security Posture

These workflows provide:
- **Multi-layered scanning**: Different tools catch different vulnerabilities
- **Continuous monitoring**: Automated scanning on every change
- **Comprehensive coverage**: Code, dependencies, containers, infrastructure
- **SARIF integration**: All results in GitHub Security tab
- **Scheduled assessments**: Weekly deep dives

**Result**: Enterprise-grade security monitoring for mainnet-ready blockchain.