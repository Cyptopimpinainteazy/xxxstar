// Frontend hooks for TIER 6 CRM API integration
// Use these in React components to call backend Tauri commands

import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';

// ============================================
// Type Definitions
// ============================================

export interface Contact {
  id: string;
  user_id: string;
  first_name: string;
  last_name: string;
  email: string;
  phone?: string;
  company?: string;
  job_title?: string;
  source?: string;
  status?: string;
  custom_fields?: Record<string, string>;
  tags?: string[];
  created_at: string;
  updated_at: string;
}

export interface EmailTemplate {
  id: string;
  user_id: string;
  name: string;
  subject: string;
  body: string;
  created_at: string;
}

export interface Campaign {
  id: string;
  name: string;
  campaign_type: string;
  status: string;
  target_contacts: number;
  sent_count: number;
  opened_count: number;
  created_at: string;
}

export interface LeadScore {
  contact_id: string;
  score: number;
  grade: string;
}

export interface DuplicateContact {
  id1: string;
  id2: string;
  similarity_score: number;
}

export interface CsvImportResult {
  total_rows: number;
  imported: number;
  skipped: number;
  merged: number;
  errors: string[];
}

// ============================================
// CRM API Hooks
// ============================================

export function useCrmContacts(userId: string) {
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getContacts = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke('crm_get_contacts', { userId });
      setContacts(result as Contact[]);
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  const createContact = async (input: Partial<Contact>) => {
    setLoading(true);
    setError(null);
    try {
      const created = await invoke('crm_create_contact', { userId, input });
      setContacts([...contacts, created as Contact]);
      return created;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  const updateContact = async (contactId: string, input: Partial<Contact>) => {
    setLoading(true);
    setError(null);
    try {
      const updated = await invoke('crm_update_contact', { userId, contactId, input });
      setContacts(contacts.map(c => c.id === contactId ? (updated as Contact) : c));
      return updated;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  const deleteContact = async (contactId: string) => {
    setLoading(true);
    setError(null);
    try {
      await invoke('crm_delete_contact', { userId, contactId });
      setContacts(contacts.filter(c => c.id !== contactId));
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  return { contacts, loading, error, getContacts, createContact, updateContact, deleteContact };
}

export function useCrmCsv(userId: string) {
  const [importing, setImporting] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const importCsv = async (csvContent: string, columnMapping: Record<string, string>) => {
    setImporting(true);
    setError(null);
    try {
      const result = await invoke('crm_import_csv', {
        userId,
        input: { csv_content: csvContent, column_mapping: columnMapping, skip_duplicates: true }
      });
      return result as CsvImportResult;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setImporting(false);
    }
  };

  const exportCsv = async () => {
    setExporting(true);
    setError(null);
    try {
      const result = await invoke('crm_export_csv', { userId });
      return result as string;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setExporting(false);
    }
  };

  return { importing, exporting, error, importCsv, exportCsv };
}

export function useCrmDeduplication(userId: string) {
  const [finding, setFinding] = useState(false);
  const [merging, setMerging] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const findDuplicates = async () => {
    setFinding(true);
    setError(null);
    try {
      const result = await invoke('crm_find_duplicates', { userId });
      return result as DuplicateContact[];
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setFinding(false);
    }
  };

  const mergeContacts = async (id1: string, id2: string, keepId: string) => {
    setMerging(true);
    setError(null);
    try {
      await invoke('crm_merge_contacts', {
        userId,
        input: { id1, id2, keep_fields_from_id1: keepId === id1 }
      });
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setMerging(false);
    }
  };

  return { finding, merging, error, findDuplicates, mergeContacts };
}

export function useCrmEmail(userId: string) {
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sendEmail = async (toEmail: string, subject: string, body: string, contactId?: string) => {
    setSending(true);
    setError(null);
    try {
      const result = await invoke('crm_send_email', {
        userId,
        input: { to_email: toEmail, subject, body, contact_id: contactId }
      });
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setSending(false);
    }
  };

  return { sending, error, sendEmail };
}

export function useCrmCampaigns(userId: string) {
  const [campaigns, setCampaigns] = useState<Campaign[]>([]);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getCampaigns = async () => {
    try {
      const result = await invoke('crm_get_campaigns', { userId });
      setCampaigns(result as Campaign[]);
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    }
  };

  const createCampaign = async (name: string, campaignType: string, targetContacts: number) => {
    setCreating(true);
    setError(null);
    try {
      const result = await invoke('crm_create_campaign', {
        userId,
        input: { name, campaign_type: campaignType, target_contacts: targetContacts }
      });
      setCampaigns([...campaigns, result as Campaign]);
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setCreating(false);
    }
  };

  return { campaigns, creating, error, getCampaigns, createCampaign };
}

export function useCrmLeadScoring(userId: string) {
  const [scores, setScores] = useState<LeadScore[]>([]);
  const [calculating, setCalculating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const calculateScores = async () => {
    setCalculating(true);
    setError(null);
    try {
      const result = await invoke('crm_calculate_lead_scores', { userId });
      setScores(result as LeadScore[]);
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setCalculating(false);
    }
  };

  return { scores, calculating, error, calculateScores };
}

export function useCrmAnalytics(userId: string) {
  const [analytics, setAnalytics] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getPipelineAnalytics = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke('crm_get_pipeline_analytics', { userId });
      setAnalytics(result);
      return result;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  return { analytics, loading, error, getPipelineAnalytics };
}
