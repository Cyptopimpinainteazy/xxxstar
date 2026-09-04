# X3 Foundry Revenue Model

## Platform Fee Structure

Every dApp created through X3 Foundry includes transparent, configurable platform revenue sharing.

| Revenue Source | Platform Cut | Charged On | Notes |
|---------------|-------------|------------|-------|
| dApp protocol fees | 2% | Gross protocol revenue | Best default. Fair and scalable. |
| NFT sales fee | 2% | Marketplace fee only | Never on full sale price. Avoids user rage. |
| Token launchpad raise | 1–3% | Raised funds | Depends on support level. |
| Subscription apps | 2% | Subscription revenue | Clean SaaS model. |
| Trading bot vaults | 5–10% | Performance fee only | Not principal. |
| AI SaaS apps | 5% | Credit sales | GPU swarm deserves dinner. |
| Domain registry apps | 2–5% | Registration/renewal | Recurring names are juicy. |
| Referral/affiliate flows | 0.25–1% | Referral commission | Optional. |

## Fee Ethics — The Golden Rule

**Take from revenue, not principal.**

Never charge platform revenue share on:
- User deposits
- Collateral principal
- Escrowed funds
- Unearned balances
- Refundable deposits

Charge platform share on:
- Protocol fees
- Marketplace fees
- Subscription payments
- Launch fees
- Trading fees
- Performance fees
- Auction proceeds
- NFT royalties
- Bot rental revenue
- SaaS usage fees
- Premium API fees

## RevenueConfig Structure

```rust
struct RevenueConfig {
    platform_fee_bps: u16,        // default 200 = 2%
    creator_fee_bps: u16,         // default 9700 = 97%
    ai_agent_fee_bps: u16,        // optional 50 = 0.5%
    maintenance_fee_bps: u16,     // optional 50 = 0.5%
    referral_fee_bps: u16,        // optional 50 = 0.5%
    treasury_wallet: address,
    creator_wallet: address,
    maintenance_wallet: address,
    ai_agent_wallet: address,
    referral_wallet: address,
    fee_token: address,
    fee_mode: FeeMode,
}
```

### FeeMode Variants

| Mode | Description |
|------|-------------|
| `GrossRevenue` | Platform takes % of all gross revenue |
| `NetProtocolFees` | Platform takes % of net protocol fees only |
| `SubscriptionRevenue` | Platform takes % of subscription payments |
| `TradingFeesOnly` | Platform takes % of trading fees only |
| `MarketplaceSalesOnly` | Platform takes % of marketplace sales fees only |
| `CreatorDefinedWithPlatformMinimum` | Creator sets fee, platform enforces minimum |

## Treasury Split

Incoming dApp platform fees are routed through `FoundryRevenueRouter` and split:

| Destination | Share | Purpose |
|-------------|-------|---------|
| Protocol Treasury | 40% | Core protocol development and operations |
| GPU Swarm Rewards | 20% | Rewards for GPU compute providers |
| Dev Vault | 15% | Developer ecosystem grants and bounties |
| Maintenance Vault | 10% | Ongoing maintenance and security |
| Liquidity Incentives | 10% | Liquidity provider incentives |
| Grants / Ecosystem | 5% | Community grants and ecosystem growth |

**Configurable by governance with timelock.**

## Creator Earnings

Creators receive 97% of dApp revenue (default). Earnings are:
1. Tracked on-chain per creator address
2. Withdrawable via pull-over-push pattern
3. Never locked or subject to vesting
4. Visible in real-time via analytics dashboard

## Referral/Affiliate System

Optional referral system:
- Referrer sets their code on-chain
- Referred users are linked to referrer
- Referral rewards (0.25–1%) are auto-routed
- Referrers can claim rewards at any time

## Pricing Tiers

| Feature | Free | Builder ($29/mo) | Pro ($99/mo) | Enterprise |
|---------|------|-------------------|--------------|------------|
| Apps/month | 3 | 20 | 50 | Custom |
| Testnet deploys | ✓ | ✓ | ✓ | ✓ |
| Mainnet deploys | — | 10/mo | 50/mo | Custom |
| Platform fee | 2.5% | 2% | 1.5% | Negotiable |
| Templates | Basic | All | All + Advanced | Custom |
| Audits | Basic | Standard | Advanced | Full |
| Custom branding | — | — | ✓ | ✓ |
| Analytics | Basic | Standard | Advanced | Full |
| Support | Community | Email | Priority | Dedicated |

## Revenue Routing Diagram

```
                    ┌─────────────────┐
                    │   dApp Revenue   │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  FeeConfig      │
                    │  Check          │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
     ┌────────────┐  ┌────────────┐  ┌────────────┐
     │ Platform   │  │ Creator    │  │ Referral   │
     │ 2%         │  │ 97%        │  │ 0.25-1%    │
     └──────┬─────┘  └────────────┘  └────────────┘
            │
            ▼
     ┌─────────────────────────────────────┐
     │        Treasury Split               │
     ├─────────────────────────────────────┤
     │ Protocol Treasury   40%             │
     │ GPU Swarm           20%             │
     │ Dev Vault           15%             │
     │ Maintenance Vault   10%             │
     │ Liquidity Incentives 10%            │
     │ Grants               5%             │
     └─────────────────────────────────────┘
```

## Example Calculations

### NFT Marketplace
- Monthly trading volume: 1,000,000 USDC
- Marketplace fee: 2% = 20,000 USDC
- Platform fee (2% of marketplace fee): 400 USDC
- Creator earnings: 19,400 USDC
- Treasury receives: 160 USDC (40% of 400)

### Subscription App
- Monthly subscription revenue: 50,000 USDC
- Platform fee (2%): 1,000 USDC
- Creator earnings: 48,500 USDC
- Referral (0.5%): 250 USDC

### Trading Bot Vault
- AUM: 10,000,000 USDC
- Monthly performance: 5% = 500,000 USDC
- Performance fee (20%): 100,000 USDC
- Platform fee (10% of perf fee): 10,000 USDC
- Creator earnings: 87,000 USDC
