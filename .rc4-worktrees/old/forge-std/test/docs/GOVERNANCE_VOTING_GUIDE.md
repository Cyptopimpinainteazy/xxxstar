# X3 Governance Voting Guide

## Overview

X3 governance empowers token holders to participate in network decisions through liquid democracy. Members can vote directly, delegate voting power, or use treasury funds for protocol improvements.

## Quick Start

### Creating a Proposal

```javascript
// RPC call to create governance proposal
const proposal = await x3RPC.governance.createProposal({
    title: "Increase block size to 2MB",
    description: "Proposal to increase blockchain throughput by doubling block size",
    metadata: "ipfs://QmHash123",
    minDeposit: "100000000000000000000", // 100 X3 in wei
    fundingAsked: "50000000000000000000"  // 50 X3 for development
});
```

### Voting

```javascript
// Cast your vote
await x3RPC.governance.vote({
    proposalId: proposal.id,
    choice: "yes", // or "no" / "abstain"
    weight: "1000000000000000000" // your voting power (1 token = 1 vote)
});
```

###  Delegation

```javascript
// Delegate your voting power
await x3RPC.governance.delegate({
    delegateTo: "0x742d35Cc6634C0532925a3b844Bc822e02d47E2B",
    expiryBlock: currentBlock + 2102400 // ~12 months
});

// Withdraw delegation (reclaim your voting power)
await x3RPC.governance.withdrawDelegation();
```

## Governance Workflow

### Phase 1: Proposal Creation

Every governance proposal goes through these stages:

**Requirements:**
- Minimum deposit: 100 X3 tokens
- Deposit becomes live after 7 days (awaiting opposition)

```javascript
const proposal = await x3RPC.governance.createProposal({
    title: "My Proposal Title",
    description: "Detailed description of what this proposal does",
    metadata: "ipfs://QmHash", // optional IPFS document
    votingThreshold: 66.7, // % of votes required for passage
    votingPeriod: 7 // days
});
```

**Initial State:**
```
Status: CREATED
Voting: Not started
Available Actions: Deposit more support, Fund proposal
```

### Phase 2: Voting Period

Voting begins after deposit threshold is met:

```javascript
// Vote yes
await x3RPC.governance.vote({
    proposalId: proposal.id,
    choice: "yes"
});

// Vote no
await x3RPC.governance.vote({
    proposalId: proposal.id,
    choice: "no"
});

// Abstain (counted for participation, not for/against)
await x3RPC.governance.vote({
    proposalId: proposal.id,
    choice: "abstain"
});

// Check voting status
const status = await x3RPC.governance.getProposalStatus(proposal.id);
console.log(`Voting: ${status.yesVotes} yes, ${status.noVotes} no, ${status.abstainVotes} abstain`);
```

**Voting Power Calculation:**
```
Base Power = tokens staked at proposal block
Delegated Power = power delegated to you (additive)
Total Power = Base + Delegated
```

### Phase 3: Resolution

**Approval Conditions:**
- Voting participation ≥ 33.3%
- Yes votes ≥ 66.7% of all votes cast (excluding abstentions)

**Outcomes:**

```
APPROVED  → Moves to execution queue
REJECTED  → Proposal fails, deposit is returned
EXPIRED   → Voting period ended without approval
EXECUTED  → Changes become active
```

## Vote Delegation (Liquid Democracy)

### Delegating Voting Power

Delegate your votes to trusted community members without losing your tokens:

```javascript
// Delegate to a trusted validator/lawyer
const delegateTx = await x3RPC.governance.delegate({
    delegateTo: "0x742d35Cc6634C0532925a3b844Bc822e02d47E2B",
    expiryBlock: currentBlock + 1050000, // ~6 months
    reason: "Delegates to expert governance lawyer"
});

// Verify delegation
const delegation = await x3RPC.governance.getDelegation(myAddress);
console.log('Delegated to:', delegation.delegateTo);
console.log('Expires at block:', delegation.expiryBlock);
```

### Transitive Delegation

Delegates can further delegate (up to 3 hops):

```
You → Lawyer → Validator → Governance Expert
            (Transitive delegation chain)
```

When the Governance Expert votes, your power flows through the chain.

### Withdrawal

```javascript
// Withdraw delegation immediately
await x3RPC.governance.withdrawDelegation();

// Now you regain your voting power
// Delegates won't use your power anymore
```

### Tracking Delegated Power

```javascript
// See delegations to you
const delegatedPower = await x3RPC.governance.delegationsToMe();
console.log('Voters delegating to me:', delegatedPower.length);
console.log('Total power delegated:', delegatedPower.total);

// See your delegation
const myDelegation = await x3RPC.governance.myDelegation();
if (myDelegation) {
    console.log(`Your power delegated to ${myDelegation.delegateTo}`);
}
```

## Treasury Management

### Treasury Deposit

Protocols can deposit funds to the treasury for governance-initiated funds:

```javascript
// Deposit 10,000 X3 to treasury
await x3RPC.governance.treasuryDeposit({
    amount: "10000000000000000000000",
    reason: "Quarterly developer fund"
});

// Check treasury balance
const balance = await x3RPC.governance.treasuryBalance();
console.log('Available:', balance.available);
console.log('Reserved:', balance.reserved);
```

### Treasury Spending

Propose and authorize treasury spending:

```javascript
// Create spend proposal
const spendProposal = await x3RPC.governance.proposeTreasurySpend({
    recipient: "0x742d35Cc6634C0532925a3b844Bc822e02d47E2B",
    amount: "5000000000000000000000", // 5000 X3
    description: "Q1 grants for developers",
    votingThreshold: 50 // % approval required
});

// M-of-N approval (council + stakeholders)
// Requires 3-of-5 council members to approve

// Check pending spends
const pending = await x3RPC.governance.getPendingSpends();
pending.forEach(spend => {
    console.log(`${spend.recipient}: ${spend.amount} (${spend.approvals}/5 approved)`);
});
```

### Emergency Treasury

Separate emergency fund with higher approval threshold (75%):

```javascript
// Access emergency reserves
const emergency = await x3RPC.governance.emergencyFund();
console.log('Emergency reserve:', emergency.balance);
console.log('Threshold:', emergency.approvalThreshold);

// Time-locked withdrawals (24-48 hours notice)
await x3RPC.governance.requestEmergencyWithdrawal({
    amount: "1000000000000000000000",
    reason: "Critical security incident response"
});
```

## Voting Best Practices

### Research

Before voting, understand:
1. **Technical impact** — Will changes work as intended?
2. **Economic impact** — How does this affect token value/emissions?
3. **Social impact** — Will this centralize or decentralize the network?

### Example: Evaluating a Proposal

```
PROPOSAL: "Increase validator APY to 15%"

POSITIVE FACTORS:
✓ Attracts more validators
✓ Increases network security (more participants)
✓ Better returns for stakers

NEGATIVE FACTORS:
✗ Increases token inflation
✗ Reduces incentives for application development
✗ May need long-term sustainability plan

VOTING DECISION: Need answers on inflation before voting YES
```

### Delegation Best Practices

1. **Choose trusted experts** — Delegate to established validators/lawyers
2. **Monitor delegations** — Check their voting record periodically
3. **Use expiries** — Set delegation to expire after 6-12 months
4. **Diversify** — Don't delegate all power to one entity
5. **Review changes** — Pull delegation if delegatee voting patterns change

## Proposal Types

### Standard Proposals

Changes to governance parameters:

```javascript
// Example: Change minimum deposit requirement
{
    title: "Reduce minimum proposal deposit to 50 X3",
    votingThreshold: 66.7,
    changes: {
        governanceParameter: "minProposalDeposit",
        oldValue: "100000000000000000000",
        newValue: "50000000000000000000"
    }
}
```

### Funding Proposals

Allocate treasury funds:

```javascript
{
    title: "Developer Grant Program Q1 2024",
    description: "Award 500 X3 total to promising projects",
    fundingAsked: "500000000000000000000",
    recipients: [
        {
            address: "0x...",
            amount: "150000000000000000000",
            purpose: "ZK-SNARK implementation"
        },
        {
            address: "0x...",
            amount: "350000000000000000000",
            purpose: "Cross-chain bridge development"
        }
    ]
}
```

### Council Elections

Elect governance council members:

```javascript
{
    title: "Elect governance council for 2024",
    candidates: [
        { address: "0x...", name: "Alice" },
        { address: "0x...", name: "Bob" },
        // ... up to 20 candidates
    ],
    seatsAvailable: 5,
    votingType: "approval" // vote for candidates you support
}
```

## Monitoring Governance

### Track Active Proposals

```javascript
// Get all proposals
const proposals = await x3RPC.governance.getAllProposals();

// Filter by status
const voting = proposals.filter(p => p.status === 'VOTING');
const pending = proposals.filter(p => p.status === 'PENDING');

// Track specific proposal
const proposal = await x3RPC.governance.getProposal(id);
console.log(`Status: ${proposal.status}`);
console.log(`Voting: ${proposal.yesPercent.toFixed(1)}% yes`);
console.log(`Participation: ${proposal.participation.toFixed(1)}%`);
console.log(`Voting ends: ${new Date(proposal.votingEndTime)}`);
```

### Governance Analytics

```javascript
// Voter participation over time
const history = await x3RPC.governance.getHistoricalParticipation();
console.log(`Average participation: ${history.average.toFixed(1)}%`);

// Most active voters
const topVoters = await x3RPC.governance.getTopVoters(limit: 10);
topVoters.forEach(voter => {
    console.log(`${voter.address}: ${voter.votesParticipated} votes`);
});

// Delegation network
const delegationGraph = await x3RPC.governance.getDelegationGraph();
console.log(`Total delegators: ${delegationGraph.delegatorCount}`);
console.log(`Avg delegation depth: ${delegationGraph.avgDepth}`);
```

## Common Issues

### "Voting power changed since proposal creation"

You voted at voting power 100, but yours has since increased to 150. You can:
1. Vote again to update your power
2. Your previous vote is replaced/updated

### "Cannot reduce voting power while delegated"

When delegated, you can vote directly (your power + delegated power counted).
To withdraw power from network, first withdraw delegation.

### "Delegation expired"

Delegations expire at specified block. Create a new delegation:

```javascript
await x3RPC.governance.delegate({
    delegateTo: currentDelegate,
    expiryBlock: currentBlock + 2102400
});
```

## Resources

- **Governance Forum**: https://forum.x3.chain/governance
- **Active Proposals**: https://governance.x3.chain/proposals
- **Delegation Tracker**: https://governance.x3.chain/delegations
- **Developer Guide**: https://docs.x3.chain/governance-api
- **Community FAQ**: https://forum.x3.chain/governance/faq

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**Contact**: governance@x3.chain
