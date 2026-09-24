#!/usr/bin/env bash
# X3 Chain TypeScript SDK Security Audit Script
# Performs comprehensive dependency scanning, vulnerability detection, and security analysis

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SDK_DIR="${PROJECT_ROOT}/packages/ts-sdk"
REPORT_DIR="${SCRIPT_DIR}/reports"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== X3 Chain SDK Security Audit ===${NC}"
echo "Timestamp: ${TIMESTAMP}"
echo "SDK Location: ${SDK_DIR}"
echo ""

# Create reports directory
mkdir -p "${REPORT_DIR}"

# Function to log results
log_result() {
  local level=$1
  local message=$2
  case $level in
    ERROR)
      echo -e "${RED}[ERROR]${NC} ${message}"
      echo "[ERROR] ${message}" >> "${REPORT_DIR}/audit-${TIMESTAMP}.log"
      ;;
    WARNING)
      echo -e "${YELLOW}[WARNING]${NC} ${message}"
      echo "[WARNING] ${message}" >> "${REPORT_DIR}/audit-${TIMESTAMP}.log"
      ;;
    SUCCESS)
      echo -e "${GREEN}[SUCCESS]${NC} ${message}"
      echo "[SUCCESS] ${message}" >> "${REPORT_DIR}/audit-${TIMESTAMP}.log"
      ;;
    INFO)
      echo -e "${BLUE}[INFO]${NC} ${message}"
      echo "[INFO] ${message}" >> "${REPORT_DIR}/audit-${TIMESTAMP}.log"
      ;;
  esac
}

# Change to SDK directory
cd "${SDK_DIR}"

# 1. NPM Audit
echo -e "\n${BLUE}=== Running NPM Audit ===${NC}"
log_result INFO "Starting npm audit..."

if npm audit --json > "${REPORT_DIR}/npm-audit-${TIMESTAMP}.json" 2>&1; then
  log_result SUCCESS "npm audit completed - no vulnerabilities found"
else
  AUDIT_EXIT_CODE=$?
  if [ $AUDIT_EXIT_CODE -eq 1 ]; then
    log_result ERROR "npm audit found vulnerabilities - see report"
    npm audit --json | jq '.vulnerabilities' > "${REPORT_DIR}/npm-vulnerabilities-${TIMESTAMP}.json"
  fi
fi

# Generate human-readable npm audit report
npm audit > "${REPORT_DIR}/npm-audit-readable-${TIMESTAMP}.txt" 2>&1 || true

# 2. Dependency Tree Analysis
echo -e "\n${BLUE}=== Analyzing Dependency Tree ===${NC}"
log_result INFO "Generating dependency tree..."

npm ls --all --json > "${REPORT_DIR}/dependency-tree-${TIMESTAMP}.json" 2>&1 || true
npm ls --all > "${REPORT_DIR}/dependency-tree-readable-${TIMESTAMP}.txt" 2>&1 || true

# Count dependencies
DIRECT_DEPS=$(jq '.dependencies | length' package.json)
TOTAL_DEPS=$(npm ls --all --json 2>/dev/null | jq '[.. | .dependencies? | select(. != null) | keys[]] | unique | length' || echo "N/A")

log_result INFO "Direct dependencies: ${DIRECT_DEPS}"
log_result INFO "Total dependencies (including transitive): ${TOTAL_DEPS}"

# 3. Check for outdated packages
echo -e "\n${BLUE}=== Checking for Outdated Packages ===${NC}"
log_result INFO "Checking for package updates..."

npm outdated --json > "${REPORT_DIR}/outdated-packages-${TIMESTAMP}.json" 2>&1 || true
npm outdated > "${REPORT_DIR}/outdated-packages-readable-${TIMESTAMP}.txt" 2>&1 || true

# 4. License Check
echo -e "\n${BLUE}=== Analyzing Package Licenses ===${NC}"
log_result INFO "Checking package licenses..."

if command -v license-checker &> /dev/null; then
  npx license-checker --json > "${REPORT_DIR}/licenses-${TIMESTAMP}.json" 2>&1 || true
  npx license-checker --summary > "${REPORT_DIR}/licenses-summary-${TIMESTAMP}.txt" 2>&1 || true
else
  log_result WARNING "license-checker not available, installing..."
  npm install -g license-checker
  npx license-checker --json > "${REPORT_DIR}/licenses-${TIMESTAMP}.json" 2>&1 || true
fi

# 5. Snyk Scan (if available)
echo -e "\n${BLUE}=== Running Snyk Security Scan ===${NC}"
if command -v snyk &> /dev/null; then
  log_result INFO "Running Snyk scan..."
  snyk test --json > "${REPORT_DIR}/snyk-${TIMESTAMP}.json" 2>&1 || true
  snyk test > "${REPORT_DIR}/snyk-readable-${TIMESTAMP}.txt" 2>&1 || true
else
  log_result WARNING "Snyk CLI not available - install with: npm install -g snyk"
fi

# 6. Check for known security issues in specific packages
echo -e "\n${BLUE}=== Checking Critical Dependencies ===${NC}"
log_result INFO "Analyzing critical dependencies..."

# Check @polkadot/api version
POLKADOT_VERSION=$(jq -r '.dependencies."@polkadot/api"' package.json)
log_result INFO "@polkadot/api version: ${POLKADOT_VERSION}"

# Check ethers version
ETHERS_VERSION=$(jq -r '.dependencies."ethers"' package.json)
log_result INFO "ethers version: ${ETHERS_VERSION}"

# 7. Static Analysis - Check for hardcoded secrets
echo -e "\n${BLUE}=== Scanning for Hardcoded Secrets ===${NC}"
log_result INFO "Searching for potential secrets in source code..."

SECRET_PATTERNS=(
  "password"
  "secret"
  "api[_-]?key"
  "private[_-]?key"
  "access[_-]?token"
  "auth[_-]?token"
  "aws[_-]?access"
  "-----BEGIN PRIVATE KEY-----"
  "-----BEGIN RSA PRIVATE KEY-----"
)

SECRETS_FOUND=0
for pattern in "${SECRET_PATTERNS[@]}"; do
  if grep -Eri "$pattern" src/ 2>/dev/null | grep -v "node_modules" >> "${REPORT_DIR}/potential-secrets-${TIMESTAMP}.txt"; then
    SECRETS_FOUND=1
  fi
done

if [ $SECRETS_FOUND -eq 1 ]; then
  log_result WARNING "Potential secrets found - review ${REPORT_DIR}/potential-secrets-${TIMESTAMP}.txt"
else
  log_result SUCCESS "No obvious secrets detected"
  echo "No secrets found" > "${REPORT_DIR}/potential-secrets-${TIMESTAMP}.txt"
fi

# 8. Check TypeScript configuration security
echo -e "\n${BLUE}=== Analyzing TypeScript Configuration ===${NC}"
log_result INFO "Checking tsconfig.json security settings..."

if [ -f tsconfig.json ]; then
  STRICT_MODE=$(jq -r '.compilerOptions.strict' tsconfig.json)
  NO_IMPLICIT_ANY=$(jq -r '.compilerOptions.noImplicitAny' tsconfig.json)
  STRICT_NULL_CHECKS=$(jq -r '.compilerOptions.strictNullChecks' tsconfig.json)
  
  log_result INFO "strict: ${STRICT_MODE}"
  log_result INFO "noImplicitAny: ${NO_IMPLICIT_ANY}"
  log_result INFO "strictNullChecks: ${STRICT_NULL_CHECKS}"
  
  if [ "$STRICT_MODE" != "true" ]; then
    log_result WARNING "TypeScript strict mode is not enabled"
  else
    log_result SUCCESS "TypeScript strict mode is enabled"
  fi
fi

# 9. Check package.json security configurations
echo -e "\n${BLUE}=== Analyzing Package Configuration ===${NC}"
log_result INFO "Checking package.json security settings..."

# Check for prepublish scripts (can be security risk)
PREPUBLISH=$(jq -r '.scripts.prepublish // "none"' package.json)
if [ "$PREPUBLISH" != "none" ]; then
  log_result WARNING "prepublish script detected: ${PREPUBLISH}"
fi

# Check if package is marked as private
IS_PRIVATE=$(jq -r '.private // false' package.json)
log_result INFO "Package private flag: ${IS_PRIVATE}"

# 10. Generate summary report
echo -e "\n${BLUE}=== Generating Summary Report ===${NC}"

cat > "${REPORT_DIR}/audit-summary-${TIMESTAMP}.md" <<EOF
# X3 Chain TypeScript SDK Security Audit Report

**Date**: $(date)
**SDK Version**: $(jq -r '.version' package.json)
**Auditor**: Automated Security Scan

## Summary

### Dependency Analysis
- Direct dependencies: ${DIRECT_DEPS}
- Total dependencies: ${TOTAL_DEPS}

### NPM Audit Results
$(npm audit --json 2>&1 | jq -r '"\(.metadata.vulnerabilities.critical // 0) Critical, \(.metadata.vulnerabilities.high // 0) High, \(.metadata.vulnerabilities.moderate // 0) Moderate, \(.metadata.vulnerabilities.low // 0) Low"' || echo "See npm-audit report")

### TypeScript Configuration
- Strict mode: ${STRICT_MODE}
- noImplicitAny: ${NO_IMPLICIT_ANY}
- strictNullChecks: ${STRICT_NULL_CHECKS}

### Critical Dependencies
- @polkadot/api: ${POLKADOT_VERSION}
- ethers: ${ETHERS_VERSION}

## Detailed Reports

The following detailed reports have been generated:

1. \`npm-audit-${TIMESTAMP}.json\` - Full npm audit JSON output
2. \`npm-audit-readable-${TIMESTAMP}.txt\` - Human-readable npm audit
3. \`dependency-tree-${TIMESTAMP}.json\` - Complete dependency tree
4. \`outdated-packages-${TIMESTAMP}.json\` - Outdated package analysis
5. \`licenses-${TIMESTAMP}.json\` - License information for all dependencies
6. \`potential-secrets-${TIMESTAMP}.txt\` - Hardcoded secret scan results

## Recommendations

### Immediate Actions
1. Review and fix any HIGH or CRITICAL vulnerabilities found in npm audit
2. Update outdated packages with known security issues
3. Review potential secrets flagged in source code scan

### Short-term Improvements
1. Enable TypeScript strict mode if not already enabled
2. Implement automated security scanning in CI/CD
3. Set up dependency update automation (e.g., Dependabot)
4. Add security linting rules (eslint-plugin-security)

### Long-term Best Practices
1. Regular quarterly security audits
2. Implement Software Bill of Materials (SBOM) generation
3. Set up vulnerability monitoring and alerting
4. Document security policies and procedures

## Next Steps

1. Review all generated reports in \`${REPORT_DIR}/\`
2. Prioritize vulnerabilities by severity
3. Create tickets for each security issue
4. Apply patches and retest
5. Document changes in security changelog

---

*This is an automated report. Manual review and validation is required.*
EOF

log_result SUCCESS "Summary report generated: ${REPORT_DIR}/audit-summary-${TIMESTAMP}.md"

# Print summary
echo -e "\n${GREEN}=== Audit Complete ===${NC}"
echo "Reports saved to: ${REPORT_DIR}/"
echo "Summary: ${REPORT_DIR}/audit-summary-${TIMESTAMP}.md"
echo ""
echo "Review the reports and address any HIGH or CRITICAL issues immediately."

# Return to original directory
cd "${PROJECT_ROOT}"
