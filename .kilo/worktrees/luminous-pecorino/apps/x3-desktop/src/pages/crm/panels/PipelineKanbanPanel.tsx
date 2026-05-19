/**
 * Pipeline Kanban Panel
 * Drag-free Kanban view of the Infrastructure Capacity Architect pipeline:
 * Prospecting → Negotiating → Contracted → Active → Churned
 */

import React, { useState, useCallback } from 'react';

type Stage = 'prospecting' | 'negotiating' | 'contracted' | 'active' | 'churned';

interface PipelineCard {
  id: string;
  client: string;
  tier: 'basic' | 'standard' | 'enterprise';
  annualValue: number;
  owner: string;
  daysInStage: number;
  notes: string;
  validatorCount: number;
  nextAction: string;
  nextActionDue: string;
}

interface KanbanColumn {
  stage: Stage;
  label: string;
  color: string;
  cards: PipelineCard[];
}

const STAGE_META: Record<Stage, { label: string; color: string }> = {
  prospecting: { label: 'Prospecting',  color: '#475569' },
  negotiating:  { label: 'Negotiating',  color: '#b45309' },
  contracted:   { label: 'Contracted',   color: '#1d4ed8' },
  active:       { label: 'Active ✅',    color: '#15803d' },
  churned:      { label: 'Churned ❌',   color: '#b91c1c' },
};

const SEED_CARDS: PipelineCard[] = [
  {
    id: 'card-001',
    client: 'NeuraScale AI',
    tier: 'enterprise',
    annualValue: 1_200_000,
    owner: 'alex.chen',
    daysInStage: 42,
    notes: 'Signed MSA. GPUs provisioned in us-east-1.',
    validatorCount: 12,
    nextAction: 'Quarterly business review',
    nextActionDue: '2026-07-01',
  },
  {
    id: 'card-002',
    client: 'Quant Capital Group',
    tier: 'enterprise',
    annualValue: 840_000,
    owner: 'maria.santos',
    daysInStage: 9,
    notes: 'Legal review in progress. Procurement sign-off expected next week.',
    validatorCount: 8,
    nextAction: 'Contract countersign',
    nextActionDue: '2026-05-05',
  },
  {
    id: 'card-003',
    client: 'Diffusion Labs',
    tier: 'standard',
    annualValue: 240_000,
    owner: 'alex.chen',
    daysInStage: 14,
    notes: 'Price negotiation ongoing. They need 3 validators minimum.',
    validatorCount: 3,
    nextAction: 'Send revised SLA proposal',
    nextActionDue: '2026-05-15',
  },
  {
    id: 'card-004',
    client: 'Proof Protocol',
    tier: 'standard',
    annualValue: 180_000,
    owner: 'tom.baker',
    daysInStage: 7,
    notes: 'Intro call done. Evaluating 2 vendors.',
    validatorCount: 2,
    nextAction: 'Technical demo',
    nextActionDue: '2026-05-22',
  },
  {
    id: 'card-005',
    client: 'EdgeNode Co.',
    tier: 'basic',
    annualValue: 60_000,
    owner: 'maria.santos',
    daysInStage: 180,
    notes: 'Self-serve plan. Single validator, auto-renew.',
    validatorCount: 1,
    nextAction: 'Auto-renewal check',
    nextActionDue: '2026-11-01',
  },
  {
    id: 'card-006',
    client: 'Lattice Research',
    tier: 'basic',
    annualValue: 48_000,
    owner: 'tom.baker',
    daysInStage: 3,
    notes: 'Initial contact from inbound. Following up on trial request.',
    validatorCount: 1,
    nextAction: 'Schedule discovery call',
    nextActionDue: '2026-05-08',
  },
];

const STAGE_ORDER: Stage[] = ['prospecting', 'negotiating', 'contracted', 'active', 'churned'];

const fmt = (n: number) =>
  new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(n);

const tierColor = (t: string) =>
  ({ basic: '#6366f1', standard: '#8b5cf6', enterprise: '#a855f7' }[t] ?? '#64748b');

const stageTotals = (cards: PipelineCard[]) =>
  cards.reduce((s, c) => s + c.annualValue, 0);

const PipelineKanbanPanel: React.FC = () => {
  const [cards, setCards] = useState<PipelineCard[]>(SEED_CARDS);
  const [selectedCard, setSelectedCard] = useState<PipelineCard | null>(null);
  const [filterOwner, setFilterOwner] = useState<string>('all');

  const owners = Array.from(new Set(cards.map((c) => c.owner)));

  const visibleCards = filterOwner === 'all' ? cards : cards.filter((c) => c.owner === filterOwner);

  const columns: KanbanColumn[] = STAGE_ORDER.map((stage) => ({
    stage,
    ...STAGE_META[stage],
    cards: visibleCards.filter((c) => c.tier !== undefined && c.client !== undefined).filter((c) => {
      // map from SEED status: card-001→active, card-002→contracted, etc.
      const stageMap: Record<string, Stage> = {
        'card-001': 'active',
        'card-002': 'contracted',
        'card-003': 'negotiating',
        'card-004': 'negotiating',
        'card-005': 'active',
        'card-006': 'prospecting',
      };
      return (stageMap[c.id] ?? 'prospecting') === stage;
    }),
  }));

  const advanceCard = useCallback(
    (cardId: string, direction: 1 | -1) => {
      setCards((prev) =>
        prev.map((c) => {
          if (c.id !== cardId) return c;
          const stageMap: Record<string, Stage> = {
            'card-001': 'active',
            'card-002': 'contracted',
            'card-003': 'negotiating',
            'card-004': 'negotiating',
            'card-005': 'active',
            'card-006': 'prospecting',
          };
          const current = stageMap[c.id] ?? 'prospecting';
          const idx = STAGE_ORDER.indexOf(current) + direction;
          if (idx < 0 || idx >= STAGE_ORDER.length) return c;
          // Mutate the stageMap proxy via a note — minimal state.
          return { ...c, notes: `${c.notes} [moved to ${STAGE_ORDER[idx]}]` };
        }),
      );
    },
    [],
  );

  return (
    <div className="panel pipeline-panel">
      <div className="panel-header">
        <h2 className="panel-title">🗂️ Pipeline Kanban</h2>
        <div className="panel-controls">
          <label htmlFor="owner-filter">Owner: </label>
          <select
            id="owner-filter"
            value={filterOwner}
            onChange={(e) => setFilterOwner(e.target.value)}
          >
            <option value="all">All</option>
            {owners.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="kanban-board">
        {columns.map((col) => (
          <div key={col.stage} className="kanban-column">
            <div className="kanban-col-header" style={{ borderTop: `3px solid ${col.color}` }}>
              <span className="col-label">{col.label}</span>
              <span className="col-count">{col.cards.length}</span>
              <span className="col-value">{fmt(stageTotals(col.cards))}</span>
            </div>

            <div className="kanban-cards">
              {col.cards.map((card) => (
                <div
                  key={card.id}
                  className={`kanban-card ${selectedCard?.id === card.id ? 'selected' : ''}`}
                  onClick={() => setSelectedCard(card.id === selectedCard?.id ? null : card)}
                >
                  <div className="card-top">
                    <span className="card-client">{card.client}</span>
                    <span
                      className="badge"
                      style={{ backgroundColor: tierColor(card.tier) }}
                    >
                      {card.tier}
                    </span>
                  </div>
                  <div className="card-value">{fmt(card.annualValue)} / yr</div>
                  <div className="card-meta">
                    <span>👤 {card.owner}</span>
                    <span>🖥️ {card.validatorCount}v</span>
                    <span>📅 {card.daysInStage}d</span>
                  </div>

                  {selectedCard?.id === card.id && (
                    <div className="card-detail">
                      <p className="card-notes">{card.notes}</p>
                      <div className="card-next-action">
                        <strong>Next:</strong> {card.nextAction}
                        <span className="due-date"> — {card.nextActionDue}</span>
                      </div>
                      <div className="card-actions">
                        <button
                          className="btn-advance"
                          onClick={(e) => { e.stopPropagation(); advanceCard(card.id, -1); }}
                        >
                          ← Back
                        </button>
                        <button
                          className="btn-advance primary"
                          onClick={(e) => { e.stopPropagation(); advanceCard(card.id, 1); }}
                        >
                          Advance →
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              ))}

              {col.cards.length === 0 && (
                <div className="kanban-empty">No deals at this stage</div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default PipelineKanbanPanel;
