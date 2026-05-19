# X3 Chain MCP Configuration

## Quick Usage & Deployment Notes

### File Storage
Store the configuration file at: `infra/mcp-config.json`

**Important**: Edit image names and secret keys first before deployment.

### Orchestrator Options

#### Kubernetes
Write a small operator to convert each service into a Deployment + Service. Use `launch_order` to annotate init ordering or create initContainers.

#### Docker Compose
Convert entries into `docker-compose.yml` format (can be generated programmatically).

#### Custom MCP Orchestrator
Load the JSON configuration and start services respecting `depends_on` and `launch_order`.

### Secrets Management
- Provision Vault (the `secrets-mcp` service) and inject secrets via environment variables or mounted volumes
- **Never keep secrets in the repository**

### Start-up Sequence
Follow the `launch_order` array. Wait for each service's healthcheck to succeed before starting dependent services.

### Safe Defaults
- Ensure `deception-mcp` and `flashloan-mcp` have `SAFE_MODE=true` until explicitly enabled in a sandbox
- Use strict RBAC for NavOps, Orchestration, and Secrets interfaces

## Recommended First-Phase Minimal Boot

If you don't want to deploy all services at once, start with this functional subset:

```
secrets-mcp, fs-mcp, git-mcp, docker-mcp, exec-mcp, db-mcp, x3-node-mcp, rpc-router-mcp, web3-evm-mcp, svm-mcp, wallet-mcp, explorer-mcp, logging-mcp, monitoring-mcp
```

This provides enough functionality to:
- Compile & deploy contracts
- Sign Comits
- Run a development node

## Security & Ethics Guidelines

### Critical Security Notes
⚠️ **Abusive modules** (MEV, Flashloan, Deception) are powerful tools that require careful handling:

- **Lock behind vault & RBAC**: All sensitive operations require multi-signature governance
- **Complete auditing**: Instrument comprehensive logging for all high-risk operations
- **Explicit opt-in**: Require governance approval to enable on non-development networks

### Privacy Considerations
- Agent memory may contain private strategies — treat as sensitive data
- Encrypt at rest and restrict access to authorized personnel only

### Compliance Warning
⚠️ **Legal/Regulatory Notice**: Running deception honeypots against other contracts or actively performing MEV against retail users can have serious legal and regulatory consequences.

**Use these modules only for:**
- Defense research
- Controlled experiments
- Educational purposes

**DO NOT use for:**
- Production exploitation
- Unauthorized access
- Harmful activities against other users
