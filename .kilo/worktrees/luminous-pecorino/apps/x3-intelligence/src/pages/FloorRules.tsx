// Floor Rules — "Rules of the X3 Floor" displayed as exchange bylaws

export function FloorRules() {
  return (
    <div className="page" style={{ maxWidth: 800 }}>
      <div className="page-header">
        <h1>Rules of the X3 Floor</h1>
      </div>

      <div className="card" style={{ lineHeight: 1.8 }}>
        <Section title="I. Jurisdiction">
          <p>
            X3 is a <strong>deterministic arbitrage jurisdiction</strong>. It is not a DAO.
            It is not governed by votes. It is governed by <strong>law</strong> — encoded
            in the X3 language, executed by the X3 VM, and enforced by deterministic courts.
          </p>
          <p>
            There is no governance token. There is no multisig. There is no council.
            The rules are the rules. Violations are punished automatically.
          </p>
        </Section>

        <Section title="II. Agent Obligations">
          <Clause n="2.1">
            Every agent must post a bond before executing any intent. The bond is
            denominated in the settlement asset and held in escrow by the protocol.
          </Clause>
          <Clause n="2.2">
            Agents must execute within the fee cap declared by the intent submitter.
            Exceeding the fee cap is a slashable offense.
          </Clause>
          <Clause n="2.3">
            Agents must generate an execution proof for every intent they execute.
            Proofs must be deterministically reproducible by replay.
          </Clause>
          <Clause n="2.4">
            An agent's reputation score is computed from their success rate, slash
            history, and total execution volume. Reputation cannot be purchased or
            transferred.
          </Clause>
        </Section>

        <Section title="III. Intent Lifecycle">
          <Clause n="3.1">
            An ArbIntent progresses through a fixed state machine:
            <code style={{ display: "block", margin: "8px 0", padding: "8px 12px", background: "var(--bg-tertiary)", borderRadius: 4, fontSize: 12 }}>
              Submitted → RouteBound → Executing → Executed → Finalized
            </code>
            There are no alternative paths. There are no governance overrides.
          </Clause>
          <Clause n="3.2">
            Route binding is sealed — once a route is committed, the hash of the
            sealed route is recorded on-chain. Deviating from the sealed route
            constitutes a slashable offense.
          </Clause>
          <Clause n="3.3">
            Intents expire after the deadline specified at submission time.
            Expired intents cannot be executed or finalized.
          </Clause>
        </Section>

        <Section title="IV. Slashing Constitution">
          <Clause n="4.1">
            Slashing is <strong>automatic, deterministic, and irreversible</strong>.
            There is no appeal process outside of filing a court dispute that
            triggers deterministic replay.
          </Clause>
          <Clause n="4.2">
            Severity tiers:
          </Clause>
          <table style={{ width: "100%", margin: "8px 0 16px" }}>
            <thead>
              <tr>
                <th>Tier</th>
                <th>Slash %</th>
                <th>Example</th>
              </tr>
            </thead>
            <tbody>
              <tr><td>Minor</td><td className="mono">10%</td><td>Fee cap exceeded by &lt; 5%</td></tr>
              <tr><td>Moderate</td><td className="mono">50%</td><td>Flashloan repayment failure</td></tr>
              <tr><td>Major</td><td className="mono">100%</td><td>State divergence during replay</td></tr>
              <tr><td>Critical</td><td className="mono">100% + deactivation</td><td>Double execution, proof forgery</td></tr>
            </tbody>
          </table>
          <Clause n="4.3">
            All slash events are permanently recorded in an append-only ledger.
            Records include the proof hash, the agent identity, and the violation
            details. No record can be modified or deleted.
          </Clause>
        </Section>

        <Section title="V. Court System">
          <Clause n="5.1">
            Disputes are resolved by <strong>deterministic replay</strong>, not by
            human judgment. The court replays the execution using the submitted
            proof chains and compares results.
          </Clause>
          <Clause n="5.2">
            If replay produces a state divergence, the court issues a Guilty
            verdict and the offending agent is slashed automatically.
          </Clause>
          <Clause n="5.3">
            If replay confirms the original execution, the dispute is dismissed
            and no action is taken.
          </Clause>
        </Section>

        <Section title="VI. Fee Structure">
          <Clause n="6.1">
            Fees are calculated as a vector:
            <code style={{ display: "block", margin: "8px 0", padding: "8px 12px", background: "var(--bg-tertiary)", borderRadius: 4, fontSize: 12 }}>
              TotalFee = BaseFee + ComplexityFee + CapitalFee − ReputationDiscount
            </code>
          </Clause>
          <Clause n="6.2">
            Complexity fee scales with route legs and state touches. Capital fee
            scales logarithmically with borrowed amount. Reputation discount caps
            at 30%.
          </Clause>
          <Clause n="6.3">
            External bots (non-X3 agents) pay a 1.3× penalty. Flashloan usage
            incurs a 1.5× premium. These multipliers ensure X3-native agents
            maintain economic advantage.
          </Clause>
        </Section>

        <Section title="VII. Flashloan Terms">
          <Clause n="7.1">
            Flashloans are <strong>transient execution capital</strong>. Capital is
            never custodied by agents. It is borrowed, used, and repaid within a
            single atomic execution context.
          </Clause>
          <Clause n="7.2">
            Failure at any leg of a cross-chain flashloan reverts the entire
            transaction. No partial fills. No orphaned capital.
          </Clause>
          <Clause n="7.3">
            Default on flashloan repayment results in automatic slashing of the
            agent's bond at Major severity.
          </Clause>
        </Section>

        <Section title="VIII. Execution Guarantees">
          <Clause n="8.1">
            Every execution produces a cryptographic proof chain that binds:
            agent identity, intent parameters, sealed route, state diffs, and
            settlement amounts.
          </Clause>
          <Clause n="8.2">
            Proofs are deterministic — replaying the same inputs must produce
            the same proof hash. Any divergence is evidence of protocol violation.
          </Clause>
          <Clause n="8.3">
            The protocol makes no guarantee of profitability. Agents bear all
            execution risk. The protocol guarantees only fair adjudication.
          </Clause>
        </Section>

        <div style={{ marginTop: 32, padding: "16px 0", borderTop: "1px solid var(--border)", textAlign: "center" }}>
          <span className="muted" style={{ fontSize: 12, fontFamily: "var(--font-mono)" }}>
            X3 ARBITRAGE JURISDICTION — LAW &gt; VOTING
          </span>
        </div>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <h3 style={{
        fontSize: 15,
        fontWeight: 700,
        marginBottom: 12,
        letterSpacing: "-0.01em",
      }}>
        {title}
      </h3>
      {children}
    </div>
  );
}

function Clause({ n, children }: { n: string; children: React.ReactNode }) {
  return (
    <p style={{ marginBottom: 8 }}>
      <span className="mono muted" style={{ fontSize: 12, marginRight: 8 }}>{n}</span>
      {children}
    </p>
  );
}
