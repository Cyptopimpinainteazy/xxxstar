/**
 * agentService.ts — Tauri invoke wrappers for the 15-agent surgical swarm CRM backend.
 * 4 Layers: Strategic • Execution • Media • Growth
 * All powered by local Ollama (free, no API keys).
 */
import { invoke } from '@tauri-apps/api/core';
// Use a lazy guarded invoke helper to avoid browser crashes when Tauri is not present
export async function fallbackInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
} 

/* ─── Types ──────────────────────────────────────── */

export interface AgentDef {
  id: string;
  name: string;
  role: string;
  layer: string;
  avatar: string;
  color: string;
  model: string;
  system_prompt: string;
  capabilities: string[];
  status: string;
}

export interface AgentTask {
  id: string;
  agent_id: string;
  owner_user_id: string;
  assigned_to_user_id: string;
  task_type: string;
  prompt: string;
  result: string;
  status: string;
  leads_generated: number;
  created_at: string;
  completed_at: string;
}

export interface AgentConversation {
  id: string;
  agent_id: string;
  user_id: string;
  role: string;
  content: string;
  created_at: string;
}

export interface LeadFunnel {
  id: string;
  contact_id: string;
  owner_user_id: string;
  funnel_stage: string;
  agent_id: string;
  score: number;
  notes: string;
  shared_with_king: boolean;
  created_at: string;
  updated_at: string;
}

export interface UserEmailAssignment {
  id: string;
  user_id: string;
  email_address: string;
  smtp_username: string;
  created_at: string;
  active: boolean;
}

export interface UserProxy {
  id: string;
  user_id: string;
  proxy_host: string;
  proxy_port: number;
  proxy_type: string;
  username: string;
  password: string;
  active: boolean;
  created_at: string;
}

export interface FunnelStats {
  total_leads: number;
  funnel: {
    discovered: number;
    contacted: number;
    pitched: number;
    negotiating: number;
    converted: number;
    lost: number;
  };
  tasks: { total: number; completed: number };
  emails_assigned: number;
}

export interface OllamaStatus {
  online: boolean;
  url: string;
  models: Array<{ name: string; size: number }>;
}

interface BrowserAgentDb {
  tasks: AgentTask[];
  conversations: AgentConversation[];
  leads: LeadFunnel[];
  emailAssignments: UserEmailAssignment[];
  proxies: UserProxy[];
  importedContacts: ParsedContact[];
  mediaFiles: MediaFile[];
  ragFiles: Array<{ path: string; tokens: number; indexed_at: string }>;
}

const AGENT_BROWSER_STORAGE_KEY = "x3-crm-agents-browser-db";

const hasTauriRuntime = () =>
  typeof window !== "undefined" && (((window as any).__TAURI_INTERNALS__) || ((window as any).__TAURI__));

const nowIso = () => new Date().toISOString();
const createId = (prefix: string) =>
  `${prefix}_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;

const STATIC_AGENT_ROSTER: AgentDef[] = [
  {
    id: "strategist",
    name: "Vega",
    role: "Campaign strategist",
    layer: "Strategic",
    avatar: "VS",
    color: "#11a0dc",
    model: "offline-preview",
    system_prompt: "Build campaign plans and prioritize outreach.",
    capabilities: ["planning", "positioning", "funnel design"],
    status: "ready",
  },
  {
    id: "closer",
    name: "Atlas",
    role: "Sales closer",
    layer: "Execution",
    avatar: "AT",
    color: "#ff6b35",
    model: "offline-preview",
    system_prompt: "Turn opportunities into action plans and follow-ups.",
    capabilities: ["follow-up", "objection handling", "deal support"],
    status: "ready",
  },
  {
    id: "researcher",
    name: "Cipher",
    role: "Research analyst",
    layer: "Growth",
    avatar: "CF",
    color: "#a855f7",
    model: "offline-preview",
    system_prompt: "Summarize targets and surface useful account context.",
    capabilities: ["research", "summaries", "qualification"],
    status: "ready",
  },
  {
    id: "copywriter",
    name: "Nova",
    role: "Email copywriter",
    layer: "Media",
    avatar: "NV",
    color: "#22c55e",
    model: "offline-preview",
    system_prompt: "Write concise personalized outreach copy.",
    capabilities: ["email copy", "subject lines", "messaging"],
    status: "ready",
  },
  {
    id: "ops",
    name: "Relay",
    role: "CRM ops assistant",
    layer: "Execution",
    avatar: "RY",
    color: "#eab308",
    model: "offline-preview",
    system_prompt: "Keep tasks and funnel data organized.",
    capabilities: ["task queues", "handoffs", "ops hygiene"],
    status: "ready",
  },
];

const makeEmptyAgentDb = (): BrowserAgentDb => ({
  tasks: [],
  conversations: [],
  leads: [],
  emailAssignments: [],
  proxies: [],
  importedContacts: [],
  mediaFiles: [],
  ragFiles: [],
});

const loadBrowserDb = (): BrowserAgentDb => {
  if (typeof window === "undefined") return makeEmptyAgentDb();
  try {
    const raw = window.localStorage.getItem(AGENT_BROWSER_STORAGE_KEY);
    return raw ? JSON.parse(raw) as BrowserAgentDb : makeEmptyAgentDb();
  } catch {
    return makeEmptyAgentDb();
  }
};

const saveBrowserDb = (db: BrowserAgentDb) => {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(AGENT_BROWSER_STORAGE_KEY, JSON.stringify(db));
};

const mutateBrowserDb = <T,>(updater: (db: BrowserAgentDb) => T): T => {
  const db = loadBrowserDb();
  const result = updater(db);
  saveBrowserDb(db);
  return result;
};

async function runAgentCommand<T>(cmd: string, args?: any): Promise<T> {
  if (hasTauriRuntime()) {
    return fallbackInvoke<T>(cmd, args);
  }

  return runBrowserAgentCommand<T>(cmd, args);
}

async function runBrowserAgentCommand<T>(cmd: string, args: any): Promise<T> {
  switch (cmd) {
    case "agents_get_roster":
      return STATIC_AGENT_ROSTER as T;

    case "agents_check_status":
      return {
        online: false,
        url: "http://localhost:11434",
        models: [],
      } as T;

    case "agents_run_task":
      return mutateBrowserDb((db) => {
        const task: AgentTask = {
          id: createId("task"),
          agent_id: args.agent_id,
          owner_user_id: args.owner_user_id,
          assigned_to_user_id: args.owner_user_id,
          task_type: "browser-preview",
          prompt: args.prompt,
          result: `Offline preview generated by ${args.agent_id}: ${args.prompt}`,
          status: "completed",
          leads_generated: 0,
          created_at: nowIso(),
          completed_at: nowIso(),
        };
        db.tasks.unshift(task);
        return task as T;
      });

    case "agents_get_tasks":
      return loadBrowserDb().tasks.filter((task) =>
        args.is_king ? true : task.owner_user_id === args.user_id,
      ) as T;

    case "agents_chat":
      return mutateBrowserDb((db) => {
        db.conversations.push({
          id: createId("chat_user"),
          agent_id: args.agent_id,
          user_id: args.user_id,
          role: "user",
          content: args.message,
          created_at: nowIso(),
        });
        const reply: AgentConversation = {
          id: createId("chat_assistant"),
          agent_id: args.agent_id,
          user_id: args.user_id,
          role: "assistant",
          content: `Offline preview for ${args.agent_id}: ${args.message}`,
          created_at: nowIso(),
        };
        db.conversations.push(reply);
        return reply as T;
      });

    case "agents_get_history":
      return loadBrowserDb().conversations.filter((conversation) =>
        conversation.user_id === args.user_id && conversation.agent_id === args.agent_id,
      ) as T;

    case "agents_create_lead":
      return mutateBrowserDb((db) => {
        const lead: LeadFunnel = {
          id: createId("lead"),
          contact_id: args.input.contact_id,
          owner_user_id: args.owner_user_id,
          funnel_stage: args.input.funnel_stage ?? "discovered",
          agent_id: args.input.agent_id ?? "strategist",
          score: Number(args.input.score ?? 50),
          notes: args.input.notes ?? "",
          shared_with_king: false,
          created_at: nowIso(),
          updated_at: nowIso(),
        };
        db.leads.unshift(lead);
        return lead as T;
      });

    case "agents_update_lead":
      mutateBrowserDb((db) => {
        db.leads = db.leads.map((lead) => lead.id === args.lead_id ? {
          ...lead,
          funnel_stage: args.funnel_stage ?? lead.funnel_stage,
          score: args.score ?? lead.score,
          notes: args.notes ?? lead.notes,
          updated_at: nowIso(),
        } : lead);
      });
      return undefined as T;

    case "agents_get_leads":
      return loadBrowserDb().leads.filter((lead) =>
        args.is_king ? true : lead.owner_user_id === args.user_id,
      ) as T;

    case "agents_assign_email":
      return mutateBrowserDb((db) => {
        const emailAddress = `${String(args.username).toLowerCase()}@x3star.net`;
        const assignment: UserEmailAssignment = {
          id: createId("email"),
          user_id: args.user_id,
          email_address: emailAddress,
          smtp_username: emailAddress,
          created_at: nowIso(),
          active: true,
        };
        db.emailAssignments = db.emailAssignments.filter((item) => item.user_id !== args.user_id);
        db.emailAssignments.push(assignment);
        return assignment as T;
      });

    case "agents_get_user_email":
      return (loadBrowserDb().emailAssignments.find((item) => item.user_id === args.user_id) ?? null) as T;

    case "agents_assign_proxy":
      return mutateBrowserDb((db) => {
        const proxy: UserProxy = {
          id: createId("proxy"),
          user_id: args.user_id,
          proxy_host: args.input.proxy_host,
          proxy_port: Number(args.input.proxy_port),
          proxy_type: args.input.proxy_type ?? "socks5",
          username: args.input.username ?? "",
          password: args.input.password ?? "",
          active: true,
          created_at: nowIso(),
        };
        db.proxies = db.proxies.filter((item) => item.user_id !== args.user_id);
        db.proxies.push(proxy);
        return proxy as T;
      });

    case "agents_get_proxy":
      return (loadBrowserDb().proxies.find((item) => item.user_id === args.user_id) ?? null) as T;

    case "agents_get_all_proxies":
      // Browser preview mode: only return proxies if user is king
      if (!args.is_king) {
        return [] as T;
      }
      return loadBrowserDb().proxies as T;

    case "agents_get_funnel_stats": {
      // Browser preview mode: only return stats if user is king
      if (!args.is_king) {
        return {
          total_leads: 0,
          funnel: { discovered: 0, contacted: 0, pitched: 0, negotiating: 0, converted: 0, lost: 0 },
          tasks: { total: 0, completed: 0 },
          emails_assigned: 0,
        } as T;
      }
      const db = loadBrowserDb();
      const funnel = db.leads.reduce<FunnelStats["funnel"]>((acc, lead) => {
        const stage = lead.funnel_stage as keyof FunnelStats["funnel"];
        if (stage in acc) acc[stage] += 1;
        return acc;
      }, { discovered: 0, contacted: 0, pitched: 0, negotiating: 0, converted: 0, lost: 0 });
      return {
        total_leads: db.leads.length,
        funnel,
        tasks: {
          total: db.tasks.length,
          completed: db.tasks.filter((task) => task.status === "completed").length,
        },
        emails_assigned: db.emailAssignments.length,
      } as T;
    }

    case "agents_web_search":
      return {
        query: args.query,
        results: [],
        analysis: "Browser preview mode: live web search is only available in the Tauri-backed agent runtime.",
        count: 0,
      } as T;

    case "agents_fetch_website":
      return {
        url: args.url,
        page_text_length: 0,
        analysis: "Browser preview mode: website fetching is disabled without the desktop agent backend.",
      } as T;

    case "agents_rag_index":
      return mutateBrowserDb((db) => {
        const file = {
          path: args.folder_path,
          tokens: 1200,
          indexed_at: nowIso(),
        };
        db.ragFiles = [file];
        return {
          folder: args.folder_path,
          files_found: 1,
          files_indexed: 1,
          total_tokens: 1200,
        } as T;
      });

    case "agents_rag_query":
      return {
        query: args.query,
        answer: `Offline preview answer for ${args.query}`,
        sources: loadBrowserDb().ragFiles.map((file) => file.path),
        docs_searched: loadBrowserDb().ragFiles.length,
      } as T;

    case "agents_rag_stats": {
      const files = loadBrowserDb().ragFiles;
      return {
        total_docs: files.length,
        total_tokens: files.reduce((sum, file) => sum + file.tokens, 0),
        files,
      } as T;
    }

    case "agents_import_contacts":
      return mutateBrowserDb((db) => {
        const contacts = String(args.raw_text)
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean)
          .map((line, index) => {
            const [namePart, emailPart = "", companyPart = ""] = line.split(",").map((part) => part.trim());
            const [first_name = "", ...rest] = namePart.split(" ");
            return {
              first_name,
              last_name: rest.join(" "),
              email: emailPart,
              phone: "",
              company: companyPart,
              job_title: "",
              country: "",
              network: "",
              ranking: Math.max(1, 100 - index * 5),
              website: "",
              notes: "",
              source: "browser-preview",
            } satisfies ParsedContact;
          });
        db.importedContacts = contacts;
        return {
          raw_length: String(args.raw_text).length,
          contacts_parsed: contacts.length,
          contacts_imported: contacts.length,
          contacts,
        } as T;
      });

    case "agents_get_contacts_sorted": {
      const contacts = [...loadBrowserDb().importedContacts];
      if (args.filter_network) {
        contacts.splice(0, contacts.length, ...contacts.filter((contact) => contact.network === args.filter_network));
      }
      if (args.filter_country) {
        contacts.splice(0, contacts.length, ...contacts.filter((contact) => contact.country === args.filter_country));
      }
      contacts.sort((a, b) => {
        if (args.sort_by === "company") return a.company.localeCompare(b.company);
        if (args.sort_by === "first_name") return a.first_name.localeCompare(b.first_name);
        return b.ranking - a.ranking;
      });
      return contacts as T;
    }

    case "agents_get_contact_filters": {
      const contacts = loadBrowserDb().importedContacts;
      return {
        networks: [...new Set(contacts.map((contact) => contact.network).filter(Boolean))],
        countries: [...new Set(contacts.map((contact) => contact.country).filter(Boolean))],
      } as T;
    }

    case "agents_toggle_proxy":
      return mutateBrowserDb((db) => {
        db.proxies = db.proxies.map((proxy) => proxy.user_id === args.user_id ? { ...proxy, active: args.active } : proxy);
        return { user_id: args.user_id, proxy_active: args.active } as T;
      });

    case "agents_scan_media":
      return {
        folder: args.folder_path,
        files_found: loadBrowserDb().mediaFiles.length,
        files: loadBrowserDb().mediaFiles,
      } as T;

    case "agents_get_media":
      return loadBrowserDb().mediaFiles as T;

    case "agents_personalized_message":
      return {
        contact_id: args.contact_id,
        contact_name: "Contact",
        message_type: args.message_type,
        message: `Offline preview message for ${args.message_type}.`,
        used_website: false,
      } as T;

    default:
      throw new Error(`Unsupported agent command in browser mode: ${cmd}`);
  }
}

/* ─── Agent Roster ───────────────────────────────── */
export const getAgentRoster = () =>
  runAgentCommand<AgentDef[]>("agents_get_roster");

export const checkAgentStatus = () =>
  runAgentCommand<OllamaStatus>("agents_check_status");

/* ─── Agent Tasks ────────────────────────────────── */
export const runAgentTask = (ownerUserId: string, agentId: string, prompt: string) =>
  runAgentCommand<AgentTask>("agents_run_task", { owner_user_id: ownerUserId, agent_id: agentId, prompt });

export const getAgentTasks = (userId: string, isKing: boolean) =>
  runAgentCommand<AgentTask[]>("agents_get_tasks", { user_id: userId, is_king: isKing });

/* ─── Agent Chat ─────────────────────────────────── */
export const chatWithAgent = (userId: string, agentId: string, message: string) =>
  runAgentCommand<AgentConversation>("agents_chat", { user_id: userId, agent_id: agentId, message });

export const getAgentHistory = (userId: string, agentId: string) =>
  runAgentCommand<AgentConversation[]>("agents_get_history", { user_id: userId, agent_id: agentId });

/* ─── Lead Funnel ────────────────────────────────── */
export const createLead = (ownerUserId: string, input: {
  contact_id: string;
  funnel_stage?: string;
  agent_id?: string;
  score?: number;
  notes?: string;
}) => runAgentCommand<LeadFunnel>("agents_create_lead", { owner_user_id: ownerUserId, input });

export const updateLead = (leadId: string, funnelStage?: string, score?: number, notes?: string) =>
  runAgentCommand<void>("agents_update_lead", { lead_id: leadId, funnel_stage: funnelStage ?? null, score: score ?? null, notes: notes ?? null });

export const getLeads = (userId: string, isKing: boolean) =>
  runAgentCommand<LeadFunnel[]>("agents_get_leads", { user_id: userId, is_king: isKing });

/* ─── Email Assignment ───────────────────────────── */
export const assignEmail = (userId: string, username: string) =>
  runAgentCommand<UserEmailAssignment>("agents_assign_email", { user_id: userId, username });

export const getUserEmail = (userId: string) =>
  runAgentCommand<UserEmailAssignment | null>("agents_get_user_email", { user_id: userId });

/* ─── Proxy Management ───────────────────────────── */
export const assignProxy = (userId: string, input: {
  proxy_host: string;
  proxy_port: number;
  proxy_type?: string;
  username?: string;
  password?: string;
}) => runAgentCommand<UserProxy>("agents_assign_proxy", { user_id: userId, input });

export const getProxy = (userId: string) =>
  runAgentCommand<UserProxy | null>("agents_get_proxy", { user_id: userId });

export const getAllProxies = () =>
  runAgentCommand<UserProxy[]>("agents_get_all_proxies");

/* ─── Funnel Stats (King) ────────────────────────── */
export const getFunnelStats = () =>
  runAgentCommand<FunnelStats>("agents_get_funnel_stats");

/* ─── Web Search ─────────────────────────────────── */
export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
}

export interface WebSearchResponse {
  query: string;
  results: SearchResult[];
  analysis: string | null;
  count: number;
}

export const webSearch = (query: string, agentId?: string) =>
  runAgentCommand<WebSearchResponse>("agents_web_search", { query, agent_id: agentId ?? null });

export const fetchWebsite = (url: string, agentId: string) =>
  runAgentCommand<{ url: string; page_text_length: number; analysis: string }>("agents_fetch_website", { url, agent_id: agentId });

/* ─── RAG System ─────────────────────────────────── */
export interface RagStats {
  total_docs: number;
  total_tokens: number;
  files: Array<{ path: string; tokens: number; indexed_at: string }>;
}

export const ragIndex = (folderPath: string) =>
  runAgentCommand<{ folder: string; files_found: number; files_indexed: number; total_tokens: number }>("agents_rag_index", { folder_path: folderPath });

export const ragQuery = (query: string, agentId: string) =>
  runAgentCommand<{ query: string; answer: string; sources: string[]; docs_searched: number }>("agents_rag_query", { query, agent_id: agentId });

export const ragStats = () =>
  runAgentCommand<RagStats>("agents_rag_stats");

/* ─── Contact Import & Sorting ───────────────────── */
export interface ParsedContact {
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  company: string;
  job_title: string;
  country: string;
  network: string;
  ranking: number;
  website: string;
  notes: string;
  source: string;
}

export const importContacts = (ownerUserId: string, rawText: string) =>
  runAgentCommand<{ raw_length: number; contacts_parsed: number; contacts_imported: number; contacts: ParsedContact[] }>(
    "agents_import_contacts", { owner_user_id: ownerUserId, raw_text: rawText }
  );

export const getContactsSorted = (ownerUserId: string, sortBy: string, filterNetwork?: string, filterCountry?: string) =>
  runAgentCommand<any[]>("agents_get_contacts_sorted", {
    owner_user_id: ownerUserId, sort_by: sortBy,
    filter_network: filterNetwork ?? null, filter_country: filterCountry ?? null,
  });

export const getContactFilters = (ownerUserId: string) =>
  runAgentCommand<{ networks: string[]; countries: string[] }>("agents_get_contact_filters", { owner_user_id: ownerUserId });

/* ─── Proxy/VPN Toggle ───────────────────────────── */
export const toggleProxy = (userId: string, active: boolean) =>
  runAgentCommand<{ user_id: string; proxy_active: boolean }>("agents_toggle_proxy", { user_id: userId, active });

/* ─── Media Folder ───────────────────────────────── */
export interface MediaFile {
  id: string;
  file_name: string;
  file_path: string;
  file_type: string;
  file_size: number;
}

export const scanMedia = (folderPath: string) =>
  runAgentCommand<{ folder: string; files_found: number; files: MediaFile[] }>("agents_scan_media", { folder_path: folderPath });

export const getMedia = () =>
  runAgentCommand<any[]>("agents_get_media");

/* ─── Personalized Messages ──────────────────────── */
export const generatePersonalizedMessage = (contactId: string, agentId: string, messageType: string) =>
  runAgentCommand<{ contact_id: string; contact_name: string; message_type: string; message: string; used_website: boolean }>(
    "agents_personalized_message", { contact_id: contactId, agent_id: agentId, message_type: messageType }
  );

/* ─── 90-Day Rollout ─────────────────────────────── */
export interface RolloutPhase {
  id: string;
  phase_num: number;
  title: string;
  description: string;
  start_day: number;
  end_day: number;
  status: string;
  milestones: string;
  progress: number;
  created_at: string;
  updated_at: string;
}

export const seedRollout = () =>
  invoke<{ phases_seeded: number }>("agents_seed_rollout");

export const getRollout = () =>
  invoke<RolloutPhase[]>("agents_get_rollout");

export const updateRollout = (phaseId: string, status?: string, progress?: number, milestones?: string) =>
  invoke<{ phase_id: string; updated: boolean }>("agents_update_rollout", {
    phase_id: phaseId,
    status: status ?? null,
    progress: progress ?? null,
    milestones: milestones ?? null,
  });

/* ─── Page Builder ───────────────────────────────── */
export interface GeneratedPage {
  id: string;
  slug: string;
  title: string;
  page_type: string;
  meta_title: string;
  meta_desc: string;
  seo_keywords: string;
  status: string;
  agent_id: string;
  created_at: string;
  updated_at: string;
}

export interface PageContent extends GeneratedPage {
  html_content: string;
}

export const generatePage = (slug: string, title: string, pageType: string, prompt: string, agentId?: string) =>
  invoke<{ id: string; slug: string; title: string; page_type: string; meta_title: string; meta_desc: string; seo_keywords: string; html_length: number; status: string; agent_id: string }>(
    "agents_generate_page", { slug, title, page_type: pageType, prompt, agent_id: agentId ?? null }
  );

export const getPages = () =>
  invoke<GeneratedPage[]>("agents_get_pages");

export const getPageContent = (pageId: string) =>
  invoke<PageContent>("agents_get_page_content", { page_id: pageId });

export const updatePageStatus = (pageId: string, status: string) =>
  invoke<{ page_id: string; status: string }>("agents_update_page_status", { page_id: pageId, status });

export const deletePage = (pageId: string) =>
  invoke<{ page_id: string; deleted: boolean }>("agents_delete_page", { page_id: pageId });

/* ─── Agent Hierarchy ────────────────────────────── */
export interface AgentHierarchy {
  layers: string[];
  agents_by_layer: Record<string, Array<{
    id: string;
    name: string;
    role: string;
    avatar: string;
    color: string;
    capabilities: string[];
    status: string;
  }>>;
  total_agents: number;
}

export const getAgentHierarchy = () =>
  invoke<AgentHierarchy>("agents_get_hierarchy");
