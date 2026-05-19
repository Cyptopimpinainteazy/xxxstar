import React, { useEffect, useState } from "react";
import { useCrmStore } from "@/stores/crmStore";
import type { CreateDealInput, UpdateDealInput, Deal } from "@/stores/crmStore";

const DEAL_STAGES = ["discovery", "proposal", "negotiation", "contract", "closed"];

const stageColor: Record<string, string> = {
  discovery: "#3b82f6", proposal: "#a855f7", negotiation: "#eab308",
  contract: "#f97316", closed: "#22c55e",
};

const blankDeal: CreateDealInput = {
  contactId: "", title: "", value: 0, currency: "USD",
  stage: "discovery", probability: 10, expectedClose: "", notes: "",
};

const DealsPage: React.FC = () => {
  const {
    deals, contacts, loadDeals, createDeal, updateDeal, deleteDeal,
    selectedDeal, selectDeal, loadContacts,
  } = useCrmStore();

  const [view, setView] = useState<"pipeline" | "list">("pipeline");
  const [showModal, setShowModal] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [form, setForm] = useState<CreateDealInput>(blankDeal);

  useEffect(() => { loadDeals(); loadContacts(); }, [loadDeals, loadContacts]);

  const openDeals = deals.filter((d) => !d.won && !d.lost);
  const totalPipeline = openDeals.reduce((s, d) => s + (d.value || 0), 0);
  const wonDeals = deals.filter((d) => d.won);
  const lostDeals = deals.filter((d) => d.lost);

  const openNew = () => {
    setForm(blankDeal);
    setEditMode(false);
    selectDeal(null);
    setShowModal(true);
  };

  const openEdit = (d: Deal) => {
    selectDeal(d);
    setForm({
      contactId: d.contactId, title: d.title, value: d.value,
      currency: d.currency, stage: d.stage, probability: d.probability,
      expectedClose: d.expectedClose, notes: d.notes,
    });
    setEditMode(true);
    setShowModal(true);
  };

  const handleSave = async () => {
    if (!form.title.trim()) return;
    if (editMode && selectedDeal) {
      await updateDeal(selectedDeal.id, form as UpdateDealInput);
    } else {
      await createDeal(form);
    }
    setShowModal(false);
  };

  const handleDelete = async () => {
    if (selectedDeal && confirm(`Delete deal "${selectedDeal.title}"?`)) {
      await deleteDeal(selectedDeal.id);
      setShowModal(false);
    }
  };

  const markWon = async () => {
    if (selectedDeal) {
      await updateDeal(selectedDeal.id, { won: true, lost: false });
      setShowModal(false);
    }
  };

  const markLost = async () => {
    if (selectedDeal) {
      await updateDeal(selectedDeal.id, { won: false, lost: true });
      setShowModal(false);
    }
  };

  const contactName = (id: string) => {
    const c = contacts.find((c) => c.id === id);
    return c ? `${c.firstName} ${c.lastName}` : "";
  };

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Deals</h1>
        <div style={{ display: "flex", gap: 8 }}>
          <div className="crm-filter-pills">
            <button className={`crm-pill ${view === "pipeline" ? "active" : ""}`} onClick={() => setView("pipeline")}>Pipeline</button>
            <button className={`crm-pill ${view === "list" ? "active" : ""}`} onClick={() => setView("list")}>List</button>
          </div>
          <button className="crm-btn primary" onClick={openNew}>+ New Deal</button>
        </div>
      </div>

      {/* Summary */}
      <div className="crm-stats-grid small">
        <div className="crm-stat-card compact">
          <span className="crm-stat-value">${totalPipeline.toLocaleString()}</span>
          <span className="crm-stat-label">Pipeline</span>
        </div>
        <div className="crm-stat-card compact">
          <span className="crm-stat-value">{openDeals.length}</span>
          <span className="crm-stat-label">Open</span>
        </div>
        <div className="crm-stat-card compact" style={{ borderColor: "#22c55e" }}>
          <span className="crm-stat-value">{wonDeals.length}</span>
          <span className="crm-stat-label">Won</span>
        </div>
        <div className="crm-stat-card compact" style={{ borderColor: "#ef4444" }}>
          <span className="crm-stat-value">{lostDeals.length}</span>
          <span className="crm-stat-label">Lost</span>
        </div>
      </div>

      {/* Pipeline View */}
      {view === "pipeline" && (
        <div className="crm-pipeline">
          {DEAL_STAGES.map((stage) => {
            const stageDeals = openDeals.filter((d) => (d.stage || "discovery") === stage);
            const stageTotal = stageDeals.reduce((s, d) => s + (d.value || 0), 0);
            return (
              <div key={stage} className="crm-pipeline-col">
                <div className="crm-pipeline-header" style={{ borderTopColor: stageColor[stage] }}>
                  <span className="crm-pipeline-title">{stage}</span>
                  <span className="crm-pipeline-count">{stageDeals.length} · ${stageTotal.toLocaleString()}</span>
                </div>
                <div className="crm-pipeline-cards">
                  {stageDeals.map((d) => (
                    <div key={d.id} className="crm-pipeline-card" onClick={() => openEdit(d)}>
                      <div className="crm-pipeline-card-title">{d.title}</div>
                      <div className="crm-pipeline-card-value">${(d.value || 0).toLocaleString()}</div>
                      {d.contactId && <div className="crm-pipeline-card-contact">{contactName(d.contactId)}</div>}
                      <div className="crm-pipeline-card-prob">{d.probability ?? 0}% likely</div>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* List View */}
      {view === "list" && (
        <div className="crm-table-wrap">
          <table className="crm-table">
            <thead>
              <tr>
                <th>Title</th>
                <th>Value</th>
                <th>Stage</th>
                <th>Probability</th>
                <th>Contact</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {deals.length === 0 ? (
                <tr><td colSpan={6} className="crm-empty-cell">No deals yet</td></tr>
              ) : (
                deals.map((d) => (
                  <tr key={d.id} className="crm-table-row" onClick={() => openEdit(d)}>
                    <td>{d.title}</td>
                    <td>${(d.value || 0).toLocaleString()}</td>
                    <td><span className="crm-tag" style={{ background: stageColor[d.stage] || "#666" }}>{d.stage || "—"}</span></td>
                    <td>{d.probability ?? 0}%</td>
                    <td>{d.contactId ? contactName(d.contactId) : "—"}</td>
                    <td>{d.won ? "🏆 Won" : d.lost ? "❌ Lost" : "🔵 Open"}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* Deal Modal */}
      {showModal && (
        <div className="crm-modal-overlay" onClick={() => setShowModal(false)}>
          <div className="crm-modal" onClick={(e) => e.stopPropagation()}>
            <h2>{editMode ? "Edit Deal" : "New Deal"}</h2>
            <div className="crm-form">
              <label>Title *</label>
              <input value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />

              <div className="crm-form-row">
                <div>
                  <label>Value</label>
                  <input type="number" value={form.value || 0} onChange={(e) => setForm({ ...form, value: +e.target.value })} />
                </div>
                <div>
                  <label>Currency</label>
                  <input value={form.currency || "USD"} onChange={(e) => setForm({ ...form, currency: e.target.value })} />
                </div>
              </div>

              <div className="crm-form-row">
                <div>
                  <label>Stage</label>
                  <select value={form.stage || "discovery"} onChange={(e) => setForm({ ...form, stage: e.target.value })}>
                    {DEAL_STAGES.map((s) => <option key={s} value={s}>{s}</option>)}
                  </select>
                </div>
                <div>
                  <label>Probability %</label>
                  <input type="number" min={0} max={100} value={form.probability ?? 10} onChange={(e) => setForm({ ...form, probability: +e.target.value })} />
                </div>
              </div>

              <div className="crm-form-row">
                <div>
                  <label>Contact</label>
                  <select value={form.contactId || ""} onChange={(e) => setForm({ ...form, contactId: e.target.value })}>
                    <option value="">None</option>
                    {contacts.map((c) => <option key={c.id} value={c.id}>{c.firstName} {c.lastName}</option>)}
                  </select>
                </div>
                <div>
                  <label>Expected Close</label>
                  <input type="date" value={form.expectedClose || ""} onChange={(e) => setForm({ ...form, expectedClose: e.target.value })} />
                </div>
              </div>

              <label>Notes</label>
              <textarea value={form.notes || ""} onChange={(e) => setForm({ ...form, notes: e.target.value })} rows={3} />

              <div className="crm-form-actions">
                {editMode && (
                  <>
                    <button className="crm-btn danger" onClick={handleDelete}>Delete</button>
                    <button className="crm-btn success" onClick={markWon}>🏆 Won</button>
                    <button className="crm-btn danger-outline" onClick={markLost}>❌ Lost</button>
                  </>
                )}
                <div style={{ flex: 1 }} />
                <button className="crm-btn" onClick={() => setShowModal(false)}>Cancel</button>
                <button className="crm-btn primary" onClick={handleSave}>{editMode ? "Update" : "Create"}</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default DealsPage;
