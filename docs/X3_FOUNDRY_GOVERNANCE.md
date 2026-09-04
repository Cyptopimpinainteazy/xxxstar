# X3 Foundry Governance

## Governance Overview

X3 Foundry uses a transparent, timelock-protected governance system to manage platform parameters, template approvals, fee configurations, and dispute resolution.

## Proposal Lifecycle

```
1. PROPOSE ──> 2. VOTE ──> 3. TIMELOCK ──> 4. EXECUTE
                     │                          │
                     ▼                          ▼
                Can be rejected            Can be cancelled
                (proposal fails)           (emergency only)
```

### 1. Propose
- Any governance member can submit a proposal
- Proposal includes: target contract, function signature, parameters, rationale
- Proposals are public and transparent
- Minimum deposit required to prevent spam

### 2. Vote
- Voting period: 7 days (configurable)
- Votes: For, Against, Abstain
- Quorum required: minimum % of voting power
- Approval threshold: majority of votes cast
- Vote power: proportional to governance token stake

### 3. Timelock
- Minimum delay: 48 hours for parameter changes
- Minimum delay: 7 days for fee increases
- Minimum delay: 24 hours for fee decreases
- Emergency actions: 0 delay (limited to security only)

### 4. Execute
- After timelock expires, anyone can execute
- Failed executions are logged
- Partial execution is not possible (all-or-nothing)

## Governance Parameters

| Parameter | Default | Min | Max | Timelock |
|-----------|---------|-----|-----|----------|
| Platform min fee | 50 bps (0.5%) | 10 bps | 500 bps | 7 days |
| Platform max fee | 500 bps (5%) | 50 bps | 1000 bps | 7 days |
| Treasury split | 40/20/15/10/10/5 | — | — | 7 days |
| Template approval | Governance vote | — | — | 48 hours |
| Featured app fee | 1000 X3 | 0 | — | 48 hours |
| Dispute period | 7 days | 1 day | 30 days | 48 hours |
| Voting period | 7 days | 1 day | 14 days | 48 hours |
| Quorum | 10% | 5% | 50% | 7 days |

## Timelock Controls

### Fee Increases (7-day timelock)
- Platform minimum fee
- Platform maximum fee
- Treasury split percentages
- Template registration fees

### Standard Changes (48-hour timelock)
- Template approval/deprecation
- Featured app selection
- Dispute resolution parameters
- Marketplace moderation rules

### Emergency Actions (no timelock)
- Pause all deployments (security incident)
- Suspend specific dApp (verified exploit)
- Block malicious template
- Freeze revenue router (critical bug)

**Emergency actions require 2/3 multisig approval.**

## Template Approval Process

1. Developer submits template for review
2. Security audit runs automatically
3. Governance reviews audit results
4. Vote on template approval
5. If approved: template is listed in registry
6. If rejected: developer receives audit report with reasons

### Template Requirements
- Complete source code
- Passing security audit
- Clear documentation
- Fee transparency declaration
- License declaration
- No scam patterns

## Dispute Resolution

### Dispute Types
- **Fee dispute**: Creator claims incorrect fee routing
- **Revenue dispute**: Creator claims missing revenue
- **Template dispute**: Template contains undisclosed functionality
- **Marketplace dispute**: Listing contains false information
- **Fork dispute**: Fork violates license terms

### Resolution Process
1. Disputant raises dispute with deposit
2. Respondent has 7 days to respond
3. Governance reviews evidence
4. Vote on resolution
5. If ruled in favor: deposit returned + compensation
6. If ruled against: deposit forfeited to platform

## Marketplace Moderation

### Moderation Actions
- Feature/delist apps
- Verify/unverify creators
- Flag suspicious listings
- Remove scam content
- Ban repeat offenders

### Moderation Principles
- Transparent with public reasoning
- Appealable within 7 days
- Proportional to violation
- First offense: warning
- Second offense: 30-day suspension
- Third offense: permanent ban

## Governance Contract Reference

### FoundryGovernance.sol
```
propose(target, signature, params, description)  → proposalId
vote(proposalId, support)                         → weight
execute(proposalId)                               → success
cancel(proposalId)                                → (emergency only)
setTimelock(delay)                                → (governance only)
pause() / unpause()                               → (emergency multisig)
```

### Key Security Properties
- Fee increases require 7-day timelock
- Emergency pause requires 2/3 multisig
- All proposals are public
- All votes are on-chain
- Timelock cannot be bypassed
- Governance cannot drain user funds
