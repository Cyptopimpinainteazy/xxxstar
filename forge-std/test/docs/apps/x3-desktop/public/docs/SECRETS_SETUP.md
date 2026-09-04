# GitHub Secrets Configuration Guide

## Quick Setup

### 1. Navigate to Repository Settings

1. Go to your GitHub repository
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**

---

## Required Secrets

### Docker Registry Credentials

```
Secret Name: DOCKER_USERNAME
Value: Your Docker Hub username (or ghcr.io if using GitHub Container Registry)
```

```
Secret Name: DOCKER_PASSWORD
Value: Docker access token (NOT your password)
  - Docker Hub: https://hub.docker.com/settings/security
  - GitHub Registry: Personal access token with read:packages, write:packages
```

**⚠️ IMPORTANT**: Use an access token, not your actual password!

---

### Staging Deployment

```
Secret Name: STAGING_HOST
Value: staging.x3-chain.com (or your staging server hostname/IP)
```

```
Secret Name: STAGING_USER
Value: ubuntu (or your SSH username)
```

```
Secret Name: STAGING_KEY
Value: [Paste entire private SSH key here - including -----BEGIN PRIVATE KEY----- header]
```

**Setup Steps**:

1. Generate SSH key on staging server:
   ```bash
   ssh-keygen -t ed25519 -f dashboard-deploy -C "github-actions"
   ```

2. Add public key to `~/.ssh/authorized_keys`:
   ```bash
   cat dashboard-deploy.pub >> ~/.ssh/authorized_keys
   ```

3. Copy private key to GitHub secret:
   ```bash
   cat dashboard-deploy
   # Copy entire output including headers
   ```

---

### Production Deployment

```
Secret Name: PROD_HOST
Value: api.x3-chain.com (or your production server hostname/IP)
```

```
Secret Name: PROD_USER
Value: ubuntu (or your SSH username)
```

```
Secret Name: PROD_KEY
Value: [Paste entire private SSH key here]
```

**Setup**: Same as staging, create separate key for production

---

### Notifications

```
Secret Name: SLACK_WEBHOOK_URL
Value: https://hooks.slack.com/services/YOUR/WEBHOOK/URL
```

**Getting Slack Webhook**:

1. Go to [Slack Apps](https://api.slack.com/apps)
2. Create New App → "From scratch"
3. Name: "GitHub Actions"
4. Select workspace
5. Enable **Incoming Webhooks**
6. Add New Webhook to Workspace
7. Copy Webhook URL to secret

---

### Optional: Code Quality

```
Secret Name: SONAR_HOST_URL
Value: https://sonarqube.x3-chain.com
```

```
Secret Name: SONAR_TOKEN
Value: Your SonarQube authentication token
```

**Getting SonarQube Token**:
1. Log into SonarQube instance
2. User → My Account → Security
3. Generate token
4. Copy to GitHub secret

---

## Verification

### 1. Test Slack Webhook

```bash
curl -X POST -H 'Content-type: application/json' \
  --data '{"text":"Test message"}' \
  YOUR_WEBHOOK_URL
```

### 2. Test SSH Deployment Key

```bash
# From local machine
ssh -i staging-deploy ubuntu@staging.x3-chain.com "echo 'SSH works!'"
```

### 3. Test Docker Credentials

```bash
# Locally
docker login -u $DOCKER_USERNAME -p $DOCKER_PASSWORD
docker pull ghcr.io/yourorg/gpu-swarm-dashboard:latest
```

---

## Workflow Files Reference

### dashboard-ci-cd.yml

Uses these secrets:
- `DOCKER_USERNAME` - Push to registry
- `DOCKER_PASSWORD` - Push to registry
- `STAGING_HOST` - SSH deployment
- `STAGING_USER` - SSH deployment
- `STAGING_KEY` - SSH deployment
- `PROD_HOST` - SSH deployment
- `PROD_USER` - SSH deployment
- `PROD_KEY` - SSH deployment
- `SLACK_WEBHOOK_URL` - Notifications

### dashboard-docker.yml

Uses these secrets:
- `DOCKER_USERNAME` - Build and push
- `DOCKER_PASSWORD` - Build and push

---

## Security Best Practices

### ✅ DO

- Use access tokens, not passwords
- Rotate keys periodically (monthly)
- Use different keys for staging/prod
- Enable branch protection rules
- Require code review before deploy
- Monitor secret access
- Test secrets in non-prod first

### ❌ DON'T

- Commit secrets to repository
- Share secrets in chat/email
- Use the same key for multiple services
- Store secrets in plaintext files
- Skip SSH key rotation
- Disable security checks
- Deploy without review

---

## Troubleshooting

### "Authentication failed"

**Check**:
1. Verify secret names match exactly (case-sensitive)
2. Confirm no trailing whitespace in secret values
3. Test credentials locally first
4. Verify access/permissions for the account

### "SSH permission denied"

**Fix**:
```bash
# On server, check key is in authorized_keys
cat ~/.ssh/authorized_keys | grep github-actions

# Verify permissions
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

### "Docker push failed"

**Check**:
1. Token has correct permissions (write:packages, read:packages)
2. Registry URL matches secret configuration
3. Token hasn't expired
4. Repository is not private without proper access

### "Slack webhook not working"

**Verify**:
1. Webhook URL is complete: `https://hooks.slack.com/services/...`
2. Channel exists and allows apps
3. Test webhook locally with curl
4. Check if bot is still installed in workspace

---

## Environment-Specific Configuration

### Add Environment Secrets

Secrets can be scoped to environments:

1. **Settings** → **Environments** → **Create environment**
   - `staging`
   - `production`

2. Add environment-specific secrets:
   ```
   staging/STAGING_HOST
   staging/STAGING_USER
   staging/STAGING_KEY
   
   production/PROD_HOST
   production/PROD_USER
   production/PROD_KEY
   ```

3. Reference in workflows:
   ```yaml
   - name: Deploy
     environment: production
     uses: ...
   ```

**Benefit**: Prevent accidental production deploys

---

## Rotating Secrets

### Monthly Rotation Schedule

1. **SSH Keys**
   ```bash
   ssh-keygen -t ed25519 -f new-key
   # Update on server, then in GitHub
   ```

2. **Docker Token**
   - Regenerate on Docker Hub/GitHub
   - Update secret value
   - Verify deployment still works

3. **Slack Webhook**
   - Create new webhook
   - Test locally first
   - Update secret
   - Delete old webhook

---

## Testing Secrets Safely

### In Pull Requests

Workflows on PRs don't have access to secrets (security feature).

To test:
1. Use workflow environment variables instead
2. Add print statements (don't commit)
3. Create test branch with temporary secrets

### Local Testing

```bash
# Export secrets locally
export DOCKER_USERNAME="your-username"
export DOCKER_PASSWORD="your-token"

# Run workflow locally (requires Act)
act -s DOCKER_USERNAME -s DOCKER_PASSWORD
```

---

## Support

For issues:
1. Check workflow logs: GitHub Actions → your run
2. Verify secret exists and isn't empty
3. Test credentials locally
4. Check access permissions
5. Review security policies

**Need help?** Create an issue with:
- Error message (sanitize credentials!)
- Which secret is affected
- When it started failing
- Steps to reproduce

---

**Last Updated**: February 2026
**Status**: Ready for Configuration ✅
