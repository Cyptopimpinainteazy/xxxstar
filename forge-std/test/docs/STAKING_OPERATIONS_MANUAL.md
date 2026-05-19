# X3 Staking Operations Manual

## Getting Started

Staking in X3 allows you to earn rewards by securing the network. Lock your tokens with validators and earn APY-based returns.

### Prerequisites

- X3 tokens (minimum 1 X3 to stake)
- X3 wallet with BiometricAuth enabled
- Active network connection

### Initial Setup

```javascript
import { StakingLedger, RewardCalculator, ValidatorStats } from '@x3/staking-analytics';

// Initialize staking system
const ledger = new StakingLedger();
const calculator = new RewardCalculator();
const validatorStats = new ValidatorStatsManager();

// Import your wallet
const position = await ledger.stake(
    "alice",                // delegator
    "validator_1",          // chosen validator
    1000000000000000000n,   // 1 X3 (in wei)
    10.0,                   // 10% validator commission
    0.5                     // 0.5% unstaking fee
);
```

## Understanding Your Stake

### Staking Position

A staking position tracks all your funds with a single validator:

```javascript
// Get your position
const position = await ledger.get_position("pos_1");

console.log('Status:', position.status); // Active, Locked, Unbonding
console.log('Active Balance:', position.active_balance);
console.log('Locked Until:', position.locked_until);
console.log('Accumulated Rewards:', position.accumulated_rewards);
```

**Position Lifecycle:**

```
CREATED
  ↓
ACTIVE (Earning rewards)
  ↓
LOCKED (Cannot unbond during lock period)
  ↓
UNBONDING (Locked for 28 eras, ~6 days)
  ↓
CLAIMED (Funds returned)
```

### Balance Breakdown

Your stake consists of:
- **Active Balance** — Currently earning rewards
- **Unlocking Balance** — In unbonding period (28 eras)
- **Rewards** — Accumulated but unclaimed

```javascript
position.total_balance()     // active + unbonding
position.accumulated_rewards // unclaimed rewards
position.position_value()    // total balance + rewards
```

## Setting Up Staking

### Choosing a Validator

Evaluate validators before staking:

```javascript
// Get validator statistics
const validator = await validatorStats.get_validator("validator_1");

// Check performance
console.log('Uptime:', validator.performance.uptime_percentage + '%');
console.log('Commission:', validator.commission + '%');
console.log('Backed Amount:', validator.backed_amount);
console.log('Nominator Count:', validator.nominator_count);

// Get overall score
const score = validator.overall_score(); // 0-100
const isRecommended = validator.is_recommended(); // > 80 && uptime > 90%
```

**What to Look For:**

| Metric | Good | Acceptable | Poor |
|--------|------|-----------|------|
| Uptime | > 95% | 90-95% | < 90% |
| Commission | < 5% | 5-10% | > 10% |
| Nominators | 200-500 | 50-200 | < 50 |
| Score | > 85 | 70-85 | < 70 |

### Creating Your First Stake

```javascript
// Stake with your chosen validator
const positionId = await ledger.stake(
    "alice",                    // your address
    "validator_7",              // validator to stake with
    "5000000000000000000n",     // 5 X3
    validator.commission,       // Get from validator stats
    0.5                         // Standard unstaking fee
);

console.log('Position created:', positionId);
```

## Earning Rewards

### APY and Rewards Calculation

```javascript
// Get current APY for your stake
const apy = await calculator.calculate_apy("5000000000000000000n");
console.log('Current APY:', apy.toFixed(2) + '%');

// Estimate monthly rewards
const monthlyReward = await calculator.estimate_reward(
    "5000000000000000000n",  // your stake
    apy,
    30                       // days
);
console.log('Est. monthly reward:', monthlyReward);

// Project annual returns
const calcs = await calculator.get_apy_calculation("5000000000000000000n");
console.log('Annual rewards (gross):', calcs.estimated_annual_reward);
console.log('After commission:', calcs.estimated_annual_reward * (1 - validator.commission/100));
```

### Reward Accrual

Rewards accrue every **era** (~6 hours) automatically:

```javascript
// Rewards are added to accumulated_rewards field
// Check latest reward accrual
const latestRewards = position.accumulated_rewards;

// Rewards continue even during unbonding
// But only active balance balance earns
```

### Compounding

Rewards accrue automatically. Claim and re-stake for compound growth:

```javascript
// Calculate compound effect
const compounded = await calculator.compound_balance(
    "5000000000000000000n",  // initial stake
    apy,
    24                       // months
);
console.log('Balance after 2 years:', compounded);

// Claim and re-stake for compounding
const claimed = await ledger.claim_rewards(positionId);
await ledger.stake(
    "alice",
    "validator_7",
    claimed,                 // Add claimed rewards
    validator.commission,
    0.5
);
```

## Managing Your Stake

### Claiming Rewards

```javascript
// Claim accumulated rewards (no unbonding period)
const rewardsClaimed = await ledger.claim_rewards(positionId);
console.log('Claimed:', rewardsClaimed);

// Rewards applied to wallet balance immediately
```

### Unstaking

To withdraw staked funds:

```javascript
// Begin unbonding (takes 28 eras, ~6 days)
await ledger.unbond(positionId, "1000000000000000000n"); // Unbond 1 X3

// Position status changes to UNBONDING
// After 28 eras:
const claimed = await ledger.claim_unbonded(positionId);
console.log('Funds available:', claimed);
```

**Unbonding Timeline:**

```
Day 0: Call unbond()
  Status: UNBONDING
  Locked for 28 eras

Day 6-7: Era 28 reached
  Status: CLAIMED (available to claim)
  Funds returned to wallet

Day 8: claim_unbonded()
  Funds now in your wallet
```

### Partial Unstaking

You can unbond in stages:

```javascript
const total = "10000000000000000000n"; // 10 X3

// Unbond 3 X3 in phase 1
await ledger.unbond(positionId, "3000000000000000000n");

// Unbond 2 X3 in phase 2  
await ledger.unbond(positionId, "2000000000000000000n");

// Unbond remaining 5 X3
await ledger.unbond(positionId, "5000000000000000000n");

// Position tracks all 3 phases separately
const position = await ledger.get_position(positionId);
console.log('Unbonding phases:', position.unbonding_phases.length); // 3
```

## Risk Management

### Validator Risk Assessment

```javascript
// Get slash tracker for validator
const slashTracker = await validatorStats.slash_tracker.get_history("validator_7");

// Check slashing events
console.log('Slashing count:', slashTracker.history.length);
console.log('Total slashed:', slashTracker.total_slashed);
console.log('Risk score:', slashTracker.risk_score()); // 0-100

// Evaluate risk
if (slashTracker.risk_score() > 50) {
    console.log('⚠️ High risk validator - consider alternatives');
}
```

**Slashing Types:**

| Type | Penalty | Recovery |
|------|---------|----------|
| Offline | 0.01% | ~2 weeks |
| Equivocation | 5-10% | ~2 months |
| Misbehavior | 0-10% | ~3 months |

### Slashing Insurance

Slashing is extremely rare:
- Offline: Happens occasionally, minor penalty
- Equivocation: Requires node failure, major penalty
- Misbehavior: Intentional misconduct, severe penalty

**Protection:**
1. Choose reputable validators (score > 80)
2. Distribute across multiple validators
3. Monitor validator performance quarterly

## Advanced Strategies

### Multi-Validator Staking

Distribute risk across multiple validators:

```javascript
// Stake with 3 different validators
const pos1 = await ledger.stake("alice", "validator_1", "3333333333333333333n", 5, 0.5);
const pos2 = await ledger.stake("alice", "validator_2", "3333333333333333333n", 7, 0.5);
const pos3 = await ledger.stake("alice", "validator_3", "3333333333333333333n", 6, 0.5);

// Monitor all positions
const positions = ledger.delegator_positions("alice");
positions.forEach(pos => {
    console.log(`${pos.validator}: ${pos.active_balance}`);
});
```

### Commission Impact Analysis

```javascript
// Compare net rewards at different commissions
const scenarios = await simulator.commission_impact(
    "10000000000000000000n",  // 10 X3 stake
    10.0,                     // 10% APY
    [5.0, 7.5, 10.0],        // Compare 5%, 7.5%, 10% commission
    12                        // 12 months
);

scenarios.forEach(scenario => {
    const netRewards = scenario.total_net_rewards();
    console.log(`${scenario.validator_commission}%: ${netRewards}`);
});

// Result: 5% commission earns more despite lower gross APY
```

### Long-Term Projection

```javascript
// Project 5-year stakes
const projection = await simulator.project_five_years(
    "10000000000000000000n",  // 10 X3
    12.0,                     // 12% APY estimate
    7.0,                      // 7% validator commission
    0.5                       // 0.5% unstake fee
);

// Monthly growth with compounding
projection.projection_points.forEach(point => {
    console.log(`Month ${point.period}: ${point.projected_balance} (${point.roi_percentage.toFixed(1)}% ROI)`);
});
```

## Monitoring & Maintenance

### Weekly Checklist

- ✓ Check validator uptime (should be > 98%)
- ✓ Verify reward accrual (check for missed eras)
- ✓ Update APY forecast (market conditions change)
- ✓ Review commission announcements

### Monthly Review

```javascript
// Get performance stats
const stats = await validatorStats.get_validator(validator);
const recentSlashes = slashTracker.recent_slashes(validator, 30);

if (recentSlashes.length > 0) {
    console.warn('⚠️ Recent slashing events detected');
} else {
    console.log('✓ No slashing in last 30 days');
}

// Evaluate switching
const alternativeValidator = // find validator with better score
const profit = // calculate profit from switching
```

### Quarterly Portfolio Review

```javascript
// Calculate aggregate statistics
const totalStaked = ledger.delegator_total_balance("alice");
const totalRewards = ledger.delegator_claimable_rewards("alice");
const averageAPY = // avg of all positions
const riskScore = // portfolio risk assessment

console.log('Total Staked:', totalStaked);
console.log('Claimable Rewards:', totalRewards);
console.log('Average APY:', averageAPY.toFixed(2) + '%');
console.log('Portfolio Risk:', riskScore.toFixed(1) + '/100');
```

## Troubleshooting

### Rewards Stopped Accruing

**Causes:**
1. **Validator offline** — Check validator status (should return online soon)
2. **You got slashed** — Check slashing history
3. **Position expired** — Recreate position

**Solution:**
```javascript
const validator = await validatorStats.get_validator(validator_id);
if (!validator.performance.uptime_percentage > 50) {
    console.log('Validator is offline - move to different validator');
    // Unbond and stake with new validator
}
```

### Cannot Claim Unbonded Funds

**Cause:** 28 eras haven't passed yet

**Solution:**
```javascript
const phase = position.unbonding_phases[0];
const erasRemaining = phase.unlock_era - current_era;
console.log(`Claim available in ${erasRemaining} eras (~${erasRemaining * 6} hours)`);
```

### Validator Commission Increased

This is allowed but should reset low. Monitor commission changes quarterly.

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**Support**: staking-support@x3.chain
