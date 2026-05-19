// Proof Explorer — inspect execution proofs and their verification status

import { useState } from "react";
import type { ExecutionProof } from "../types";

const DEMO_PROOFS: ExecutionProof[] = [
  {
    hash: "e9c1a2b3d4f56789abcdef0123456789e9c1a2b3d4f56789abcdef0123456789",
    intentId: "0xa3f1..8c02",
    agentId: "agent-alpha",
    blockNumber: 18_942_103,
    stateDiffCount: 4,
    timestamp: Date.now() - 12000,
    verified: true,
  },
  {
    hash: "f8a1b2c3d4e56789abcdef0123456789f8a1b2c3d4e56789abcdef0123456789",
    intentId: "0xd4c3..9f87",
    agentId: "agent-delta",
    blockNumber: 18_941_887,
    stateDiffCount: 2,
    timestamp: Date.now() - 60000,
    verified: true,
  },
  {
    hash: "a2b3c4d5e6f78901abcdef0123456789a2b3c4d5e6f78901abcdef0123456789",
    intentId: "0xf9e8..7d6c",
    agentId: "agent-charlie",
    blockNumber: 18_942_200,
    stateDiffCount: 6,
    timestamp: Date.now() - 5000,
    verified: false,
  },
];

export function ProofExplorer() {
  const [proofs] = useState<ExecutionProof[]>(DEMO_PROOFS);
  const [selected, setSelected] = useState<ExecutionProof | null>(null);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Proof Explorer</h1>
        <span className="subtitle">Deterministic execution receipts</span>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
        {/* Proof list */}
        <div className="card">
          <div className="card-header">
            <h2>Recent Proofs</h2>
          </div>
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Hash</th>
                  <th>Block</th>
                  <th>Verified</th>
                </tr>
              </thead>
              <tbody>
                {proofs.map((proof) => (
                  <tr
                    key={proof.hash}
                    onClick={() => setSelected(proof)}
                    style={{ cursor: "pointer" }}
                  >
                    <td className="mono hash">{proof.hash.slice(0, 16)}...</td>
                    <td className="mono" style={{ fontSize: 12 }}>
                      {proof.blockNumber.toLocaleString()}
                    </td>
                    <td>
                      {proof.verified ? (
                        <span className="badge badge-green">verified</span>
                      ) : (
                        <span className="badge badge-amber">pending</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Proof detail */}
        <div className="card">
          <div className="card-header">
            <h2>Proof Detail</h2>
          </div>
          {selected ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <ProofField label="Hash" value={selected.hash} mono />
              <ProofField label="Intent ID" value={selected.intentId} mono />
              <ProofField label="Agent" value={selected.agentId} mono />
              <ProofField
                label="Block"
                value={selected.blockNumber.toLocaleString()}
                mono
              />
              <ProofField
                label="State Diffs"
                value={selected.stateDiffCount.toString()}
                mono
              />
              <ProofField
                label="Timestamp"
                value={new Date(selected.timestamp).toISOString()}
              />
              <ProofField
                label="Verified"
                value={selected.verified ? "YES" : "PENDING"}
                color={selected.verified ? "var(--accent-green)" : "var(--accent-amber)"}
              />
            </div>
          ) : (
            <div className="muted" style={{ padding: 32, textAlign: "center" }}>
              Select a proof to inspect.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ProofField({
  label,
  value,
  mono,
  color,
}: {
  label: string;
  value: string;
  mono?: boolean;
  color?: string;
}) {
  return (
    <div>
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.08em",
          color: "var(--text-muted)",
          marginBottom: 2,
        }}
      >
        {label}
      </div>
      <div
        className={mono ? "mono" : ""}
        style={{
          fontSize: 13,
          wordBreak: "break-all",
          color: color ?? "var(--text-primary)",
        }}
      >
        {value}
      </div>
    </div>
  );
}
