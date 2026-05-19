# Inferstructor Dashboard - Admin Workflows

## Overview

The Inferstructor Dashboard provides administrators with a centralized control surface for managing validators, RPC endpoints, emergency controls, and real-time metrics across the Inferstructor network.

## Key Admin Features

## Deployment Configuration

The dashboard reads GPU lane health endpoints from environment configuration:
- `VITE_GPU_LANE_BASE` provides the default base URL for lane checks.
- `VITE_GPU_LANE_1_URL`, `VITE_GPU_LANE_2_URL`, and `VITE_GPU_LANE_3_URL` can override each lane explicitly.

For deployment references, use:
- `.env.example` for local/dev defaults.
- `.env.production` for production endpoint values.

### 1. Validator Controls

**Purpose**: Manage validator approval, suspension, and unlock states.

**Access**: Admin Mode → Validator Controls

**Workflow**:
1. Admin enters admin mode with credentials
2. Views list of validators with current status:
   - **Approved**: Active validator, fully operational
   - **Pending**: New validator awaiting approval
   - **Suspended**: Inactive validator, halted operations
3. Actions available:
   - **Approve**: Transition from Pending → Approved (enables operations)
   - **Suspend**: Transition to Suspended (halts operations)
   - **Unlock**: Transition from Suspended → Approved (resumes operations)
4. Search by name or validator ID for quick lookup
5. All changes logged to audit trail with timestamp and actor

**Best Practices**:
- Review validator performance metrics before approval
- Communicate suspension reasons to affected parties
- Check audit logs regularly for approval patterns
- Use search for mass operations (filter by chain then bulk action)

### 2. RPC Endpoint Management

**Purpose**: Configure and monitor blockchain RPC endpoints used by the system.

**Access**: Admin Mode → Admin Controls → RPC Endpoints tab

**Workflow**:
1. View current RPC endpoints (chain, URL, health status)
2. Monitor endpoint status (green = healthy, red = unhealthy)
3. Actions:
   - **Add Endpoint**: Create new RPC endpoint with chain and URL
   - **Edit Endpoint**: Update existing endpoint configuration
   - **Health Check**: System automatically monitors endpoint health

**Configuration**:
- Each endpoint requires:
  - Chain selection (Ethereum, Solana, etc.)
  - RPC endpoint URL
  - Optional: Rate limit, authentication headers

**Best Practices**:
- Maintain 2+ endpoints per chain for redundancy
- Monitor endpoint health scores continuously
- Replace unhealthy endpoints immediately
- Update endpoints when providers change URLs

### 3. Faucet Configuration

**Purpose**: Control testnet faucet distribution parameters.

**Access**: Admin Mode → Admin Controls → Faucet Config tab

**Parameters**:
- **Rate Limit**: Maximum tokens per hour (tokens/hour)
- **Max Per Address**: Maximum tokens any single address can receive
- **Cooldown Period**: Waiting time between consecutive requests per address

**Workflow**:
1. Navigate to Faucet Config
2. Adjust parameters based on current demand:
   - Increase rate limit during high testing activity
   - Decrease during low activity to conserve funds
3. Set max per address to prevent single account abuse
4. Configure cooldown to spread distribution over time
5. Click "Save Settings" to apply changes
6. Changes apply immediately to new requests

**Recommended Settings**:
- Rate Limit: 10,000 tokens/hour (production), 100,000 tokens/hour (testing)
- Max Per Address: 1,000 tokens
- Cooldown Period: 24 hours

### 4. Emergency Controls

**Purpose**: Immediately halt all network operations in crisis situations.

**Access**: Admin Mode → Admin Controls → Emergency tab

**Emergency Pause Feature**:
- **Status**: Toggle button (ACTIVE/INACTIVE)
- **Alert**: Red visual indicator when active
- **Scope**: Affects all validators and swap operations
- **Notification**: All users automatically notified

**When to Use**:
- Security breach or attack detected
- Critical bug affecting consensus
- Network fork or chain split
- Scheduled maintenance window
- Transaction finality issues

**Activation Procedure**:
1. Navigate to Emergency Controls
2. Review the warning message
3. Click pause toggle to ACTIVE
4. Yellow warning bar confirms activation
5. Users see system-wide notification
6. All operations automatically halted

**Deactivation Procedure**:
1. Click pause toggle to INACTIVE
2. System resumes normal operations
3. Validators resume duties
4. Users notified of resumption

**Best Practices**:
- Use sparingly - only for genuine emergencies
- Communicate reason to team immediately
- Plan maintenance windows in advance
- Test emergency procedures monthly
- Keep audit log for post-incident analysis

### 5. RBAC (Role-Based Access Control)

**Purpose**: Define permissions for different user roles.

**Access**: Admin Mode → Admin Controls → RBAC tab

**Built-in Roles**:

#### Administrator
- **Permissions**:
  - validator_approval: Approve/suspend validators
  - emergency_pause: Trigger emergency halt
  - audit_view: View audit logs
  - settings_modify: Change system settings
- **Use Case**: Senior ops team members, incident responders

#### Operator
- **Permissions**:
  - validator_view: See validator list and status
  - metrics_view: View performance metrics
  - audit_view: Read audit logs
- **Use Case**: Regular ops team, monitoring specialists

#### Viewer
- **Permissions**:
  - metrics_view: See dashboards and leaderboards
  - leaderboard_view: View validator rankings
- **Use Case**: Read-only access, executives, external stakeholders

**Role Assignment Workflow**:
1. Identify user role requirements
2. Assign appropriate role from RBAC matrix
3. Permissions apply immediately
4. Audit log captures role changes
5. Users must log out/in to see new permissions

**Permission Model**:
- Permissions are additive (higher roles include lower permissions)
- No granular per-validator permissions (role-based only)
- No time-limited permissions (always active once assigned)

### 6. Audit Logs

**Purpose**: Track all administrative actions for compliance and debugging.

**Access**: Admin Mode → Admin Controls → Audit Logs tab

**Log Contents**:
- **Action**: What was done (e.g., "Validator approved", "Emergency pause triggered")
- **Actor**: Who performed the action (email/username)
- **Timestamp**: When the action occurred (ISO 8601 format)
- **Status**: Success, Failed, or Pending

**Retention Policy**:
- Logs retained for 90 days by default
- Export capability for long-term storage
- Immutable (cannot be deleted or modified)

**Common Audit Trail Entries**:
- Validator approval/suspension/unlock
- RPC endpoint additions/updates
- Faucet configuration changes
- Emergency pause activation/deactivation
- RBAC role assignments
- Settings modifications
- CSV export requests

**Compliance Use Cases**:
- Investigate security incidents
- Verify approval workflows
- Track configuration changes
- Demonstrate operational controls
- Generate compliance reports

### 7. Leaderboard & Metrics

**Purpose**: Monitor real-time validator performance and export metrics.

**Access**: Admin Mode → Metrics page

**Dashboard Views**:

#### Summary Metrics
- **Avg TPS**: Average transactions per second across all validators
- **Avg Latency**: Mean block propagation time in milliseconds
- **Avg Uptime**: System availability percentage
- **Gas Efficiency**: Average gas optimization score

#### Hourly Snapshots
- Timestamped records of metrics every hour
- Used for trend analysis and performance tracking
- Persists in localStorage for offline access

#### Validator Rankings
- Sort by: TPS, Latency, Uptime, Gas Efficiency
- Filter by chain: Ethereum, Solana, or All
- Shows individual validator performance

**Admin Features**:
- **Add Snapshot** (Admin Mode): Manually insert snapshot for testing/backfill
- **Export CSV**: Download all snapshots to CSV format
- **Admin Mode Toggle**: Enable/disable manual snapshot injection
- **Persistence**: Data automatically saved to browser localStorage

**Workflow: Export Metrics for Reporting**:
1. Navigate to Leaderboard & Metrics
2. Click "Admin Mode" to enable
3. Click "Export CSV" to download file
4. File named: `metrics-export-YYYY-MM-DD.csv`
5. Contains all snapshots with full metrics history
6. Use in Excel/Google Sheets for analysis

**Workflow: Manual Snapshot for Testing**:
1. Enter Admin Mode
2. Click "+ Add Snapshot"
3. New row automatically generated with:
   - Current timestamp
   - Randomized metrics (within realistic range)
4. Refresh page - snapshot persists
5. New snapshots appear in CSV export

## Security Considerations

### Admin Authentication
- Each admin must provide valid credentials
- Separate admin login path from operator login
- Session timeout recommended after 30 minutes
- IP whitelisting recommended for production

### Audit Trail Protection
- All sensitive actions logged with actor ID
- Impossible to modify/delete audit entries
- Regular audits of admin access logs
- Alert on suspicious patterns (e.g., late night approvals)

### Privilege Escalation Prevention
- No user can assign themselves higher roles
- Role changes require secondary admin approval
- Cross-auditor pattern for critical actions
- Emergency pause requires 2-admin confirmation

### Rate Limiting
- Faucet parameters prevent per-address abuse
- API rate limits on all endpoints
- Cooldown periods enforce temporal spacing
- Burst protection against DOS attacks

## Common Tasks

### Add New Validator
1. Validator submits registration
2. Admin reviews performance metrics
3. Navigate to Validator Controls
4. Search for validator by name
5. Click "Approve" action
6. Confirm in audit logs

### Respond to RPC Endpoint Outage
1. Receive alert: RPC endpoint unhealthy
2. Navigate to Admin Controls → RPC Endpoints
3. Identify unhealthy endpoint (red status)
4. Click "Edit" to update URL
5. Monitor status for recovery
6. Add backup endpoint if needed
7. Document in audit notes

### Emergency System Halt
1. Detect critical issue
2. Navigate to Emergency Controls
3. Verify warning message
4. Click "ACTIVE" toggle
5. Confirm system halted
6. Notify users/team
7. Investigate root cause
8. Toggle "INACTIVE" to resume
9. Post-incident review

### Prepare Weekly Performance Report
1. Navigate to Leaderboard & Metrics
2. Enable Admin Mode
3. Click "Export CSV"
4. Save file with date: `metrics-2024-04-06.csv`
5. Open in spreadsheet application
6. Generate charts and trends
7. Identify top/bottom performers
8. Prepare executive summary

### Grant New Team Member Access
1. New member completes onboarding
2. Admin determines appropriate role
3. Navigate to Admin Controls → RBAC
4. Locate role matching responsibilities
5. Assign new member email to role
6. Send login credentials
7. Member logs in to see restricted features
8. Admin verifies in audit logs

## Troubleshooting

### Issue: Can't find validator to approve
**Solution**:
- Use search by ID instead of name (more reliable)
- Check validator status - may already be approved
- Verify validator is in correct chain filter
- Check audit logs for recent approvals

### Issue: RPC endpoint shows unhealthy status
**Solution**:
- Check endpoint URL is correct
- Verify endpoint service is running
- Check for network connectivity issues
- Try adding backup endpoint
- Contact RPC provider for support

### Issue: Faucet rate limit too high/low
**Solution**:
- Monitor faucet requests per hour
- Adjust rate limit based on demand
- Check cooldown period isn't too long
- Review max per address to prevent abuse

### Issue: Snapshots disappeared after browser refresh
**Solution**:
- Check localStorage isn't disabled
- Verify browser storage quota not exceeded
- Try exporting CSV before data loss
- Use CSV as backup restore source

### Issue: Emergency pause not responding
**Solution**:
- Refresh browser page
- Clear browser cache
- Check browser console for JavaScript errors
- Try different browser
- Contact system administrator

## Key Metrics to Monitor

1. **Validator Health**: Uptime % > 99.8%
2. **Network TPS**: Track trend vs. historical average
3. **Block Latency**: Should stay < 100ms
4. **Gas Efficiency**: Aim for > 85%
5. **RPC Endpoint Availability**: All endpoints > 99.9% uptime
6. **Faucet Distribution**: Monitor depletion rate vs. refill rate
7. **Audit Log Activity**: Unusual patterns indicate security issues

## Further Reading

- See `TESTING.md` for testing procedures
- See project `proposal.md` for feature overview
- See `design.md` for architecture details
- Check GitHub issues for known limitations
