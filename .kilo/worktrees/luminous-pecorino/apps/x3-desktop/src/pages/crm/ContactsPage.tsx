import React, { useEffect, useState } from "react";
import { useCrmStore } from "@/stores/crmStore";
import type { CreateContactInput, UpdateContactInput, Contact } from "@/stores/crmStore";
import { format } from "date-fns";

const STAGES = ["lead", "prospect", "qualified", "customer", "churned"];
const PRIORITIES = ["low", "medium", "high"];

const blankContact: CreateContactInput = {
  firstName: "", lastName: "", email: "", phone: "",
  company: "", jobTitle: "", address: "", city: "",
  state: "", zip: "", country: "", website: "",
  notes: "", tags: "", source: "", stage: "lead", priority: "medium",
};

const ContactsPage: React.FC = () => {
  const {
    contacts, loadContacts, createContact, updateContact, deleteContact,
    selectedContact, selectContact,
  } = useCrmStore();

  const [showModal, setShowModal] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [search, setSearch] = useState("");
  const [stageFilter, setStageFilter] = useState<string>("all");
  const [form, setForm] = useState<CreateContactInput>(blankContact);

  useEffect(() => { loadContacts(); }, [loadContacts]);

  const filtered = contacts.filter((c) => {
    const matchSearch =
      `${c.firstName} ${c.lastName} ${c.email} ${c.company}`.toLowerCase().includes(search.toLowerCase());
    const matchStage = stageFilter === "all" || c.stage === stageFilter;
    return matchSearch && matchStage;
  });

  const openNew = () => {
    setForm(blankContact);
    setEditMode(false);
    selectContact(null);
    setShowModal(true);
  };

  const openEdit = (c: Contact) => {
    selectContact(c);
    setForm({
      firstName: c.firstName, lastName: c.lastName, email: c.email, phone: c.phone,
      company: c.company, jobTitle: c.jobTitle, address: c.address, city: c.city,
      state: c.state, zip: c.zip, country: c.country, website: c.website,
      notes: c.notes, tags: c.tags, source: c.source, stage: c.stage, priority: c.priority,
    });
    setEditMode(true);
    setShowModal(true);
  };

  const handleSave = async () => {
    if (!form.firstName.trim()) return;
    if (editMode && selectedContact) {
      await updateContact(selectedContact.id, form as UpdateContactInput);
    } else {
      await createContact(form);
    }
    setShowModal(false);
  };

  const handleDelete = async () => {
    if (selectedContact && confirm(`Delete ${selectedContact.firstName} ${selectedContact.lastName}?`)) {
      await deleteContact(selectedContact.id);
      setShowModal(false);
    }
  };

  const stageColor: Record<string, string> = {
    lead: "#3b82f6", prospect: "#a855f7", qualified: "#eab308",
    customer: "#22c55e", churned: "#ef4444",
  };

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Contacts ({contacts.length})</h1>
        <button className="crm-btn primary" onClick={openNew}>+ New Contact</button>
      </div>

      {/* Filters */}
      <div className="crm-filters">
        <input
          className="crm-search"
          placeholder="Search contacts..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <div className="crm-filter-pills">
          <button
            className={`crm-pill ${stageFilter === "all" ? "active" : ""}`}
            onClick={() => setStageFilter("all")}
          >All</button>
          {STAGES.map((s) => (
            <button
              key={s}
              className={`crm-pill ${stageFilter === s ? "active" : ""}`}
              style={stageFilter === s ? { background: stageColor[s], color: "#fff" } : {}}
              onClick={() => setStageFilter(s)}
            >{s}</button>
          ))}
        </div>
      </div>

      {/* Contact List */}
      <div className="crm-table-wrap">
        <table className="crm-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Email</th>
              <th>Company</th>
              <th>Phone</th>
              <th>Stage</th>
              <th>Priority</th>
              <th>Added</th>
            </tr>
          </thead>
          <tbody>
            {filtered.length === 0 ? (
              <tr><td colSpan={7} className="crm-empty-cell">No contacts found</td></tr>
            ) : (
              filtered.map((c) => (
                <tr key={c.id} className="crm-table-row" onClick={() => openEdit(c)}>
                  <td>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <div className="crm-avatar-sm">{c.firstName.charAt(0).toUpperCase()}</div>
                      <span>{c.firstName} {c.lastName}</span>
                    </div>
                  </td>
                  <td>{c.email}</td>
                  <td>{c.company}</td>
                  <td>{c.phone}</td>
                  <td>
                    <span className="crm-tag" style={{ background: stageColor[c.stage] || "#666" }}>
                      {c.stage || "—"}
                    </span>
                  </td>
                  <td>
                    <span className={`crm-priority ${c.priority}`}>{c.priority || "—"}</span>
                  </td>
                  <td>{c.createdAt ? format(new Date(c.createdAt), "MMM d") : ""}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Contact Modal */}
      {showModal && (
        <div className="crm-modal-overlay" onClick={() => setShowModal(false)}>
          <div className="crm-modal large" onClick={(e) => e.stopPropagation()}>
            <h2>{editMode ? "Edit Contact" : "New Contact"}</h2>
            <div className="crm-form">
              <div className="crm-form-row">
                <div><label>First Name *</label><input value={form.firstName} onChange={(e) => setForm({ ...form, firstName: e.target.value })} /></div>
                <div><label>Last Name</label><input value={form.lastName || ""} onChange={(e) => setForm({ ...form, lastName: e.target.value })} /></div>
              </div>
              <div className="crm-form-row">
                <div><label>Email</label><input type="email" value={form.email || ""} onChange={(e) => setForm({ ...form, email: e.target.value })} /></div>
                <div><label>Phone</label><input value={form.phone || ""} onChange={(e) => setForm({ ...form, phone: e.target.value })} /></div>
              </div>
              <div className="crm-form-row">
                <div><label>Company</label><input value={form.company || ""} onChange={(e) => setForm({ ...form, company: e.target.value })} /></div>
                <div><label>Job Title</label><input value={form.jobTitle || ""} onChange={(e) => setForm({ ...form, jobTitle: e.target.value })} /></div>
              </div>
              <div className="crm-form-row">
                <div><label>Address</label><input value={form.address || ""} onChange={(e) => setForm({ ...form, address: e.target.value })} /></div>
                <div><label>City</label><input value={form.city || ""} onChange={(e) => setForm({ ...form, city: e.target.value })} /></div>
              </div>
              <div className="crm-form-row">
                <div><label>State</label><input value={form.state || ""} onChange={(e) => setForm({ ...form, state: e.target.value })} /></div>
                <div><label>Zip</label><input value={form.zip || ""} onChange={(e) => setForm({ ...form, zip: e.target.value })} /></div>
                <div><label>Country</label><input value={form.country || ""} onChange={(e) => setForm({ ...form, country: e.target.value })} /></div>
              </div>
              <div className="crm-form-row">
                <div><label>Website</label><input value={form.website || ""} onChange={(e) => setForm({ ...form, website: e.target.value })} /></div>
                <div><label>Source</label><input value={form.source || ""} onChange={(e) => setForm({ ...form, source: e.target.value })} placeholder="e.g. referral, web, event" /></div>
              </div>
              <div className="crm-form-row">
                <div>
                  <label>Stage</label>
                  <select value={form.stage || "lead"} onChange={(e) => setForm({ ...form, stage: e.target.value })}>
                    {STAGES.map((s) => <option key={s} value={s}>{s}</option>)}
                  </select>
                </div>
                <div>
                  <label>Priority</label>
                  <select value={form.priority || "medium"} onChange={(e) => setForm({ ...form, priority: e.target.value })}>
                    {PRIORITIES.map((p) => <option key={p} value={p}>{p}</option>)}
                  </select>
                </div>
              </div>
              <label>Tags</label>
              <input value={form.tags || ""} onChange={(e) => setForm({ ...form, tags: e.target.value })} placeholder="comma-separated" />
              <label>Notes</label>
              <textarea value={form.notes || ""} onChange={(e) => setForm({ ...form, notes: e.target.value })} rows={3} />

              <div className="crm-form-actions">
                {editMode && <button className="crm-btn danger" onClick={handleDelete}>Delete</button>}
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

export default ContactsPage;
