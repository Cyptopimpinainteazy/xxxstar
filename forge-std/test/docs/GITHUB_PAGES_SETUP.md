# GitHub Pages Dashboard Setup Guide

## Overview

This guide walks through setting up GitHub Pages to publish the ProofForge dashboard from your X3 Atomic Star repository. The dashboard provides real-time visualization of proof verification scores, module status, and blockchain readiness indicators.

## Prerequisites

- Repository hosted on GitHub
- GitHub Pages enabled for the repository
- `.github/workflows/proof-gates.yml` deployed
- `scripts/publish-dashboard.sh` available
- Sufficient workflow permissions to deploy to `gh-pages` branch

## Step 1: Enable GitHub Pages

### Via GitHub UI

1. Go to repository **Settings** → **Pages**
2. Under "Build and deployment":
   - Source: Select **"Deploy from a branch"**
   - Branch: Select **`gh-pages`** (or `main` if using root folder)
   - Folder: Select **`/ (root)`** or **`/dashboard`**
3. Click **Save**

### Verification

After enabling, GitHub shows: "Your site is published at: `https://username.github.io/x3-atomic-star/`"

## Step 2: Configure Workflow for Dashboard Deployment

### Create `.github/workflows/deploy-dashboard.yml`

```yaml
name: Deploy ProofForge Dashboard

on:
  workflow_run:
    workflows: ["ProofForge Gates"]
    types: [completed]
    branches: [main]
  schedule:
    - cron: '0 4 * * *'  # 1 hour after proof-gates runs (3 AM UTC)
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pages: write
      id-token: write
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-gnu
      
      - name: Restore Rust cache
        uses: Swatinem/rust-cache@v2
      
      - name: Build ProofForge binary
        run: |
          cargo build -p proof-forge --release
          ls -lh target/release/x3-proof
      
      - name: Generate dashboard
        run: |
          chmod +x scripts/publish-dashboard.sh
          ./scripts/publish-dashboard.sh ./dashboard
      
      - name: Create index redirect (if using root publish)
        if: always()
        run: |
          cat > index.html << 'EOF'
          <!DOCTYPE html>
          <html>
            <head>
              <meta http-equiv="refresh" content="0; url=dashboard/" />
            </head>
            <body>
              <p>Redirecting to <a href="dashboard/">dashboard</a>...</p>
            </body>
          </html>
          EOF
      
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./dashboard
          cname: ''
          force_orphan: false
          commit_message: "Deploy: ProofForge Dashboard Update"
          user_name: 'github-actions[bot]'
          user_email: 'github-actions[bot]@users.noreply.github.com'
      
      - name: Comment on workflow run
        if: github.event.workflow_run.conclusion == 'success'
        uses: actions/github-script@v7
        with:
          script: |
            const dashboardUrl = context.payload.repository.html_url + '/deployments';
            console.log('Dashboard published to GitHub Pages');
            console.log('URL: ${{ github.server_url }}/${{ github.repository }}/deployments');
```

### Save to Repository

```bash
cp .github/workflows/deploy-dashboard.yml .github/workflows/
git add .github/workflows/deploy-dashboard.yml
git commit -m "Add GitHub Pages dashboard deployment workflow"
git push origin main
```

## Step 3: Configure Publishing Source

### Option A: Publish from `gh-pages` Branch (Recommended)

```yaml
# In .github/workflows/deploy-dashboard.yml (already configured above)
- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./dashboard
```

This automatically creates/updates the `gh-pages` branch with dashboard content.

### Option B: Publish from `/docs` Folder in Main Branch

If you prefer to keep everything in `main`:

1. Create `docs/` directory:
   ```bash
   mkdir -p docs
   cp -r dashboard/* docs/
   ```

2. In GitHub Settings → Pages:
   - Source: **Deploy from a branch**
   - Branch: **main**
   - Folder: **/docs**

3. Update workflow:
   ```yaml
   - name: Copy dashboard to docs/
     run: cp -r dashboard/* docs/
   
   - name: Commit and push
     run: |
       git config user.name "github-actions[bot]"
       git config user.email "github-actions[bot]@users.noreply.github.com"
       git add docs/
       git commit -m "Update dashboard" || echo "No changes"
       git push origin main
   ```

## Step 4: Local Testing

### Build and View Dashboard Locally

```bash
# Build the binary
cargo build -p proof-forge --release

# Generate dashboard
./scripts/publish-dashboard.sh ./dashboard

# View with local server
cd dashboard
python3 -m http.server 8000
# Visit: http://localhost:8000
```

### Test with Python Simple Server

```bash
# From repository root
python3 -m http.server -d dashboard 8000

# Or using Node.js http-server
npm install -g http-server
http-server -p 8000 dashboard/
```

## Step 5: Monitor Deployments

### GitHub Actions Insights

1. Go to **Actions** tab in repository
2. Select **"Deploy ProofForge Dashboard"** workflow
3. Check recent runs:
   - ✅ Green = Dashboard deployed successfully
   - ❌ Red = Deployment failed (check logs)

### View Deployment History

```bash
# List all GitHub Pages deployments
gh api repos/{owner}/{repo}/pages/deployments

# View specific deployment
gh api repos/{owner}/{repo}/deployments/{deployment_id}
```

### Access Published Dashboard

```
https://{github-username}.github.io/{repository-name}/
Example: https://myusername.github.io/x3-atomic-star/
```

## Step 6: Customize Dashboard Content

### Edit HTML Template

Edit `scripts/publish-dashboard.sh` to customize:

```bash
cat > "${OUTPUT_DIR}/index.html" << 'HTMLEOF'
<!-- Your custom HTML here -->
HTMLEOF
```

### Include Custom CSS/JS

Add to dashboard generation:

```bash
# Copy additional assets
cp assets/custom.css "${OUTPUT_DIR}/"
cp assets/custom.js "${OUTPUT_DIR}/"

# Update HTML to reference them
sed -i 's|</head>|<link rel="stylesheet" href="custom.css"></head>|' "${OUTPUT_DIR}/index.html"
```

### Real-Time Data Updates

Add fetch script to `index.html`:

```javascript
<script>
  async function updateDashboard() {
    try {
      const response = await fetch('./proof-score.json');
      const data = await response.json();
      
      document.getElementById('overall-score').textContent = data.overall_score;
      document.getElementById('grade').textContent = data.grade;
      document.getElementById('timestamp').textContent = new Date().toISOString();
    } catch (error) {
      console.error('Failed to fetch dashboard data:', error);
    }
  }
  
  // Update on page load and every 5 minutes
  updateDashboard();
  setInterval(updateDashboard, 5 * 60 * 1000);
</script>
```

## Step 7: Advanced Configuration

### Add Custom Domain (Optional)

```yaml
- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./dashboard
    cname: 'proofforge.example.com'  # Your custom domain
```

Then update DNS records to point to GitHub Pages IP.

### Enable HTTPS

GitHub Pages automatically provides HTTPS certificates for:
- `*.github.io` subdomains
- Custom domains (via CNAME)

No additional configuration needed.

### Cache Dashboard for Performance

In workflow:

```yaml
- name: Cache dashboard files
  uses: actions/cache@v3
  with:
    path: dashboard/
    key: dashboard-${{ github.run_id }}
    restore-keys: |
      dashboard-
```

## Troubleshooting

### Dashboard Not Updating

**Symptom:** Dashboard shows stale data

**Solutions:**
1. Check workflow execution: **Actions** tab → **Deploy ProofForge Dashboard**
2. Verify `proof-gates.yml` completes successfully before dashboard deploy
3. Clear browser cache: `Ctrl+Shift+Delete` or `Cmd+Shift+Delete`
4. Check GitHub Pages settings: Settings → Pages → Source is correct

### 404 Error When Accessing Dashboard

**Symptom:** `https://username.github.io/repo/` shows 404

**Solutions:**
1. Verify GitHub Pages is enabled: Settings → Pages
2. Confirm `gh-pages` branch exists: `git branch -r | grep gh-pages`
3. Check branch has content: `git log gh-pages -- dashboard/`
4. Wait 1-2 minutes after deployment (GitHub Pages indexing)

### Permission Denied: Cannot Deploy

**Symptom:** Workflow fails with permission error

**Solutions:**
1. Check token permissions: Settings → Actions → General → Workflow permissions
2. Set to **"Read and write permissions"**
3. Enable **"Allow GitHub Actions to create and approve pull requests"**

### Workflow Never Triggers

**Symptom:** Dashboard deployment workflow doesn't run

**Solutions:**
1. Verify trigger events are configured in `.yml`
2. Check `proof-gates.yml` completes (dashboard workflow depends on it)
3. Manually trigger: Actions → Deploy ProofForge Dashboard → Run workflow

## Dashboard Files Reference

### Generated Files Structure

```
dashboard/
├── index.html              # Main dashboard (opens in browser)
├── proof-score.json        # Current proof scores (JSON)
├── metadata.json           # Generation metadata
└── module-scores.csv       # Detailed module scores (CSV export)
```

### File Descriptions

| File | Purpose | Format | Update Frequency |
|------|---------|--------|------------------|
| `index.html` | Interactive dashboard UI | HTML/CSS/JS | After each proof run |
| `proof-score.json` | All proof metrics | JSON | After each proof run |
| `metadata.json` | Generation timestamp, version | JSON | After each proof run |
| `module-scores.csv` | Module details for external analysis | CSV | After each proof run |

## Integration with CI/CD Pipeline

### Complete Workflow Sequence

```
1. Developer pushes to main
   ↓
2. proof-gates.yml workflow triggers
   ├─ Build binary
   ├─ Run tests
   ├─ S0 gate
   ├─ S1 gate
   └─ Generate dashboard (saves to artifact)
   ↓
3. deploy-dashboard.yml workflow triggers (on completion)
   ├─ Download latest proof-score.json
   ├─ Build HTML dashboard
   ├─ Deploy to gh-pages branch
   └─ Publish to GitHub Pages
   ↓
4. Dashboard live at: https://username.github.io/repo/
```

### Cross-Workflow Artifact Sharing

To pass dashboard files between workflows:

```yaml
# In proof-gates.yml
- name: Upload dashboard artifacts
  uses: actions/upload-artifact@v4
  with:
    name: dashboard-data
    path: dashboard/
    retention-days: 90

# In deploy-dashboard.yml
- name: Download dashboard artifacts
  uses: actions/download-artifact@v4
  with:
    name: dashboard-data
    path: dashboard/
```

## Maintenance

### Regular Updates

- **Weekly:** Review dashboard metrics and fix any anomalies
- **Monthly:** Update module scores based on new proof results
- **Quarterly:** Audit dashboard HTML/CSS for design improvements
- **Annually:** Review GitHub Pages configuration for deprecated features

### Backup Dashboard Data

```bash
# Backup current dashboard
git clone --single-branch --branch gh-pages https://github.com/user/repo dashboard-backup

# Archive metrics
tar czf dashboard-metrics-$(date +%Y-%m-%d).tar.gz dashboard/proof-score.json dashboard/module-scores.csv
```

## References

- [GitHub Pages Documentation](https://docs.github.com/en/pages)
- [GitHub Pages Actions Deployment](https://github.com/peaceiris/actions-gh-pages)
- [GitHub Actions Workflows](https://docs.github.com/en/actions/using-workflows)
- [ProofForge CLI Reference](./PROOFFORGE_CLI.md)

---

**Last Updated:** 2024  
**ProofForge Version:** 1.0.0  
**Status:** ✅ Production Ready
