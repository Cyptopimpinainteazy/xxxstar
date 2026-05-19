import React, { useEffect } from "react";
import { Link } from "react-router-dom";
import { useCrmStore } from "@/stores/crmStore";
import { format } from "date-fns";

const DashboardPage: React.FC = () => {
  const { stats, events, deals, contacts, loadAll } = useCrmStore();

  useEffect(() => { loadAll(); }, [loadAll]);

  const upcomingEvents = events
    .filter((e) => new Date(e.startAt) >= new Date())
    .sort((a, b) => new Date(a.startAt).getTime() - new Date(b.startAt).getTime())
    .slice(0, 5);

  const recentContacts = [...contacts]
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    .slice(0, 5);

  const openDeals = deals.filter((d) => !d.won && !d.lost);
  const totalPipeline = openDeals.reduce((sum, d) => sum + (d.value || 0), 0);

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Dashboard</h1>
      </div>

      {/* Stat Cards */}
      <div className="crm-stats-grid">
        <div className="crm-stat-card">
          <div className="crm-stat-icon">👥</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">{stats?.contactCount ?? 0}</span>
            <span className="crm-stat-label">Total Contacts</span>
          </div>
        </div>
        <div className="crm-stat-card">
          <div className="crm-stat-icon">💰</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">{stats?.dealCount ?? 0}</span>
            <span className="crm-stat-label">Active Deals</span>
          </div>
        </div>
        <div className="crm-stat-card">
          <div className="crm-stat-icon">🏆</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">{stats?.wonDealCount ?? 0}</span>
            <span className="crm-stat-label">Deals Won</span>
          </div>
        </div>
        <div className="crm-stat-card">
          <div className="crm-stat-icon">📅</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">{stats?.upcomingEvents ?? 0}</span>
            <span className="crm-stat-label">Upcoming Events</span>
          </div>
        </div>
        <div className="crm-stat-card">
          <div className="crm-stat-icon">✉️</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">{stats?.emailSentCount ?? 0}</span>
            <span className="crm-stat-label">Emails Sent</span>
          </div>
        </div>
        <div className="crm-stat-card accent">
          <div className="crm-stat-icon">📈</div>
          <div className="crm-stat-info">
            <span className="crm-stat-value">${totalPipeline.toLocaleString()}</span>
            <span className="crm-stat-label">Pipeline Value</span>
          </div>
        </div>
      </div>

      {/* Two-column layout */}
      <div className="crm-dash-cols">
        {/* Upcoming Events */}
        <div className="crm-card">
          <div className="crm-card-header">
            <h3>📅 Upcoming Events</h3>
            <Link to="/crm/calendar" className="crm-link">View All →</Link>
          </div>
          {upcomingEvents.length === 0 ? (
            <p className="crm-empty">No upcoming events</p>
          ) : (
            <ul className="crm-list">
              {upcomingEvents.map((ev) => (
                <li key={ev.id} className="crm-list-item">
                  <span className="crm-list-dot" style={{ background: ev.color || "#ff6b35" }} />
                  <div className="crm-list-content">
                    <span className="crm-list-title">{ev.title}</span>
                    <span className="crm-list-meta">
                      {format(new Date(ev.startAt), "MMM d, h:mm a")}
                      {ev.location ? ` · ${ev.location}` : ""}
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Recent Contacts */}
        <div className="crm-card">
          <div className="crm-card-header">
            <h3>👥 Recent Contacts</h3>
            <Link to="/crm/contacts" className="crm-link">View All →</Link>
          </div>
          {recentContacts.length === 0 ? (
            <p className="crm-empty">No contacts yet</p>
          ) : (
            <ul className="crm-list">
              {recentContacts.map((c) => (
                <li key={c.id} className="crm-list-item">
                  <div className="crm-avatar-sm">
                    {c.firstName.charAt(0).toUpperCase()}
                  </div>
                  <div className="crm-list-content">
                    <span className="crm-list-title">{c.firstName} {c.lastName}</span>
                    <span className="crm-list-meta">
                      {c.company || c.email || "No details"}
                    </span>
                  </div>
                  {c.stage && <span className="crm-tag">{c.stage}</span>}
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Open Deals */}
        <div className="crm-card">
          <div className="crm-card-header">
            <h3>💰 Open Deals</h3>
            <Link to="/crm/deals" className="crm-link">View All →</Link>
          </div>
          {openDeals.length === 0 ? (
            <p className="crm-empty">No open deals</p>
          ) : (
            <ul className="crm-list">
              {openDeals.slice(0, 5).map((d) => (
                <li key={d.id} className="crm-list-item">
                  <div className="crm-list-content">
                    <span className="crm-list-title">{d.title}</span>
                    <span className="crm-list-meta">
                      ${d.value?.toLocaleString() ?? 0} · {d.stage || "New"} · {d.probability ?? 0}%
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
