import React, { useEffect, useState } from "react";
import { useCrmStore } from "@/stores/crmStore";
import type { SendEmailInput } from "@/stores/crmStore";
import { format } from "date-fns";

const EmailPage: React.FC = () => {
  const {
    sentEmails, contacts, templates, smtpConfig,
    loadSentEmails, loadContacts, loadTemplates, loadSmtpConfig,
    sendEmail, loading, error,
  } = useCrmStore();

  const [tab, setTab] = useState<"compose" | "sent" | "templates">("compose");
  const [form, setForm] = useState<SendEmailInput>({
    toEmail: "", subject: "", body: "", contactId: "", templateId: "",
  });
  const [sendSuccess, setSendSuccess] = useState(false);

  useEffect(() => {
    loadSentEmails();
    loadContacts();
    loadTemplates();
    loadSmtpConfig();
  }, [loadSentEmails, loadContacts, loadTemplates, loadSmtpConfig]);

  const handleContactSelect = (contactId: string) => {
    const c = contacts.find((c) => c.id === contactId);
    setForm({
      ...form,
      contactId,
      toEmail: c?.email || form.toEmail,
    });
  };

  const handleTemplateSelect = (templateId: string) => {
    const t = templates.find((t) => t.id === templateId);
    if (t) {
      setForm({
        ...form,
        templateId,
        subject: t.subject,
        body: t.body,
      });
    }
  };

  const handleSend = async () => {
    if (!form.toEmail || !form.subject) return;
    if (!smtpConfig) {
      alert("Please configure SMTP settings first (Settings page).");
      return;
    }
    setSendSuccess(false);
    await sendEmail(form);
    if (!error) {
      setSendSuccess(true);
      setForm({ toEmail: "", subject: "", body: "", contactId: "", templateId: "" });
      setTimeout(() => setSendSuccess(false), 3000);
    }
  };

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Email</h1>
        {!smtpConfig && (
          <span className="crm-alert-inline">⚠️ SMTP not configured — go to Settings</span>
        )}
      </div>

      {/* Tabs */}
      <div className="crm-tabs">
        <button className={`crm-tab ${tab === "compose" ? "active" : ""}`} onClick={() => setTab("compose")}>✏️ Compose</button>
        <button className={`crm-tab ${tab === "sent" ? "active" : ""}`} onClick={() => setTab("sent")}>📤 Sent ({sentEmails.length})</button>
        <button className={`crm-tab ${tab === "templates" ? "active" : ""}`} onClick={() => setTab("templates")}>📋 Templates</button>
      </div>

      {/* Compose Tab */}
      {tab === "compose" && (
        <div className="crm-card" style={{ maxWidth: 700 }}>
          {sendSuccess && <div className="crm-success-banner">✅ Email sent successfully!</div>}
          {error && <div className="crm-error-banner">❌ {error}</div>}

          <div className="crm-form">
            <div className="crm-form-row">
              <div style={{ flex: 2 }}>
                <label>To Email *</label>
                <input
                  type="email"
                  value={form.toEmail}
                  onChange={(e) => setForm({ ...form, toEmail: e.target.value })}
                  placeholder="recipient@example.com"
                />
              </div>
              <div style={{ flex: 1 }}>
                <label>Or select contact</label>
                <select value={form.contactId || ""} onChange={(e) => handleContactSelect(e.target.value)}>
                  <option value="">Pick contact...</option>
                  {contacts.filter((c) => c.email).map((c) => (
                    <option key={c.id} value={c.id}>{c.firstName} {c.lastName} ({c.email})</option>
                  ))}
                </select>
              </div>
            </div>

            {templates.length > 0 && (
              <>
                <label>Template</label>
                <select value={form.templateId || ""} onChange={(e) => handleTemplateSelect(e.target.value)}>
                  <option value="">No template</option>
                  {templates.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
                </select>
              </>
            )}

            <label>Subject *</label>
            <input value={form.subject} onChange={(e) => setForm({ ...form, subject: e.target.value })} placeholder="Email subject" />

            <label>Body (HTML supported)</label>
            <textarea
              value={form.body}
              onChange={(e) => setForm({ ...form, body: e.target.value })}
              rows={10}
              placeholder="Write your email content here..."
            />

            <div className="crm-form-actions">
              <div style={{ flex: 1 }} />
              <button className="crm-btn primary" onClick={handleSend} disabled={loading}>
                {loading ? "Sending..." : "📤 Send Email"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Sent Tab */}
      {tab === "sent" && (
        <div className="crm-table-wrap">
          <table className="crm-table">
            <thead>
              <tr>
                <th>To</th>
                <th>Subject</th>
                <th>Status</th>
                <th>Date</th>
              </tr>
            </thead>
            <tbody>
              {sentEmails.length === 0 ? (
                <tr><td colSpan={4} className="crm-empty-cell">No emails sent yet</td></tr>
              ) : (
                sentEmails.map((e) => (
                  <tr key={e.id}>
                    <td>{e.toEmail}</td>
                    <td>{e.subject}</td>
                    <td>
                      <span className={`crm-tag ${e.status === "sent" ? "success" : "danger"}`}>
                        {e.status}
                      </span>
                    </td>
                    <td>{e.createdAt ? format(new Date(e.createdAt), "MMM d, h:mm a") : ""}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* Templates Tab */}
      {tab === "templates" && (
        <div>
          <TemplateManager />
        </div>
      )}
    </div>
  );
};

/* ─── Inline Template Manager ─── */
const TemplateManager: React.FC = () => {
  const { templates, createTemplate, deleteTemplate, loadTemplates } = useCrmStore();
  const [name, setName] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");

  useEffect(() => { loadTemplates(); }, [loadTemplates]);

  const handleCreate = async () => {
    if (!name.trim() || !subject.trim()) return;
    await createTemplate({ name, subject, body });
    setName(""); setSubject(""); setBody("");
  };

  return (
    <div>
      <div className="crm-card" style={{ maxWidth: 600, marginBottom: 16 }}>
        <h3>Create Template</h3>
        <div className="crm-form">
          <label>Name</label>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Welcome Email" />
          <label>Subject</label>
          <input value={subject} onChange={(e) => setSubject(e.target.value)} />
          <label>Body</label>
          <textarea value={body} onChange={(e) => setBody(e.target.value)} rows={5} />
          <button className="crm-btn primary" onClick={handleCreate}>Save Template</button>
        </div>
      </div>

      {templates.length > 0 && (
        <div className="crm-card">
          <h3>Saved Templates</h3>
          <ul className="crm-list">
            {templates.map((t) => (
              <li key={t.id} className="crm-list-item" style={{ justifyContent: "space-between" }}>
                <div>
                  <strong>{t.name}</strong>
                  <div className="crm-list-meta">Subject: {t.subject}</div>
                </div>
                <button className="crm-btn danger sm" onClick={() => deleteTemplate(t.id)}>Delete</button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};

export default EmailPage;
