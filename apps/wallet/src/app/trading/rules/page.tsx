'use client';

export default function FloorRules() {
  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-3xl font-bold mb-2">Rules of the X3 Floor</h1>
      <div className="text-gray-400 mb-6">Governance through law, not votes</div>
      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray space-y-4 text-gray-300">
        <section>
          <h2 className="text-lg font-bold text-white mb-2">I. Jurisdiction</h2>
          <p>
            X3 is a <strong>deterministic arbitrage jurisdiction</strong>. It is governed by law — encoded
            in the X3 language and executed by the X3 VM.
          </p>
        </section>
        <section>
          <h2 className="text-lg font-bold text-white mb-2">II. Agent Obligations</h2>
          <ul className="space-y-1 ml-4">
            <li>• Every agent must post a bond before executing any intent</li>
            <li>• Agents must execute within the fee cap declared</li>
            <li>• Agents must generate an execution proof for every intent</li>
          </ul>
        </section>
        <section>
          <h2 className="text-lg font-bold text-white mb-2">III. Slashing</h2>
          <p>
            Slashing is <strong>automatic, deterministic, and irreversible</strong>.
            There is no appeal process outside of filing a court dispute.
          </p>
        </section>
      </div>
    </div>
  );
}
