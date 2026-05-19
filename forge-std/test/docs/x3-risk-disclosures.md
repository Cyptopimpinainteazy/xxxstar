# X3 Risk Disclosures

*Read before participating. The protocol does not protect you from yourself.*

---

## General Risk Statement

Participation in the X3 arbitrage jurisdiction involves significant financial risk. By registering as an agent, submitting intents, or providing flashloan liquidity, you accept the following risks in full.

---

## 1. Bond Loss Risk

Your bond is at risk of slashing. Slashing is:

- **Automatic**: Triggered by protocol-detected violations.
- **Deterministic**: The same violation always produces the same penalty.
- **Irreversible**: Slashed funds cannot be recovered.

You may lose **up to 100% of your bond** in a single event. Critical violations additionally result in permanent deactivation and identity burn.

There is no insurance fund. There is no safety net. There is no appeal process beyond filing a dispute that triggers deterministic replay.

---

## 2. Execution Risk

Intent execution may result in financial loss due to:

- **Price movement**: Prices may move adversely between route binding and execution.
- **Slippage**: Actual execution prices may differ from expected prices.
- **Gas costs**: Transaction costs may exceed estimates.
- **Competition**: Other agents or external bots may capture the same opportunity.
- **Failed execution**: Cross-chain legs may fail, causing full revert and wasted gas.

The protocol guarantees atomic settlement (no partial fills), but it does not guarantee that the atomic outcome will be profitable.

---

## 3. Flashloan Risk

Flashloan usage introduces additional risk vectors:

- **Repayment failure**: If the execution does not generate sufficient output to repay the flashloan (principal + premium), the entire transaction reverts. Gas costs are still incurred.
- **Premium variability**: Flashloan premiums are computed dynamically via the fee curve engine and may be higher than expected.
- **Liquidity gaps**: Flashloan pools may have insufficient liquidity for the requested amount.
- **Cascading reverts**: If any leg of a cross-chain flashloan execution fails, all legs revert. The agent bears the gas cost on every chain.

---

## 4. Smart Contract Risk

The X3 protocol is implemented as smart contracts and runtime code. While every effort has been made to ensure correctness:

- **Bugs may exist**: Software is fallible. Undiscovered bugs could result in incorrect slashing, lost funds, or protocol malfunction.
- **No formal verification**: Not all components have been formally verified.
- **Upgrade risk**: Protocol upgrades may change rules, fee structures, or slash penalties. Changes apply to all future activity.
- **No warranty**: The protocol is provided "as is" without warranty of any kind.

---

## 5. Blockchain Risk

The X3 protocol operates on the underlying blockchain and inherits its risk profile:

- **Consensus failures**: Chain reorganizations, forks, or consensus halts may affect execution and settlement.
- **Finality delays**: Transactions may take longer than expected to finalize.
- **MEV exposure**: Even with X3's fee structure advantages, transactions may be subject to MEV extraction by validators or block builders.
- **Network congestion**: High gas prices may make execution economically infeasible.

---

## 6. Economic Design Risk

The X3 fee structure and reputation system create economic dynamics that may not behave as expected:

- **Fee structure changes**: Protocol upgrades may alter fee calculations, affecting profitability.
- **Reputation loss**: A single slashing event can significantly reduce your reputation and increase your fees.
- **Market structure changes**: External market conditions may reduce arbitrage opportunities.
- **Competitive dynamics**: As more agents join, per-agent profitability may decrease.

---

## 7. Irreversibility Risk

X3 is designed with **no admin keys, no emergency pause, no governance override**. This means:

- Mistakes cannot be manually corrected.
- Slashing cannot be reversed, even if the violation was caused by an external factor (e.g., chain reorganization).
- Lost access to agent keys means permanent loss of bond and identity.
- There is no customer support.

---

## 8. Regulatory Risk

The legal and regulatory status of decentralized arbitrage systems varies by jurisdiction. You are solely responsible for:

- Determining whether your participation complies with applicable laws.
- Reporting any income or gains as required.
- Understanding the tax implications of bond posting, slashing, and flashloan usage.

The protocol makes no representation regarding regulatory compliance.

---

## 9. No Recourse

By participating in X3, you acknowledge:

1. You have read and understood these risk disclosures.
2. You accept all enumerated risks and any risks not explicitly listed.
3. You waive any claim against the protocol, its developers, or other participants for losses incurred through participation.
4. You understand that the protocol is autonomous and cannot be compelled to act differently by any party.

---

## Summary

| Risk | Mitigation |
|---|---|
| Bond loss | Post only what you can afford to lose. Execute carefully. |
| Execution loss | Model profitability before execution. Account for worst-case gas and slippage. |
| Flashloan failure | Ensure sufficient margin. Test execution paths thoroughly. |
| Smart contract bug | Monitor protocol announcements. Diversify across systems. |
| Blockchain failure | Use chains with strong finality guarantees. |
| Irreversibility | Secure your keys. Double-check before executing. |

**The protocol enforces rules. It does not protect participants from risk.**

---

*X3 Arbitrage Jurisdiction — Risk Disclosures v1.0*
