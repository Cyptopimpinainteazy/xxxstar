/**
 * crmService.ts — Tauri invoke wrappers for the CRM backend.
 */
// Use a lazy, guarded tauri invoke helper to avoid runtime errors in browser dev
export async function fallbackInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
} 

/* ─── Types ──────────────────────────────────────── */

export interface Contact {
  id: string;
  ownerUserId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  company: string;
  jobTitle: string;
  avatarUrl: string;
  address: string;
  city: string;
  state: string;
  zip: string;
  country: string;
  website: string;
  notes: string;
  tags: string;
  source: string;
  stage: string;
  priority: string;
  lastContacted: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateContactInput {
  firstName: string;
  lastName?: string;
  email?: string;
  phone?: string;
  company?: string;
  jobTitle?: string;
  address?: string;
  city?: string;
  state?: string;
  zip?: string;
  country?: string;
  website?: string;
  notes?: string;
  tags?: string;
  source?: string;
  stage?: string;
  priority?: string;
}

export interface UpdateContactInput {
  firstName?: string;
  lastName?: string;
  email?: string;
  phone?: string;
  company?: string;
  jobTitle?: string;
  avatarUrl?: string;
  address?: string;
  city?: string;
  state?: string;
  zip?: string;
  country?: string;
  website?: string;
  notes?: string;
  tags?: string;
  source?: string;
  stage?: string;
  priority?: string;
}

export interface CalendarEvent {
  id: string;
  ownerUserId: string;
  title: string;
  description: string;
  location: string;
  eventType: string;
  startAt: string;
  endAt: string;
  allDay: boolean;
  color: string;
  recurrence: string;
  reminderMins: number;
  contactId: string;
  dealId: string;
  completed: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateEventInput {
  title: string;
  description?: string;
  location?: string;
  eventType?: string;
  startAt: string;
  endAt: string;
  allDay?: boolean;
  color?: string;
  recurrence?: string;
  reminderMins?: number;
  contactId?: string;
  dealId?: string;
}

export interface UpdateEventInput {
  title?: string;
  description?: string;
  location?: string;
  eventType?: string;
  startAt?: string;
  endAt?: string;
  allDay?: boolean;
  color?: string;
  recurrence?: string;
  reminderMins?: number;
  contactId?: string;
  dealId?: string;
  completed?: boolean;
}

export interface Deal {
  id: string;
  ownerUserId: string;
  contactId: string;
  title: string;
  value: number;
  currency: string;
  stage: string;
  probability: number;
  expectedClose: string;
  notes: string;
  won: boolean;
  lost: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateDealInput {
  contactId?: string;
  title: string;
  value?: number;
  currency?: string;
  stage?: string;
  probability?: number;
  expectedClose?: string;
  notes?: string;
}

export interface UpdateDealInput {
  contactId?: string;
  title?: string;
  value?: number;
  currency?: string;
  stage?: string;
  probability?: number;
  expectedClose?: string;
  notes?: string;
  won?: boolean;
  lost?: boolean;
}

export interface Activity {
  id: string;
  ownerUserId: string;
  contactId: string;
  dealId: string;
  eventId: string;
  activityType: string;
  subject: string;
  body: string;
  createdAt: string;
}

export interface CreateActivityInput {
  contactId?: string;
  dealId?: string;
  eventId?: string;
  activityType: string;
  subject?: string;
  body?: string;
}

export interface EmailTemplate {
  id: string;
  ownerUserId: string;
  name: string;
  subject: string;
  body: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateEmailTemplateInput {
  name: string;
  subject: string;
  body: string;
}

export interface SmtpConfig {
  id: string;
  ownerUserId: string;
  host: string;
  port: number;
  username: string;
  fromName: string;
  fromEmail: string;
  useTls: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface SaveSmtpConfigInput {
  host: string;
  port?: number;
  username: string;
  password: string;
  fromName: string;
  fromEmail: string;
  useTls?: boolean;
}

export interface SendEmailInput {
  toEmail: string;
  subject: string;
  body: string;
  contactId?: string;
  templateId?: string;
}

export interface SentEmail {
  id: string;
  ownerUserId: string;
  contactId: string;
  toEmail: string;
  subject: string;
  body: string;
  status: string;
  errorMessage: string;
  templateId: string;
  createdAt: string;
}

export interface CrmStats {
  contactCount: number;
  dealCount: number;
  openDealValue: number;
  wonDealCount: number;
  eventCount: number;
  upcomingEvents: number;
  emailSentCount: number;
  activityCount: number;
}

interface BrowserCrmUserData {
  contacts: Contact[];
  events: CalendarEvent[];
  deals: Deal[];
  activities: Activity[];
  templates: EmailTemplate[];
  smtpConfig: (SmtpConfig & { password?: string }) | null;
  sentEmails: SentEmail[];
}

type BrowserCrmDb = Record<string, BrowserCrmUserData>;

const CRM_BROWSER_STORAGE_KEY = "x3-crm-browser-db";

const hasTauriRuntime = () =>
  typeof window !== "undefined" && (((window as any).__TAURI_INTERNALS__) || ((window as any).__TAURI__));

const nowIso = () => new Date().toISOString();
const createId = (prefix: string) =>
  `${prefix}_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;

const makeEmptyUserData = (): BrowserCrmUserData => ({
  contacts: [],
  events: [],
  deals: [],
  activities: [],
  templates: [],
  smtpConfig: null,
  sentEmails: [],
});

const loadBrowserDb = (): BrowserCrmDb => {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(CRM_BROWSER_STORAGE_KEY);
    return raw ? JSON.parse(raw) as BrowserCrmDb : {};
  } catch {
    return {};
  }
};

const saveBrowserDb = (db: BrowserCrmDb) => {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CRM_BROWSER_STORAGE_KEY, JSON.stringify(db));
};

const readUserData = (userId: string): BrowserCrmUserData => {
  const db = loadBrowserDb();
  return db[userId] ?? makeEmptyUserData();
};

const mutateUserData = <T,>(userId: string, updater: (data: BrowserCrmUserData) => T): T => {
  const db = loadBrowserDb();
  const data = db[userId] ?? makeEmptyUserData();
  const result = updater(data);
  db[userId] = data;
  saveBrowserDb(db);
  return result;
};

const computeStats = (data: BrowserCrmUserData): CrmStats => {
  const now = Date.now();
  const openDeals = data.deals.filter((deal) => !deal.won && !deal.lost);
  const upcomingEvents = data.events.filter((event) => {
    const eventTime = Date.parse(event.startAt);
    return !Number.isNaN(eventTime) && eventTime >= now && !event.completed;
  });

  return {
    contactCount: data.contacts.length,
    dealCount: openDeals.length,
    openDealValue: openDeals.reduce((sum, deal) => sum + (deal.value || 0), 0),
    wonDealCount: data.deals.filter((deal) => deal.won).length,
    eventCount: data.events.length,
    upcomingEvents: upcomingEvents.length,
    emailSentCount: data.sentEmails.filter((email) => email.status === "sent").length,
    activityCount: data.activities.length,
  };
};

async function runCrmCommand<T>(cmd: string, args?: any): Promise<T> {
  if (hasTauriRuntime()) {
    return fallbackInvoke<T>(cmd, args);
  }

  return runBrowserCrmCommand<T>(cmd, args);
}

async function runBrowserCrmCommand<T>(cmd: string, args: any): Promise<T> {
  switch (cmd) {
    case "crm_create_contact":
      return mutateUserData(args.userId, (data) => {
        const timestamp = nowIso();
        const contact: Contact = {
          id: createId("contact"),
          ownerUserId: args.userId,
          firstName: args.input.firstName,
          lastName: args.input.lastName ?? "",
          email: args.input.email ?? "",
          phone: args.input.phone ?? "",
          company: args.input.company ?? "",
          jobTitle: args.input.jobTitle ?? "",
          avatarUrl: "",
          address: args.input.address ?? "",
          city: args.input.city ?? "",
          state: args.input.state ?? "",
          zip: args.input.zip ?? "",
          country: args.input.country ?? "",
          website: args.input.website ?? "",
          notes: args.input.notes ?? "",
          tags: args.input.tags ?? "",
          source: args.input.source ?? "",
          stage: args.input.stage ?? "lead",
          priority: args.input.priority ?? "medium",
          lastContacted: "",
          createdAt: timestamp,
          updatedAt: timestamp,
        };
        data.contacts.unshift(contact);
        return contact as T;
      });

    case "crm_update_contact":
      return mutateUserData(args.userId, (data) => {
        const index = data.contacts.findIndex((contact) => contact.id === args.contactId);
        if (index < 0) throw new Error("Contact not found");
        const updated: Contact = {
          ...data.contacts[index],
          ...args.input,
          updatedAt: nowIso(),
        };
        data.contacts[index] = updated;
        return updated as T;
      });

    case "crm_get_contacts":
      return readUserData(args.userId).contacts as T;

    case "crm_get_contact": {
      const contact = readUserData(args.userId).contacts.find((item) => item.id === args.contactId);
      if (!contact) throw new Error("Contact not found");
      return contact as T;
    }

    case "crm_delete_contact":
      mutateUserData(args.userId, (data) => {
        data.contacts = data.contacts.filter((contact) => contact.id !== args.contactId);
        data.deals = data.deals.map((deal) => deal.contactId === args.contactId ? { ...deal, contactId: "" } : deal);
        data.events = data.events.map((event) => event.contactId === args.contactId ? { ...event, contactId: "" } : event);
        data.activities = data.activities.filter((activity) => activity.contactId !== args.contactId);
      });
      return undefined as T;

    case "crm_create_event":
      return mutateUserData(args.userId, (data) => {
        const timestamp = nowIso();
        const event: CalendarEvent = {
          id: createId("event"),
          ownerUserId: args.userId,
          title: args.input.title,
          description: args.input.description ?? "",
          location: args.input.location ?? "",
          eventType: args.input.eventType ?? "meeting",
          startAt: args.input.startAt,
          endAt: args.input.endAt,
          allDay: args.input.allDay ?? false,
          color: args.input.color ?? "#ff6b35",
          recurrence: args.input.recurrence ?? "",
          reminderMins: args.input.reminderMins ?? 15,
          contactId: args.input.contactId ?? "",
          dealId: args.input.dealId ?? "",
          completed: false,
          createdAt: timestamp,
          updatedAt: timestamp,
        };
        data.events.push(event);
        data.events.sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt));
        return event as T;
      });

    case "crm_update_event":
      return mutateUserData(args.userId, (data) => {
        const index = data.events.findIndex((event) => event.id === args.eventId);
        if (index < 0) throw new Error("Event not found");
        const updated: CalendarEvent = {
          ...data.events[index],
          ...args.input,
          updatedAt: nowIso(),
        };
        data.events[index] = updated;
        data.events.sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt));
        return updated as T;
      });

    case "crm_get_events": {
      const events = readUserData(args.userId).events.filter((event) => {
        const start = args.start ? Date.parse(args.start) : null;
        const end = args.end ? Date.parse(args.end) : null;
        const eventStart = Date.parse(event.startAt);
        if (start !== null && eventStart < start) return false;
        if (end !== null && eventStart > end) return false;
        return true;
      });
      return events as T;
    }

    case "crm_delete_event":
      mutateUserData(args.userId, (data) => {
        data.events = data.events.filter((event) => event.id !== args.eventId);
        data.activities = data.activities.filter((activity) => activity.eventId !== args.eventId);
      });
      return undefined as T;

    case "crm_create_deal":
      return mutateUserData(args.userId, (data) => {
        const timestamp = nowIso();
        const deal: Deal = {
          id: createId("deal"),
          ownerUserId: args.userId,
          contactId: args.input.contactId ?? "",
          title: args.input.title,
          value: Number(args.input.value ?? 0),
          currency: args.input.currency ?? "USD",
          stage: args.input.stage ?? "discovery",
          probability: Number(args.input.probability ?? 10),
          expectedClose: args.input.expectedClose ?? "",
          notes: args.input.notes ?? "",
          won: false,
          lost: false,
          createdAt: timestamp,
          updatedAt: timestamp,
        };
        data.deals.unshift(deal);
        return deal as T;
      });

    case "crm_update_deal":
      return mutateUserData(args.userId, (data) => {
        const index = data.deals.findIndex((deal) => deal.id === args.dealId);
        if (index < 0) throw new Error("Deal not found");
        const updated: Deal = {
          ...data.deals[index],
          ...args.input,
          updatedAt: nowIso(),
        };
        data.deals[index] = updated;
        return updated as T;
      });

    case "crm_get_deals":
      return readUserData(args.userId).deals as T;

    case "crm_delete_deal":
      mutateUserData(args.userId, (data) => {
        data.deals = data.deals.filter((deal) => deal.id !== args.dealId);
        data.events = data.events.map((event) => event.dealId === args.dealId ? { ...event, dealId: "" } : event);
        data.activities = data.activities.filter((activity) => activity.dealId !== args.dealId);
      });
      return undefined as T;

    case "crm_create_activity":
      return mutateUserData(args.userId, (data) => {
        const activity: Activity = {
          id: createId("activity"),
          ownerUserId: args.userId,
          contactId: args.input.contactId ?? "",
          dealId: args.input.dealId ?? "",
          eventId: args.input.eventId ?? "",
          activityType: args.input.activityType,
          subject: args.input.subject ?? "",
          body: args.input.body ?? "",
          createdAt: nowIso(),
        };
        data.activities.unshift(activity);
        return activity as T;
      });

    case "crm_get_activities": {
      const activities = readUserData(args.userId).activities.filter((activity) =>
        args.contactId ? activity.contactId === args.contactId : true,
      );
      return activities as T;
    }

    case "crm_create_email_template":
      return mutateUserData(args.userId, (data) => {
        const timestamp = nowIso();
        const template: EmailTemplate = {
          id: createId("template"),
          ownerUserId: args.userId,
          name: args.input.name,
          subject: args.input.subject,
          body: args.input.body,
          createdAt: timestamp,
          updatedAt: timestamp,
        };
        data.templates.unshift(template);
        return template as T;
      });

    case "crm_get_email_templates":
      return readUserData(args.userId).templates as T;

    case "crm_delete_email_template":
      mutateUserData(args.userId, (data) => {
        data.templates = data.templates.filter((template) => template.id !== args.templateId);
      });
      return undefined as T;

    case "crm_save_smtp_config":
      return mutateUserData(args.userId, (data) => {
        const timestamp = nowIso();
        const smtpConfig: SmtpConfig & { password?: string } = {
          id: data.smtpConfig?.id ?? createId("smtp"),
          ownerUserId: args.userId,
          host: args.input.host,
          port: Number(args.input.port ?? 587),
          username: args.input.username,
          fromName: args.input.fromName,
          fromEmail: args.input.fromEmail,
          useTls: args.input.useTls ?? true,
          createdAt: data.smtpConfig?.createdAt ?? timestamp,
          updatedAt: timestamp,
          password: args.input.password ?? data.smtpConfig?.password ?? "",
        };
        data.smtpConfig = smtpConfig;
        const { password: _password, ...publicConfig } = smtpConfig;
        return publicConfig as T;
      });

    case "crm_get_smtp_config": {
      const smtpConfig = readUserData(args.userId).smtpConfig;
      if (!smtpConfig) return null as T;
      const { password: _password, ...publicConfig } = smtpConfig;
      return publicConfig as T;
    }

    case "crm_send_email":
      return mutateUserData(args.userId, (data) => {
        if (!data.smtpConfig) throw new Error("SMTP not configured");
        const email: SentEmail = {
          id: createId("email"),
          ownerUserId: args.userId,
          contactId: args.input.contactId ?? "",
          toEmail: args.input.toEmail,
          subject: args.input.subject,
          body: args.input.body,
          status: "sent",
          errorMessage: "",
          templateId: args.input.templateId ?? "",
          createdAt: nowIso(),
        };
        data.sentEmails.unshift(email);
        return email as T;
      });

    case "crm_get_sent_emails":
      return readUserData(args.userId).sentEmails as T;

    case "crm_get_stats":
      return computeStats(readUserData(args.userId)) as T;

    default:
      throw new Error(`Unsupported CRM command in browser mode: ${cmd}`);
  }
}

/* ─── Contacts ───────────────────────────────────── */
export const createContact = (userId: string, input: CreateContactInput) =>
  runCrmCommand<Contact>("crm_create_contact", { userId, input });

export const updateContact = (contactId: string, userId: string, input: UpdateContactInput) =>
  runCrmCommand<Contact>("crm_update_contact", { contactId, userId, input });

export const getContacts = (userId: string) =>
  runCrmCommand<Contact[]>("crm_get_contacts", { userId });

export const getContact = (contactId: string, userId: string) =>
  runCrmCommand<Contact>("crm_get_contact", { contactId, userId });

export const deleteContact = (contactId: string, userId: string) =>
  runCrmCommand<void>("crm_delete_contact", { contactId, userId });

/* ─── Calendar Events ────────────────────────────── */
export const createEvent = (userId: string, input: CreateEventInput) =>
  runCrmCommand<CalendarEvent>("crm_create_event", { userId, input });

export const updateEvent = (eventId: string, userId: string, input: UpdateEventInput) =>
  runCrmCommand<CalendarEvent>("crm_update_event", { eventId, userId, input });

export const getEvents = (userId: string, start?: string, end?: string) =>
  runCrmCommand<CalendarEvent[]>("crm_get_events", { userId, start: start ?? null, end: end ?? null });

export const deleteEvent = (eventId: string, userId: string) =>
  runCrmCommand<void>("crm_delete_event", { eventId, userId });

/* ─── Deals ──────────────────────────────────────── */
export const createDeal = (userId: string, input: CreateDealInput) =>
  runCrmCommand<Deal>("crm_create_deal", { userId, input });

export const updateDeal = (dealId: string, userId: string, input: UpdateDealInput) =>
  runCrmCommand<Deal>("crm_update_deal", { dealId, userId, input });

export const getDeals = (userId: string) =>
  runCrmCommand<Deal[]>("crm_get_deals", { userId });

export const deleteDeal = (dealId: string, userId: string) =>
  runCrmCommand<void>("crm_delete_deal", { dealId, userId });

/* ─── Activities ─────────────────────────────────── */
export const createActivity = (userId: string, input: CreateActivityInput) =>
  runCrmCommand<Activity>("crm_create_activity", { userId, input });

export const getActivities = (userId: string, contactId?: string) =>
  runCrmCommand<Activity[]>("crm_get_activities", { userId, contactId: contactId ?? null });

/* ─── Email Templates ────────────────────────────── */
export const createEmailTemplate = (userId: string, input: CreateEmailTemplateInput) =>
  runCrmCommand<EmailTemplate>("crm_create_email_template", { userId, input });

export const getEmailTemplates = (userId: string) =>
  runCrmCommand<EmailTemplate[]>("crm_get_email_templates", { userId });

export const deleteEmailTemplate = (templateId: string, userId: string) =>
  runCrmCommand<void>("crm_delete_email_template", { templateId, userId });

/* ─── SMTP ───────────────────────────────────────── */
export const saveSmtpConfig = (userId: string, input: SaveSmtpConfigInput) =>
  runCrmCommand<SmtpConfig>("crm_save_smtp_config", { userId, input });

export const getSmtpConfig = (userId: string) =>
  runCrmCommand<SmtpConfig | null>("crm_get_smtp_config", { userId });

/* ─── Send Email ─────────────────────────────────── */
export const sendEmail = (userId: string, input: SendEmailInput) =>
  runCrmCommand<SentEmail>("crm_send_email", { userId, input });

export const getSentEmails = (userId: string) =>
  runCrmCommand<SentEmail[]>("crm_get_sent_emails", { userId });

/* ─── Stats ──────────────────────────────────────── */
export const getCrmStats = (userId: string) =>
  runCrmCommand<CrmStats>("crm_get_stats", { userId });
