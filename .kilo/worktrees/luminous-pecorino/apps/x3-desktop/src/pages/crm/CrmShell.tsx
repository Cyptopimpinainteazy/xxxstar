import React, { useEffect } from "react";
import { Routes, Route, Link, useLocation, useNavigate } from "react-router-dom";
import { useCrmStore } from "@/stores/crmStore";
import DashboardPage from "./DashboardPage";
import CalendarPage from "./CalendarPage";
import ContactsPage from "./ContactsPage";
import DealsPage from "./DealsPage";
import EmailPage from "./EmailPage";
import SettingsPage from "./SettingsPage";
import AgentsPage from "./AgentsPage";

const NAV_ITEMS = [
  { path: "/crm",          label: "Dashboard", icon: "📊" },
  { path: "/crm/agents",    label: "AI Agents", icon: "🤖" },
  { path: "/crm/calendar",  label: "Calendar",  icon: "📅" },
  { path: "/crm/contacts",  label: "Contacts",  icon: "👥" },
  { path: "/crm/deals",     label: "Deals",     icon: "💰" },
  { path: "/crm/email",     label: "Email",     icon: "✉️" },
  { path: "/crm/settings",  label: "Settings",  icon: "⚙️" },
];

const CrmShell: React.FC = () => {
  const { loadAll, stats } = useCrmStore();
  const location = useLocation();
  const navigate = useNavigate();

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  return (
    <div className="crm-app">
      {/* Sidebar */}
      <aside className="crm-sidebar">
        <div className="crm-sidebar-header">
          <Link to="/crm" className="crm-logo">
            <span className="crm-logo-icon">📋</span>
            <span className="crm-logo-text">X3 CRM</span>
          </Link>
        </div>

        <nav className="crm-nav">
          {NAV_ITEMS.map((item) => {
            const isActive = location.pathname === item.path ||
              (item.path !== "/crm" && location.pathname.startsWith(item.path));
            return (
              <Link
                key={item.path}
                to={item.path}
                className={`crm-nav-item ${isActive ? "active" : ""}`}
              >
                <span className="crm-nav-icon">{item.icon}</span>
                <span className="crm-nav-label">{item.label}</span>
              </Link>
            );
          })}
        </nav>

        {/* Quick Stats */}
        {stats && (
          <div className="crm-sidebar-stats">
            <div className="crm-stat-mini">
              <span className="crm-stat-mini-val">{stats.contactCount}</span>
              <span className="crm-stat-mini-lbl">Contacts</span>
            </div>
            <div className="crm-stat-mini">
              <span className="crm-stat-mini-val">{stats.dealCount}</span>
              <span className="crm-stat-mini-lbl">Deals</span>
            </div>
            <div className="crm-stat-mini">
              <span className="crm-stat-mini-val">{stats.upcomingEvents}</span>
              <span className="crm-stat-mini-lbl">Upcoming</span>
            </div>
          </div>
        )}

        {/* Back to Desktop */}
        <div className="crm-sidebar-footer">
          <Link to="/" className="crm-back-link">← Desktop</Link>
          <Link to="/social" className="crm-back-link">🌐 Social</Link>
        </div>
      </aside>

      {/* Main Content */}
      <main className="crm-main">
        <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 16 }}>
          <button
            onClick={() => navigate("/")}
            className="crm-btn primary"
            style={{ boxShadow: "0 4px 12px rgba(255, 107, 53, 0.3)" }}
          >
            ← Back to Desktop
          </button>
        </div>
        <Routes>
          <Route index element={<DashboardPage />} />
          <Route path="agents" element={<AgentsPage />} />
          <Route path="calendar" element={<CalendarPage />} />
          <Route path="contacts" element={<ContactsPage />} />
          <Route path="deals" element={<DealsPage />} />
          <Route path="email" element={<EmailPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
};

export default CrmShell;
