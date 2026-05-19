import React, { useEffect, useState, useMemo } from "react";
import { useCrmStore } from "@/stores/crmStore";
import type { CreateEventInput, CalendarEvent } from "@/stores/crmStore";
import {
  format, startOfMonth, endOfMonth, startOfWeek, endOfWeek,
  addDays, addMonths, subMonths, isSameMonth, isSameDay, isToday,
} from "date-fns";

const EVENT_COLORS = ["#ff6b35", "#a855f7", "#22c55e", "#3b82f6", "#ef4444", "#eab308"];

const CalendarPage: React.FC = () => {
  const {
    events, calendarMonth, contacts,
    loadEvents, createEvent, updateEvent, deleteEvent,
    selectEvent, selectedEvent, setCalendarMonth,
  } = useCrmStore();

  const [showModal, setShowModal] = useState(false);
  const [editMode, setEditMode] = useState(false);

  const blankForm: CreateEventInput = {
    title: "", description: "", location: "", eventType: "meeting",
    startAt: "", endAt: "", allDay: false, color: "#ff6b35",
    recurrence: "", reminderMins: 15, contactId: "", dealId: "",
  };
  const [form, setForm] = useState<CreateEventInput>(blankForm);

  useEffect(() => { loadEvents(); }, [loadEvents]);

  /* ─── Calendar grid ─── */
  const weeks = useMemo(() => {
    const monthStart = startOfMonth(calendarMonth);
    const monthEnd = endOfMonth(calendarMonth);
    const gridStart = startOfWeek(monthStart, { weekStartsOn: 0 });
    const gridEnd = endOfWeek(monthEnd, { weekStartsOn: 0 });
    const rows: Date[][] = [];
    let day = gridStart;
    while (day <= gridEnd) {
      const week: Date[] = [];
      for (let i = 0; i < 7; i++) { week.push(day); day = addDays(day, 1); }
      rows.push(week);
    }
    return rows;
  }, [calendarMonth]);

  const eventsForDay = (d: Date) =>
    events.filter((e) => isSameDay(new Date(e.startAt), d));

  /* ─── Handlers ─── */
  const openNew = (day?: Date) => {
    const base = day ?? new Date();
    const startAt = format(base, "yyyy-MM-dd'T'HH:mm");
    const endAt = format(addDays(base, 0), "yyyy-MM-dd'T'HH:mm");
    setForm({ ...blankForm, startAt, endAt });
    setEditMode(false);
    setShowModal(true);
  };

  const openEdit = (ev: CalendarEvent) => {
    selectEvent(ev);
    setForm({
      title: ev.title, description: ev.description, location: ev.location,
      eventType: ev.eventType, startAt: ev.startAt.replace(" ", "T").slice(0, 16),
      endAt: ev.endAt.replace(" ", "T").slice(0, 16), allDay: ev.allDay,
      color: ev.color || "#ff6b35", recurrence: ev.recurrence,
      reminderMins: ev.reminderMins, contactId: ev.contactId, dealId: ev.dealId,
    });
    setEditMode(true);
    setShowModal(true);
  };

  const handleSave = async () => {
    if (!form.title.trim()) return;
    if (editMode && selectedEvent) {
      await updateEvent(selectedEvent.id, form);
    } else {
      await createEvent(form);
    }
    setShowModal(false);
    selectEvent(null);
  };

  const handleDelete = async () => {
    if (selectedEvent && confirm("Delete this event?")) {
      await deleteEvent(selectedEvent.id);
      setShowModal(false);
      selectEvent(null);
    }
  };

  const prevMonth = () => setCalendarMonth(subMonths(calendarMonth, 1));
  const nextMonth = () => setCalendarMonth(addMonths(calendarMonth, 1));
  const goToday = () => setCalendarMonth(new Date());

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Calendar</h1>
        <button className="crm-btn primary" onClick={() => openNew()}>+ New Event</button>
      </div>

      {/* Month navigation */}
      <div className="crm-calendar-nav">
        <button className="crm-btn-icon" onClick={prevMonth}>◀</button>
        <h2>{format(calendarMonth, "MMMM yyyy")}</h2>
        <button className="crm-btn-icon" onClick={nextMonth}>▶</button>
        <button className="crm-btn sm" onClick={goToday}>Today</button>
      </div>

      {/* Calendar Grid */}
      <div className="crm-calendar-grid">
        {["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map((d) => (
          <div key={d} className="crm-cal-header">{d}</div>
        ))}
        {weeks.flat().map((day, i) => {
          const dayEvents = eventsForDay(day);
          const inMonth = isSameMonth(day, calendarMonth);
          return (
            <div
              key={i}
              className={`crm-cal-cell ${!inMonth ? "muted" : ""} ${isToday(day) ? "today" : ""}`}
              onClick={() => openNew(day)}
            >
              <span className="crm-cal-day">{format(day, "d")}</span>
              <div className="crm-cal-events">
                {dayEvents.slice(0, 3).map((ev) => (
                  <div
                    key={ev.id}
                    className="crm-cal-event"
                    style={{ borderLeftColor: ev.color || "#ff6b35" }}
                    onClick={(e) => { e.stopPropagation(); openEdit(ev); }}
                    title={ev.title}
                  >
                    {ev.title}
                  </div>
                ))}
                {dayEvents.length > 3 && (
                  <span className="crm-cal-more">+{dayEvents.length - 3} more</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Event Modal */}
      {showModal && (
        <div className="crm-modal-overlay" onClick={() => setShowModal(false)}>
          <div className="crm-modal" onClick={(e) => e.stopPropagation()}>
            <h2>{editMode ? "Edit Event" : "New Event"}</h2>
            <div className="crm-form">
              <label>Title *</label>
              <input value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />

              <label>Description</label>
              <textarea value={form.description || ""} onChange={(e) => setForm({ ...form, description: e.target.value })} rows={3} />

              <div className="crm-form-row">
                <div>
                  <label>Start</label>
                  <input type="datetime-local" value={form.startAt} onChange={(e) => setForm({ ...form, startAt: e.target.value })} />
                </div>
                <div>
                  <label>End</label>
                  <input type="datetime-local" value={form.endAt} onChange={(e) => setForm({ ...form, endAt: e.target.value })} />
                </div>
              </div>

              <div className="crm-form-row">
                <div>
                  <label>Location</label>
                  <input value={form.location || ""} onChange={(e) => setForm({ ...form, location: e.target.value })} />
                </div>
                <div>
                  <label>Type</label>
                  <select value={form.eventType || "meeting"} onChange={(e) => setForm({ ...form, eventType: e.target.value })}>
                    <option value="meeting">Meeting</option>
                    <option value="call">Call</option>
                    <option value="task">Task</option>
                    <option value="deadline">Deadline</option>
                    <option value="reminder">Reminder</option>
                    <option value="other">Other</option>
                  </select>
                </div>
              </div>

              <div className="crm-form-row">
                <div>
                  <label>Color</label>
                  <div style={{ display: "flex", gap: 6 }}>
                    {EVENT_COLORS.map((c) => (
                      <button
                        key={c}
                        className={`crm-color-swatch ${form.color === c ? "selected" : ""}`}
                        style={{ background: c }}
                        onClick={() => setForm({ ...form, color: c })}
                      />
                    ))}
                  </div>
                </div>
                <div>
                  <label>Reminder (mins)</label>
                  <input type="number" value={form.reminderMins ?? 15} onChange={(e) => setForm({ ...form, reminderMins: +e.target.value })} />
                </div>
              </div>

              <div className="crm-form-row">
                <div>
                  <label>Contact</label>
                  <select value={form.contactId || ""} onChange={(e) => setForm({ ...form, contactId: e.target.value })}>
                    <option value="">None</option>
                    {contacts.map((c) => (
                      <option key={c.id} value={c.id}>{c.firstName} {c.lastName}</option>
                    ))}
                  </select>
                </div>
                <label className="crm-checkbox-row">
                  <input type="checkbox" checked={form.allDay ?? false} onChange={(e) => setForm({ ...form, allDay: e.target.checked })} />
                  All Day
                </label>
              </div>

              <div className="crm-form-actions">
                {editMode && (
                  <button className="crm-btn danger" onClick={handleDelete}>Delete</button>
                )}
                <div style={{ flex: 1 }} />
                <button className="crm-btn" onClick={() => setShowModal(false)}>Cancel</button>
                <button className="crm-btn primary" onClick={handleSave}>
                  {editMode ? "Update" : "Create"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CalendarPage;
