/**
 * crmStore.ts — Global CRM state via Zustand.
 */
import { create } from "zustand";
import * as api from "@/services/crmService";
import type {
  Contact, CreateContactInput, UpdateContactInput,
  CalendarEvent, CreateEventInput, UpdateEventInput,
  Deal, CreateDealInput, UpdateDealInput,
  Activity, CreateActivityInput,
  EmailTemplate, CreateEmailTemplateInput,
  SmtpConfig, SaveSmtpConfigInput,
  SendEmailInput, SentEmail, CrmStats,
} from "@/services/crmService";
import { useSocialStore } from "./socialStore";

/* Re-export for convenience */
export type {
  Contact, CreateContactInput, UpdateContactInput,
  CalendarEvent, CreateEventInput, UpdateEventInput,
  Deal, CreateDealInput, UpdateDealInput,
  Activity, CreateActivityInput,
  EmailTemplate, CreateEmailTemplateInput,
  SmtpConfig, SaveSmtpConfigInput,
  SendEmailInput, SentEmail, CrmStats,
};

interface CrmState {
  /* data */
  contacts: Contact[];
  events: CalendarEvent[];
  deals: Deal[];
  activities: Activity[];
  templates: EmailTemplate[];
  smtpConfig: SmtpConfig | null;
  sentEmails: SentEmail[];
  stats: CrmStats | null;

  /* selected items */
  selectedContact: Contact | null;
  selectedEvent: CalendarEvent | null;
  selectedDeal: Deal | null;

  /* calendar view */
  calendarMonth: Date;

  /* ui */
  loading: boolean;
  error: string | null;

  /* helpers */
  userId: () => string | null;

  /* ── Contacts ── */
  loadContacts: () => Promise<void>;
  createContact: (input: CreateContactInput) => Promise<void>;
  updateContact: (contactId: string, input: UpdateContactInput) => Promise<void>;
  deleteContact: (contactId: string) => Promise<void>;
  selectContact: (contact: Contact | null) => void;

  /* ── Events ── */
  loadEvents: (start?: string, end?: string) => Promise<void>;
  createEvent: (input: CreateEventInput) => Promise<void>;
  updateEvent: (eventId: string, input: UpdateEventInput) => Promise<void>;
  deleteEvent: (eventId: string) => Promise<void>;
  selectEvent: (event: CalendarEvent | null) => void;
  setCalendarMonth: (d: Date) => void;

  /* ── Deals ── */
  loadDeals: () => Promise<void>;
  createDeal: (input: CreateDealInput) => Promise<void>;
  updateDeal: (dealId: string, input: UpdateDealInput) => Promise<void>;
  deleteDeal: (dealId: string) => Promise<void>;
  selectDeal: (deal: Deal | null) => void;

  /* ── Activities ── */
  loadActivities: (contactId?: string) => Promise<void>;
  createActivity: (input: CreateActivityInput) => Promise<void>;

  /* ── Templates ── */
  loadTemplates: () => Promise<void>;
  createTemplate: (input: CreateEmailTemplateInput) => Promise<void>;
  deleteTemplate: (templateId: string) => Promise<void>;

  /* ── SMTP ── */
  loadSmtpConfig: () => Promise<void>;
  saveSmtpConfig: (input: SaveSmtpConfigInput) => Promise<void>;

  /* ── Email ── */
  sendEmail: (input: SendEmailInput) => Promise<void>;
  loadSentEmails: () => Promise<void>;

  /* ── Stats ── */
  loadStats: () => Promise<void>;

  /* ── Bulk loader ── */
  loadAll: () => Promise<void>;
}

export const useCrmStore = create<CrmState>((set, get) => ({
  contacts: [],
  events: [],
  deals: [],
  activities: [],
  templates: [],
  smtpConfig: null,
  sentEmails: [],
  stats: null,
  selectedContact: null,
  selectedEvent: null,
  selectedDeal: null,
  calendarMonth: new Date(),
  loading: false,
  error: null,

  userId: () => useSocialStore.getState().session?.userId ?? null,

  /* ── Contacts ─────────────────────────────────── */
  loadContacts: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const contacts = await api.getContacts(uid);
      set({ contacts });
    } catch (err: any) { set({ error: String(err) }); }
  },

  createContact: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      await api.createContact(uid, input);
      set({ loading: false });
      await get().loadContacts();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  updateContact: async (contactId, input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      const updated = await api.updateContact(contactId, uid, input);
      set({ loading: false, selectedContact: updated });
      await get().loadContacts();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  deleteContact: async (contactId) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      await api.deleteContact(contactId, uid);
      set({ selectedContact: null });
      await get().loadContacts();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err) }); }
  },

  selectContact: (contact) => set({ selectedContact: contact }),

  /* ── Events ───────────────────────────────────── */
  loadEvents: async (start, end) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const events = await api.getEvents(uid, start, end);
      set({ events });
    } catch (err: any) { set({ error: String(err) }); }
  },

  createEvent: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      await api.createEvent(uid, input);
      set({ loading: false });
      await get().loadEvents();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  updateEvent: async (eventId, input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      const updated = await api.updateEvent(eventId, uid, input);
      set({ loading: false, selectedEvent: updated });
      await get().loadEvents();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  deleteEvent: async (eventId) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      await api.deleteEvent(eventId, uid);
      set({ selectedEvent: null });
      await get().loadEvents();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err) }); }
  },

  selectEvent: (event) => set({ selectedEvent: event }),
  setCalendarMonth: (d) => set({ calendarMonth: d }),

  /* ── Deals ────────────────────────────────────── */
  loadDeals: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const deals = await api.getDeals(uid);
      set({ deals });
    } catch (err: any) { set({ error: String(err) }); }
  },

  createDeal: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      await api.createDeal(uid, input);
      set({ loading: false });
      await get().loadDeals();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  updateDeal: async (dealId, input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      const updated = await api.updateDeal(dealId, uid, input);
      set({ loading: false, selectedDeal: updated });
      await get().loadDeals();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  deleteDeal: async (dealId) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      await api.deleteDeal(dealId, uid);
      set({ selectedDeal: null });
      await get().loadDeals();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err) }); }
  },

  selectDeal: (deal) => set({ selectedDeal: deal }),

  /* ── Activities ───────────────────────────────── */
  loadActivities: async (contactId) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const activities = await api.getActivities(uid, contactId);
      set({ activities });
    } catch (err: any) { set({ error: String(err) }); }
  },

  createActivity: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      await api.createActivity(uid, input);
      await get().loadActivities();
    } catch (err: any) { set({ error: String(err) }); }
  },

  /* ── Templates ────────────────────────────────── */
  loadTemplates: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const templates = await api.getEmailTemplates(uid);
      set({ templates });
    } catch (err: any) { set({ error: String(err) }); }
  },

  createTemplate: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      await api.createEmailTemplate(uid, input);
      set({ loading: false });
      await get().loadTemplates();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  deleteTemplate: async (templateId) => {
    const uid = get().userId();
    if (!uid) return;
    try {
      await api.deleteEmailTemplate(templateId, uid);
      await get().loadTemplates();
    } catch (err: any) { set({ error: String(err) }); }
  },

  /* ── SMTP ─────────────────────────────────────── */
  loadSmtpConfig: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const smtpConfig = await api.getSmtpConfig(uid);
      set({ smtpConfig });
    } catch (err: any) { set({ error: String(err) }); }
  },

  saveSmtpConfig: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      const smtpConfig = await api.saveSmtpConfig(uid, input);
      set({ smtpConfig, loading: false });
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  /* ── Email ────────────────────────────────────── */
  sendEmail: async (input) => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true, error: null });
    try {
      await api.sendEmail(uid, input);
      set({ loading: false });
      await get().loadSentEmails();
      await get().loadStats();
    } catch (err: any) { set({ error: String(err), loading: false }); }
  },

  loadSentEmails: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const sentEmails = await api.getSentEmails(uid);
      set({ sentEmails });
    } catch (err: any) { set({ error: String(err) }); }
  },

  /* ── Stats ────────────────────────────────────── */
  loadStats: async () => {
    const uid = get().userId();
    if (!uid) return;
    try {
      const stats = await api.getCrmStats(uid);
      set({ stats });
    } catch (err: any) { set({ error: String(err) }); }
  },

  /* ── Bulk loader ──────────────────────────────── */
  loadAll: async () => {
    const uid = get().userId();
    if (!uid) return;
    set({ loading: true });
    try {
      const [contacts, events, deals, stats, templates, smtpConfig, sentEmails] =
        await Promise.all([
          api.getContacts(uid),
          api.getEvents(uid),
          api.getDeals(uid),
          api.getCrmStats(uid),
          api.getEmailTemplates(uid),
          api.getSmtpConfig(uid),
          api.getSentEmails(uid),
        ]);
      set({ contacts, events, deals, stats, templates, smtpConfig, sentEmails, loading: false });
    } catch (err: any) {
      set({ error: String(err), loading: false });
    }
  },
}));
